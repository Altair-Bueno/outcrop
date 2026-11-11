outcrop::strata! {
    struct S {
        #[strata(bank)]
        a: [u16; 2] = [0; 2],
        #[strata(bank)]
        b: [u16; 3] = [0; 3],
    }
}

fn main() {}
