use core::borrow::Borrow;

use crate::plumbing::Bits;
use crate::plumbing::ComputeImpl;
use crate::plumbing::Select;

macro_rules! impl_compute {
    ($(($dep:ident, $index:ident, $value:ident)),+) => {
        impl<S, F, O, $($dep, $index),+> ComputeImpl<S, (fn($(&$dep),+) -> O, ($($index,)+))> for F
        where
            F: Fn($(&$dep),+) -> O,
            $(S: Select<$dep, $index>,)+
        {
            type Output = O;
            const CACHED: bool = false $(|| <S as Select<$dep, $index>>::CACHED)+;

            fn dirty<Fl: Bits>(changed: &Fl) -> bool {
                false $(|| changed.get(<S as Select<$dep, $index>>::INDEX))+
            }

            fn compute(&self, strata: &S) -> O {
                $(let $value = <S as Select<$dep, $index>>::select(strata);)+
                self($(Borrow::<$dep>::borrow(&$value)),+)
            }
        }
    };
}

impl_compute!((D1, I1, d1));
impl_compute!((D1, I1, d1), (D2, I2, d2));
impl_compute!((D1, I1, d1), (D2, I2, d2), (D3, I3, d3));
impl_compute!((D1, I1, d1), (D2, I2, d2), (D3, I3, d3), (D4, I4, d4));
impl_compute!(
    (D1, I1, d1),
    (D2, I2, d2),
    (D3, I3, d3),
    (D4, I4, d4),
    (D5, I5, d5)
);
impl_compute!(
    (D1, I1, d1),
    (D2, I2, d2),
    (D3, I3, d3),
    (D4, I4, d4),
    (D5, I5, d5),
    (D6, I6, d6)
);
impl_compute!(
    (D1, I1, d1),
    (D2, I2, d2),
    (D3, I3, d3),
    (D4, I4, d4),
    (D5, I5, d5),
    (D6, I6, d6),
    (D7, I7, d7)
);
impl_compute!(
    (D1, I1, d1),
    (D2, I2, d2),
    (D3, I3, d3),
    (D4, I4, d4),
    (D5, I5, d5),
    (D6, I6, d6),
    (D7, I7, d7),
    (D8, I8, d8)
);
