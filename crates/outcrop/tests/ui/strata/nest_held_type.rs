use outcrop::Formation;

#[derive(PartialEq, Eq)]
struct Kelvin(u16);

outcrop::strata! {
    struct Probe {
        kelvin: Kelvin = Kelvin(293),
    }
}

fn main() {
    let _ = Formation::new()
        .input(Kelvin(0))
        .nest(Probe::default());
}
