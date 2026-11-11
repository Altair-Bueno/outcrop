//! Strata implemented by hand, the way `strata!` expands.

mod common;

use common::poll;
use outcrop::Feed;
use outcrop::Formation;
use outcrop::Report;
use outcrop::Sink;
use outcrop::Strata;
use outcrop::Visit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Temperature(i16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Setpoint(i16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Heating(bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Thermostat {
    s0: Temperature,
    s1: Setpoint,
    s2: Heating,
}

impl Thermostat {
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "mirrors `strata!`, which passes signals by reference"
    )]
    fn derive_2(t: &Temperature, s: &Setpoint) -> Heating {
        Heating(t.0 < s.0)
    }

    fn route_3(frame: &[i16], sink: &mut Sink<'_, Below3<'_>>) {
        sink.feed(Temperature(frame[0]));
        sink.feed(Setpoint(frame[1]));
    }
}

impl Default for Thermostat {
    fn default() -> Self {
        let s0: Temperature = Temperature(20);
        let s1: Setpoint = Setpoint(18);
        let s2: Heating = Self::derive_2(&s0, &s1);
        Self { s0, s1, s2 }
    }
}

impl Strata for Thermostat {
    type Flags = [u8; 1];

    const CHANGED: [u8; 1] = [0b111];

    fn evaluate(&mut self, changed: &mut [u8; 1]) {
        if changed[0] & 0b11 != 0 {
            let value = Self::derive_2(&self.s0, &self.s1);
            if value != self.s2 {
                self.s2 = value;
                changed[0] |= 0b100;
            }
        }
    }
}

impl Feed<Temperature, ()> for Thermostat {
    fn feed(&mut self, changed: &mut [u8; 1], value: Temperature) {
        if self.s0 != value {
            self.s0 = value;
            changed[0] |= 0b1;
        }
    }
}

impl Feed<Setpoint, ()> for Thermostat {
    fn feed(&mut self, changed: &mut [u8; 1], value: Setpoint) {
        if self.s1 != value {
            self.s1 = value;
            changed[0] |= 0b10;
        }
    }
}

struct Below3<'s>(&'s mut Thermostat);

impl Strata for Below3<'_> {
    type Flags = [u8; 1];

    const CHANGED: [u8; 1] = [0b111];

    fn evaluate(&mut self, _: &mut [u8; 1]) {}
}

impl Feed<Temperature, ()> for Below3<'_> {
    fn feed(&mut self, changed: &mut [u8; 1], value: Temperature) {
        <Thermostat as Feed<Temperature, ()>>::feed(self.0, changed, value);
    }
}

impl Feed<Setpoint, ()> for Below3<'_> {
    fn feed(&mut self, changed: &mut [u8; 1], value: Setpoint) {
        <Thermostat as Feed<Setpoint, ()>>::feed(self.0, changed, value);
    }
}

impl<'a> Feed<&'a [i16], ()> for Thermostat {
    fn feed(&mut self, changed: &mut [u8; 1], value: &'a [i16]) {
        Self::route_3(value, &mut Sink::new(&mut Below3(self), changed));
    }
}

impl<V> Report<V> for Thermostat
where
    V: Visit<Temperature> + Visit<Setpoint> + Visit<Heating>,
{
    fn report(&self, changed: &mut [u8; 1], visitor: &mut V) {
        if changed[0] & 0b1 != 0 {
            changed[0] &= !0b1;
            <V as Visit<Temperature>>::visit(visitor, &self.s0);
        }
        if changed[0] & 0b10 != 0 {
            changed[0] &= !0b10;
            <V as Visit<Setpoint>>::visit(visitor, &self.s1);
        }
        if changed[0] & 0b100 != 0 {
            changed[0] &= !0b100;
            <V as Visit<Heating>>::visit(visitor, &self.s2);
        }
    }
}

#[test]
fn first_poll_reports_every_initial_value() {
    let mut formation = Formation::from(Thermostat::default());

    assert_eq!(
        poll(&mut formation),
        ["Temperature(20)", "Setpoint(18)", "Heating(false)"]
    );
    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[test]
fn feeding_an_input_recomputes_its_dependents() {
    let mut formation = Formation::from(Thermostat::default());
    poll(&mut formation);

    formation.feed(Temperature(10));

    assert_eq!(poll(&mut formation), ["Temperature(10)", "Heating(true)"]);
}

#[test]
fn routers_feed_the_inputs_above_them() {
    let mut formation = Formation::from(Thermostat::default());
    poll(&mut formation);

    formation.feed(&[20, 25][..]);

    assert_eq!(poll(&mut formation), ["Setpoint(25)", "Heating(true)"]);
}

#[test]
fn formations_compare_values_and_pending_changes() {
    let mut formation = Formation::from(Thermostat::default());
    let copy = formation;
    assert_eq!(formation, copy);

    poll(&mut formation);

    assert_ne!(formation, copy);
}
