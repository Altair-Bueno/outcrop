use outcrop::Formation;

#[derive(PartialEq, Eq)]
struct A(u8);
#[derive(PartialEq, Eq)]
struct B(u8);

fn main() {
    let _ = Formation::new().memo(|a: &A| B(a.0));
}
