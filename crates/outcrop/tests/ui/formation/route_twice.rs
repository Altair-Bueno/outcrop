use outcrop::Formation;

fn main() {
    let _ = Formation::new().route(|_: &u8, _| {}).route(|_: &u8, _| {});
}
