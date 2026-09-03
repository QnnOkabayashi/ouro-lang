pub use index_vec::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Counter<I> {
    pub next: I,
}

impl<I: Idx> Counter<I> {
    /// Returns a new [`Counter`] starting at 0.
    pub fn new() -> Self {
        Counter {
            next: I::from_usize(0),
        }
    }

    /// Returns the next `I`, incrementing `self` in the process.
    pub fn next(&mut self) -> I {
        let next = self.next;
        self.next = I::from_usize(next.index() + 1);
        next
    }
}

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
