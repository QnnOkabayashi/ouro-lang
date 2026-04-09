use std::fmt::{Debug, Formatter, Result};

/// A span of source code.
#[derive(Copy, Clone, PartialEq)]
pub struct Span {
    pub start: Offset,
    pub end: Offset,
}

#[derive(Copy, Clone, PartialEq)]
pub struct Offset(pub u32);

impl Span {
    pub fn lookup(self, source: &str) -> &str {
        &source[self.start.0 as usize..self.end.0 as usize]
    }
}

impl Debug for Span {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}..{}", self.start.0, self.end.0)
    }
}

impl Debug for Offset {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}
