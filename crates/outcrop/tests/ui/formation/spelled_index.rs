use outcrop::Formation;

fn main() {
    let _ = Formation::new().input(0_u8).input::<u8, ((), ())>(1);
}
