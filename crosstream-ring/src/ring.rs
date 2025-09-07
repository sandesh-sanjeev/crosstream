/// Strategy used to trim records during appends into a [`Segment`].
#[derive(Debug, Clone, Copy)]
pub enum Trimmer<T> {
    Nothing,
    Trim(usize),
    TrimFn(fn(&[T]) -> usize),
    TrimJustEnough,
}
