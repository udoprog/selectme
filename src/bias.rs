use crate::set::{Iter, Number, Set};

/// Trait that implements bias in selection of branches.
pub trait Bias<Bits> {
    /// The applied bias iterator.
    type Apply: Iterator<Item = u32>;

    /// Apply the bias to the given snapshot and construct an iterator over its
    /// items.
    fn apply(&self, snapshot: Set<Bits>) -> Self::Apply;
}

/// An unbiased selector which starts from the top and works its way to the
/// bottom.
#[non_exhaustive]
pub struct Unbiased;

impl<Bits> Bias<Bits> for Unbiased
where
    Bits: Number,
{
    type Apply = Iter<Bits>;

    fn apply(&self, snapshot: Set<Bits>) -> Self::Apply {
        snapshot.iter()
    }
}

/// A biased selector which applies the given random pattern to selection.
#[non_exhaustive]
pub struct Random(u32);

impl Random {
    pub(crate) const fn new(value: u32) -> Self {
        Self(value)
    }
}

impl<Bits> Bias<Bits> for Random
where
    Bits: Number,
{
    type Apply = RandomIter<Bits>;

    fn apply(&self, set: Set<Bits>) -> Self::Apply {
        RandomIter {
            value: self.0,
            iter: Set::new(set.state().rotate_right(self.0)).iter(),
        }
    }
}

pub struct RandomIter<Bits> {
    value: u32,
    iter: Iter<Bits>,
}

impl<Bits> Iterator for RandomIter<Bits>
where
    Bits: Number,
{
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let value = Bits::from_bit(self.iter.next()?)
            .rotate_left(self.value)
            .trailing_zeros();

        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{Bias, Random};
    use crate::set::Set;

    #[test]
    fn test_random_bias() {
        let set = Set::new(2u32 + 64 + 128 + 1024);
        let random = Random::new(3);
        let mut it = random.apply(set);

        assert_eq!(it.next(), Some(6));
        assert_eq!(it.next(), Some(7));
        assert_eq!(it.next(), Some(10));
        assert_eq!(it.next(), Some(1));
        assert_eq!(it.next(), None);

        let set = Set::new(2u32 + 64 + 128 + 1024);
        let mut it = set.iter();

        assert_eq!(it.next(), Some(1));
        assert_eq!(it.next(), Some(6));
        assert_eq!(it.next(), Some(7));
        assert_eq!(it.next(), Some(10));
        assert_eq!(it.next(), None);
    }
}
