//! An Airzone Aidoo driving a direct expansion unit over Modbus.
//!
//! Register map: Aidoo Modbus manual, "Registers for Direct Expansion units".
#![expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "derived signals receive their dependencies by reference"
)]
#![expect(
    clippy::redundant_pub_crate,
    reason = "`unreachable_pub` asks for the opposite inside private modules"
)]

mod aidoo;
mod capabilities;
mod control;
mod modbus;
mod register;
mod telemetry;
mod units;

use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Visit;
use outcrop::Write;

use crate::aidoo::Aidoo;
use crate::capabilities::SetpointRange;

struct Print;

impl<T: Debug> Visit<T> for Print {
    #[expect(clippy::print_stdout, reason = "the example shows what a poll reports")]
    fn visit(&mut self, value: &T) {
        println!("  {value:?}");
    }
}

#[expect(clippy::print_stdout, reason = "the example shows what a poll reports")]
fn main() {
    let mut aidoo = Formation::from(Aidoo::default());

    // Startup: read every block once before the first poll, so the zeros
    // the registers start with are never reported.
    let status = [1, 230, 215, 2, 50, 0x0098, 0x0008, 0, 0];
    let capabilities = [
        0b1_1111, 0b111_1101, 0x1131, 300, 180, 300, 160, 300, 160, 300, 160, 300, 160,
    ];
    let sensors = [0xFFE2_u16, 205, 120, 110, 450, 750, 120, 150, 98, 245, 70];
    aidoo.feed(Write::new(0, &status));
    aidoo.feed(Write::new(14, &capabilities));
    aidoo.feed(Write::new(35, &sensors));
    aidoo.feed(Write::new(53, &[220, 2, 0, 1, 8, 2, 0, 0]));
    aidoo.feed(Write::new(73, &[0x1215, 0x0001, 0x1CAC]));

    println!("first poll:");
    aidoo.poll(&mut Print);

    // Someone switches the unit to heating at the wall.
    aidoo.feed(Write::new(0, &[1, 210, 215, 3, 50]));

    println!("after switching to heat:");
    aidoo.poll(&mut Print);

    println!(
        "setpoint range: {:?}",
        aidoo.get::<SetpointRange, _>().into_inner()
    );
}
