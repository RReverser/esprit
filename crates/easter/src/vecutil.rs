pub trait VecUtil<I, O, E> {
    fn map<F: Fn(I) -> Result<O, E>>(self, F) -> Result<Vec<O>, E>;
}

impl<I, O, E> VecUtil<I, O, E> for Vec<I> {
    fn map<F: Fn(I) -> Result<O, E>>(self, f: F) -> Result<Vec<O>, E> {
        self.into_iter().map(f).collect()
    }
}
