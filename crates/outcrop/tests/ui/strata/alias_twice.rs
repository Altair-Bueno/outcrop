#[derive(PartialEq, Eq)]
struct A(u8);
type Alias = A;

outcrop::strata! {
    struct S {
        a: A = A(0),
        alias: Alias = A(1),
    }
}

fn main() {}
