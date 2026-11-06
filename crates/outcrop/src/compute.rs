use crate::flags::Flags;
use crate::stratum::Select;

/// A function computing a derived signal from signals in the strata `S`.
///
/// Implemented for every function and closure taking one to eight references
/// to distinct signals, such as `fn(&A, &B) -> O`. The marker `M` records the
/// argument types and their positions so that the dependencies are inferred
/// from the signature.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot derive a signal from the strata beneath it",
    label = "not a function of signals already in the formation",
    note = "every argument must be a reference `&T` to a distinct signal deposited earlier with `input` or `with`, with at most eight arguments"
)]
pub trait Compute<S, M> {
    /// The derived signal.
    type Output;

    /// Returns whether any dependency changed.
    fn dirty<B: Flags>(changed: &B) -> bool;

    /// Computes the derived signal from the current dependencies.
    fn compute(&mut self, strata: &S) -> Self::Output;
}

macro_rules! impl_compute {
    ($(($dep:ident, $index:ident)),+) => {
        impl<S, F, O, $($dep, $index),+> Compute<S, (fn($(&$dep),+) -> O, ($($index,)+))> for F
        where
            F: FnMut($(&$dep),+) -> O,
            $(S: Select<$dep, $index>,)+
        {
            type Output = O;

            fn dirty<B: Flags>(changed: &B) -> bool {
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
