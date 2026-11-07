/// Receives the signals reported by
/// [`Formation::poll`](crate::Formation::poll).
///
/// Implement it once per signal type, or once for every type through a
/// blanket impl such as `impl<T: Debug> Visit<T> for Printer`. The unit
/// type discards every signal, a mutable reference forwards to the visitor
/// it points to, and a tuple of up to eight visitors reports to each of them
/// in order.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot visit the signal `{T}`",
    label = "`poll` reports `{T}` to this visitor",
    note = "implement `Visit<{T}>` for `{Self}`, or cover every signal at once with a blanket impl such as `impl<T: Debug> Visit<T>`"
)]
pub trait Visit<T> {
    /// Called with the current value of a signal that changed.
    fn visit(&mut self, value: &T);
}

impl<T> Visit<T> for () {
    fn visit(&mut self, _: &T) {}
}

impl<T, V: Visit<T> + ?Sized> Visit<T> for &mut V {
    fn visit(&mut self, value: &T) {
        (**self).visit(value);
    }
}

macro_rules! impl_visit {
    ($($visitor:ident $index:tt),+) => {
        impl<T, $($visitor: Visit<T>),+> Visit<T> for ($($visitor,)+) {
            fn visit(&mut self, value: &T) {
                $(self.$index.visit(value);)+
            }
        }
    };
}

impl_visit!(V1 0);
impl_visit!(V1 0, V2 1);
impl_visit!(V1 0, V2 1, V3 2);
impl_visit!(V1 0, V2 1, V3 2, V4 3);
impl_visit!(V1 0, V2 1, V3 2, V4 3, V5 4);
impl_visit!(V1 0, V2 1, V3 2, V4 3, V5 4, V6 5);
impl_visit!(V1 0, V2 1, V3 2, V4 3, V5 4, V6 5, V7 6);
impl_visit!(V1 0, V2 1, V3 2, V4 3, V5 4, V6 5, V7 6, V8 7);
