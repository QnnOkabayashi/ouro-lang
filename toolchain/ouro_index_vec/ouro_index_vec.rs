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
