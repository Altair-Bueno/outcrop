#[derive(PartialEq, Eq)]
struct A(u8);
#[derive(PartialEq, Eq)]
struct B(u8);

outcrop::strata! {
    struct S {
        b: B = A => |a| B(a.0),
        a: A = A(0),
    }
}

fn main() {}
