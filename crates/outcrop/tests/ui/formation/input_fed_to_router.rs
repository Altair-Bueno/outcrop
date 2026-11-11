use outcrop::Formation;

fn main() {
    let _ = Formation::new()
        .route(|_: &u8, _| {})
        .input::<&'static u8, _>(&0);
}
