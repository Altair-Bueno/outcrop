#[derive(PartialEq, Eq)]
struct Sum(u8);

outcrop::strata! {
    struct S {
        a: u8 = 0, b: u16 = 0, c: u32 = 0, d: u64 = 0, e: u128 = 0,
        f: i8 = 0, g: i16 = 0, h: i32 = 0, i: i64 = 0,
        sum: Sum = (u8, u16, u32, u64, u128, i8, i16, i32, i64)
            => |_, _, _, _, _, _, _, _, _| Sum(0),
    }
}

fn main() {}
