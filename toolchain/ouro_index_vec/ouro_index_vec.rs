pub use index_vec::*;

#[derive(Copy, Clone, Debug)]
pub struct MaxOr<I>(I);

impl<I: Idx> MaxOr<I> {
    pub fn into_non_max(self) -> Option<I> {
        if self.0.index() == u32::MAX as usize {
            None
        } else {
            Some(self.0)
        }
    }

    pub fn new(index: I) -> Self {
        MaxOr(index)
    }

    pub fn max() -> Self {
        MaxOr(I::from_usize(u32::MAX as usize))
    }
}
