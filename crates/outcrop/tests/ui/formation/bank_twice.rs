use outcrop::Formation;

fn main() {
    let _ = Formation::new()
        .bank::<1, _, _, _>([0_u16; 2])
        .bank::<1, _, _, _>([0_u16; 3]);
}
