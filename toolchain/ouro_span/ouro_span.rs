use std::fmt::{Debug, Formatter, Result};
use std::ops::{Add, AddAssign, SubAssign};

use ouro_index_vec::Idx;

pub trait Unit:
    Idx + Add<Output = Self> + SubAssign + AddAssign + From<usize> + PartialOrd + Ord + Debug
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

impl Unit for Byte {
    fn len(s: &str) -> Self {
        Byte::from(s.len())
    }
}
impl Unit for Utf8Char {
    fn len(s: &str) -> Self {
        Utf8Char::from(s.chars().count())
    }
}

impl Unit for Utf16Char {
    fn len(s: &str) -> Self {
        Utf16Char::from(s.encode_utf16().count())
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
        self.start.fmt(f)?;
        write!(f, "..")?;
        self.end.fmt(f)?;
        Ok(())
    }
}
