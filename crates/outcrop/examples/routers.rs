//! The same routers declared with `Formation::route` and with `strata!`.
//!
//! A router is declared for an `In` and fed a `&In`, so borrowed data such as
//! a byte slice can be fed without copying it into the formation.

use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Visit;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Register<const ADDRESS: u16>(u16);

#[derive(Debug)]
struct Write {
    address: u16,
    value: u16,
}

impl From<[u8; 4]> for Write {
    fn from([a, b, c, d]: [u8; 4]) -> Self {
        Self {
            address: u16::from_be_bytes([a, b]),
            value: u16::from_be_bytes([c, d]),
        }
    }
}

outcrop::strata! {
    struct Device {
        power: Register<0> = Register(0),
        setpoint: Register<1> = Register(0),
        Write => |write, sink| match write.address {
            0 => sink.feed(Register::<0>(write.value)),
            1 => sink.feed(Register::<1>(write.value)),
            _ => {}
        },
        [u8] => |frame, sink| {
            for chunk in frame.as_chunks::<4>().0 {
                sink.feed(&Write::from(*chunk));
            }
        },
    }
}

struct Print(&'static str);

impl<T: Debug> Visit<T> for Print {
    #[expect(clippy::print_stdout, reason = "the example shows what a poll reports")]
    fn visit(&mut self, value: &T) {
        println!("{}: {value:?}", self.0);
    }
}

fn main() {
    let mut built = Formation::new()
        .input(Register::<0>(0))
        .input(Register::<1>(0))
        .route(|write: &Write, sink| match write.address {
            0 => sink.feed(Register::<0>(write.value)),
            1 => sink.feed(Register::<1>(write.value)),
            _ => {}
        })
        .route(|frame: &[u8], sink| {
            for chunk in frame.as_chunks::<4>().0 {
                sink.feed(&Write::from(*chunk));
            }
        });
    let mut declared = Formation::from(Device::default());
    built.poll(&mut ());
    declared.poll(&mut ());

    let frame = [0, 0, 0, 1, 0, 1, 0, 230];
    built.feed(&frame[..]);
    declared.feed(&frame[..]);
    built.poll(&mut Print("built"));
    declared.poll(&mut Print("declared"));

    built.feed(&Write {
        address: 1,
        value: 215,
    });
    declared.feed(&Write {
        address: 1,
        value: 215,
    });
    built.poll(&mut Print("built"));
    declared.poll(&mut Print("declared"));
}
