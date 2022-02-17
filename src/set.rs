use core::fmt;

/// A snapshot of a set that can be iterated over.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Set<T> {
    state: T,
}

impl<T> Set<T>
where
    T: Number,
{
    /// Construct a new snapshot with the specified `value`.
    #[inline]
    pub(crate) fn new(state: T) -> Self {
        Self { state }
    }

    /// Access the interior state of the set.
    #[inline]
    pub(crate) fn state(&self) -> T {
        self.state
    }

    /// Test if the set is empty.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.state.is_zero()
    }

    /// Clear the given index.
    #[inline]
    pub fn clear(&mut self, index: u32) {
        self.state.unset(index);
    }

    /// Get the next index in the set.
    #[inline]
    pub fn next_index(&mut self) -> Option<u32> {
        if self.state.is_zero() {
            return None;
        }

        Some(self.state.trailing_zeros())
    }

    /// Construct an iterator over the snapshot.
    #[inline]
    pub(crate) fn iter(self) -> Iter<T> {
        Iter { state: self.state }
    }
}

pub struct Iter<T> {
    state: T,
}

impl<T> Iterator for Iter<T>
where
    T: Number,
{
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state.is_zero() {
            return None;
        }

        let index = self.state.trailing_zeros();
        self.state.unset(index);
        Some(index)
    }
}

impl<T> fmt::Debug for Set<T>
where
    T: Number,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

pub trait Number: Sized + Copy {
    const BITS: u32;

    /// Construct from a bit.
    fn from_bit(bit: u32) -> Self;

    /// Trailing zeros.
    fn trailing_zeros(self) -> u32;

    /// Test if the current number is zero.
    fn is_zero(self) -> bool;

    /// Unset the given bit.
    fn unset(&mut self, index: u32);

    /// Rotate the current number left the given number of bits.
    fn rotate_left(self, bits: u32) -> Self;

    /// Rotate the current number right the given number of bits.
    fn rotate_right(self, bits: u32) -> Self;
}

macro_rules! number {
    ($ty:ty) => {
        impl Number for $ty {
            const BITS: u32 = <$ty>::BITS as u32;

            fn from_bit(bit: u32) -> Self {
                1 << bit
            }

            fn trailing_zeros(self) -> u32 {
                <$ty>::trailing_zeros(self)
            }

            fn is_zero(self) -> bool {
                self == 0
            }

            fn unset(&mut self, index: u32) {
                *self &= !(1 << index);
            }

            fn rotate_left(self, bits: u32) -> Self {
                <$ty>::rotate_left(self, bits)
            }

            fn rotate_right(self, bits: u32) -> Self {
                <$ty>::rotate_right(self, bits)
            }
        }
    };
}

number!(u128);
number!(u64);
number!(u32);
number!(u16);
number!(u8);
