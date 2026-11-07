use crate::plumbing::Bits;
use crate::plumbing::ComputeImpl;
use crate::plumbing::Select;

macro_rules! impl_compute {
    ($(($dep:ident, $index:ident)),+) => {
        impl<S, F, O, $($dep, $index),+> ComputeImpl<S, (fn($(&$dep),+) -> O, ($($index,)+))> for F
        where
            F: FnMut($(&$dep),+) -> O,
            $(S: Select<$dep, $index>,)+
        {
            type Output = O;

            fn dirty<Fl: Bits>(changed: &Fl) -> bool {
                false $(|| changed.get(<S as Select<$dep, $index>>::INDEX))+
            }

            fn compute(&mut self, strata: &S) -> O {
                self($(<S as Select<$dep, $index>>::select(strata)),+)
            }
        }
    };
}

impl_compute!((D1, I1));
impl_compute!((D1, I1), (D2, I2));
impl_compute!((D1, I1), (D2, I2), (D3, I3));
impl_compute!((D1, I1), (D2, I2), (D3, I3), (D4, I4));
impl_compute!((D1, I1), (D2, I2), (D3, I3), (D4, I4), (D5, I5));
impl_compute!((D1, I1), (D2, I2), (D3, I3), (D4, I4), (D5, I5), (D6, I6));
impl_compute!(
    (D1, I1),
    (D2, I2),
    (D3, I3),
    (D4, I4),
    (D5, I5),
    (D6, I6),
    (D7, I7)
);
impl_compute!(
    (D1, I1),
    (D2, I2),
    (D3, I3),
    (D4, I4),
    (D5, I5),
    (D6, I6),
    (D7, I7),
    (D8, I8)
);
