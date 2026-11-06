/// Receives the signals reported by
/// [`Formation::poll`](crate::Formation::poll).
///
/// Implement it once per signal type, or once for every type through a
/// blanket impl such as `impl<T: Debug> Visit<T> for Printer`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot visit the signal `{T}`",
    label = "`poll` reports `{T}` to this visitor",
    note = "implement `Visit<{T}>` for `{Self}`, or cover every signal at once with a blanket impl such as `impl<T: Debug> Visit<T>`"
)]
pub trait Visit<T> {
    /// Called with the current value of a signal that changed.
    fn visit(&mut self, value: &T);
}
