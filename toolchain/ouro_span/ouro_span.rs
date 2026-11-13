use std::fmt::{Debug, Formatter, Result};
use std::ops::{Add, AddAssign, SubAssign};

use ouro_index_vec::Idx;

pub trait Unit:
    Idx + Add<Output = Self> + SubAssign + AddAssign + PartialOrd + Ord + Debug
{
    fn len(s: &str) -> Self;
}

/// A span of source code.
#[derive(Copy, Clone, PartialEq)]
pub struct Span<U> {
    pub start: U,
    pub end: U,
}

pub type ByteSpan = Span<Byte>;

ouro_index_vec::define_index_type! {
    /// Byte offset into source code.
    #[derive(Default)]
    pub struct Byte = u32;
}

ouro_index_vec::define_index_type! {
    #[derive(Default)]
    pub struct Utf8Char = u32;
}

ouro_index_vec::define_index_type! {
    #[derive(Default)]
    pub struct Utf16Char = u32;
}

ouro_index_vec::impl_arithmetic!(Byte);
ouro_index_vec::impl_arithmetic!(Utf8Char);
ouro_index_vec::impl_arithmetic!(Utf16Char);

impl Unit for Byte {
    fn len(s: &str) -> Self {
        Byte::from_usize(s.len())
    }
}
impl Unit for Utf8Char {
    fn len(s: &str) -> Self {
        Utf8Char::from_usize(s.chars().count())
    }
}

impl Unit for Utf16Char {
    fn len(s: &str) -> Self {
        Utf16Char::from_usize(s.encode_utf16().count())
    }
}

impl ByteSpan {
    /// Slices a source string with the span.
    pub fn lookup(self, source: &str) -> &str {
        &source[self.start.index()..self.end.index()]
    }
}

impl<U: Unit> Debug for Span<U> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}..{}", self.start.index(), self.end.index())
    }
}
