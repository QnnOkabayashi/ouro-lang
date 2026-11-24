pub use index_vec::*;

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Defines a `u32` newtype that can index an [`IndexVec`].
/// `Debug` prints it like `Node(3)`.
///
/// Ids are points, so they only do point arithmetic, with a [`Delta`]:
/// `id + delta`, `id - delta`, and `id - id` (which gives a `Delta`).
///
/// ```ignore
/// define_index_type! {
///     pub struct Node = u32;
/// }
/// ```
#[macro_export]
macro_rules! define_index_type {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident = u32;
    ) => {
        $(#[$attr])*
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name(u32);

        impl $crate::Idx for $name {
            fn from_usize(idx: usize) -> Self {
                $name(u32::try_from(idx).expect("index doesn't fit in a u32"))
            }

            fn index(self) -> usize {
                self.0 as usize
            }
        }

        // For when the other side is a `u32`. (For `usize`, use `Idx`.)
        impl $name {
            pub fn from_raw(raw: u32) -> Self {
                $name(raw)
            }

            pub fn raw(self) -> u32 {
                self.0
            }
        }

        impl ::std::ops::Add<$crate::Delta<$name>> for $name {
            type Output = $name;
            fn add(self, delta: $crate::Delta<$name>) -> $name {
                $name(self.0 + delta.raw())
            }
        }

        impl ::std::ops::Sub<$crate::Delta<$name>> for $name {
            type Output = $name;
            fn sub(self, delta: $crate::Delta<$name>) -> $name {
                $name(self.0 - delta.raw())
            }
        }

        impl ::std::ops::AddAssign<$crate::Delta<$name>> for $name {
            fn add_assign(&mut self, delta: $crate::Delta<$name>) {
                *self = *self + delta;
            }
        }

        impl ::std::ops::SubAssign<$crate::Delta<$name>> for $name {
            fn sub_assign(&mut self, delta: $crate::Delta<$name>) {
                *self = *self - delta;
            }
        }

        /// How far `start` is behind `self`. Panics in debug builds if
        /// `start` is ahead.
        impl ::std::ops::Sub for $name {
            type Output = $crate::Delta<$name>;
            fn sub(self, start: $name) -> $crate::Delta<$name> {
                $crate::Delta::from_raw(self.0 - start.0)
            }
        }
    };
}

/// The difference between two `I`s, like a `Duration` is to an `Instant`.
///
/// Unsigned: going backwards is `id - delta`, not a negative `Delta`.
///
/// Only constructible for index types: the constructors require `I: Idx`.
pub struct Delta<I> {
    raw: u32,
    _marker: PhantomData<fn() -> I>,
}

impl<I: Idx> Delta<I> {
    pub const ZERO: Self = Delta::from_raw(0);
    pub const ONE: Self = Delta::from_raw(1);

    pub const fn from_raw(raw: u32) -> Self {
        Delta {
            raw,
            _marker: PhantomData,
        }
    }

    pub fn from_usize(delta: usize) -> Self {
        Delta::from_raw(u32::try_from(delta).expect("delta doesn't fit in a u32"))
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    pub const fn to_usize(self) -> usize {
        self.raw as usize
    }
}

impl<I: Idx> Add for Delta<I> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Delta::from_raw(self.raw + rhs.raw)
    }
}

impl<I: Idx> Sub for Delta<I> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Delta::from_raw(self.raw - rhs.raw)
    }
}

impl<I: Idx> AddAssign for Delta<I> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<I: Idx> SubAssign for Delta<I> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

// Written by hand: `derive` would wrongly require `I` to implement each trait.
impl<I> Clone for Delta<I> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<I> Copy for Delta<I> {}
impl<I: Idx> Default for Delta<I> {
    fn default() -> Self {
        Delta::ZERO
    }
}
impl<I> PartialEq for Delta<I> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<I> Eq for Delta<I> {}
impl<I> PartialOrd for Delta<I> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<I> Ord for Delta<I> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}
impl<I> Hash for Delta<I> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}
impl<I> fmt::Debug for Delta<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Delta({})", self.raw)
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

impl<I: Idx> Default for MaxOr<I> {
    fn default() -> Self {
        MaxOr::max()
    }
}
