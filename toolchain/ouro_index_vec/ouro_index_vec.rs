pub use index_vec::*;

/// Defines a `u32` newtype that can index an [`IndexVec`].
/// `Debug` prints it like `Node(3)`.
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
    };
}

/// Opt-in `+`, `+=` and `-=` for an index type that is really a *quantity*,
/// like a byte offset or a token count. Works with another `$name` or with
/// a `usize`. Plain ids (`Node`, `Ref`) shouldn't get this: adding two node
/// ids means nothing.
#[macro_export]
macro_rules! impl_arithmetic {
    ($name:ident) => {
        impl ::std::ops::Add<usize> for $name {
            type Output = $name;
            fn add(self, rhs: usize) -> $name {
                <$name as $crate::Idx>::from_usize(<$name as $crate::Idx>::index(self) + rhs)
            }
        }

        impl ::std::ops::AddAssign<usize> for $name {
            fn add_assign(&mut self, rhs: usize) {
                *self = *self + rhs;
            }
        }

        impl ::std::ops::SubAssign<usize> for $name {
            fn sub_assign(&mut self, rhs: usize) {
                *self = <$name as $crate::Idx>::from_usize(<$name as $crate::Idx>::index(*self) - rhs);
            }
        }

        impl ::std::ops::Add for $name {
            type Output = $name;
            fn add(self, rhs: $name) -> $name {
                $name(self.0 + rhs.0)
            }
        }

        impl ::std::ops::AddAssign for $name {
            fn add_assign(&mut self, rhs: $name) {
                self.0 += rhs.0;
            }
        }

        impl ::std::ops::SubAssign for $name {
            fn sub_assign(&mut self, rhs: $name) {
                self.0 -= rhs.0;
            }
        }
    };
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
