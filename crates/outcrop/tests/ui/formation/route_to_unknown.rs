use outcrop::Formation;

#[derive(PartialEq, Eq)]
struct A(u8);
#[derive(PartialEq, Eq)]
struct B(u8);

fn main() {
    let _ = Formation::new()
        .input(A(0))
        .route(|a: &u8, out| out.feed(B(*a)));
}
