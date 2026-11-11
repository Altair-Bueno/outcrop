use outcrop::Formation;

#[derive(PartialEq, Eq)]
struct A(u8);
#[derive(PartialEq, Eq)]
struct B(u8);

fn main() {
    let mut formation = Formation::new().input(A(0)).view(|a: &A| B(a.0));
    formation.feed(B(1));
}
