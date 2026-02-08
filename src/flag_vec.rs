use crate::default_vec::DefaultVec;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;

type Elt = u32;

pub trait FlagLength: Clone + Eq {
    fn len(&self) -> u32;

    #[inline]
    fn base_mask(&self) -> Elt {
        (1 << self.len()) - 1
    }

    #[inline]
    fn chunk_size(&self) -> u32 {
        Elt::BITS / self.len()
    }

    #[inline]
    fn split(&self, x: usize) -> (usize, u32) {
        let chunk_size = self.chunk_size();
        let offset = (x % chunk_size as usize) as u32 * self.len();
        (x / chunk_size as usize, offset)
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Static<const N: u32>;

impl<const N: u32> FlagLength for Static<N> {
    #[inline]
    fn len(&self) -> u32 {
        N
    }
}

impl FlagLength for u32 {
    #[inline]
    fn len(&self) -> u32 {
        *self
    }
}

/// A data structure an indexed collection of `N` bit bit-flag like objects
/// Behaves like [`DefaultVec<u32>`](DefaultVec) but only considers the last `N` bits
///
/// Use [`StaticFlagVec`] when `N` is known at compile time, or [`DynamicFlagVec`] if it isn't
pub struct FlagVec<N: FlagLength, I = usize>(N, DefaultVec<Elt>, PhantomData<I>);

/// [`FlagVec`] where `N` is known at compile time
pub type StaticFlagVec<const N: u32, I = usize> = FlagVec<Static<N>, I>;

impl<const N: u32, I> Default for StaticFlagVec<N, I> {
    fn default() -> Self {
        FlagVec(Static, DefaultVec::default(), PhantomData)
    }
}

/// [`DynamicFlagVec`] where `N` is not known at compile time
pub type DynamicFlagVec<I = usize> = FlagVec<u32, I>;

impl DynamicFlagVec {
    pub fn new(flag_bit_len: u32) -> Self {
        FlagVec(flag_bit_len, DefaultVec::default(), PhantomData)
    }
}

impl<N: FlagLength, I> Clone for FlagVec<N, I> {
    fn clone(&self) -> Self {
        FlagVec(self.0.clone(), self.1.clone(), PhantomData)
    }

    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
        self.1.clone_from(&source.1);
    }
}

impl<N: FlagLength, I> PartialEq<Self> for FlagVec<N, I> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl<N: FlagLength, I> Eq for FlagVec<N, I> {}

impl<N: FlagLength, I: Into<usize>> FlagVec<N, I> {
    /// Equivalent to `self.set(x, self.get(x) | v)`
    ///
    /// ```
    /// use default_vec2::StaticFlagVec;
    /// let mut s: StaticFlagVec<4> = StaticFlagVec::default();
    /// s.or_assign(0, 3);
    /// assert_eq!(s.get(0), 3);
    /// s.or_assign(0, 9);
    /// assert_eq!(s.get(0), 11);
    /// ```
    pub fn or_assign(&mut self, x: I, v: Elt) {
        let (chunk_idx, offset) = self.0.split(x.into());
        let mask = (self.0.base_mask() & v) << offset;
        let chunk = self.1.get_mut(chunk_idx);
        *chunk |= mask;
    }

    /// Equivalent to `self.set(x, self.get(x) & v)`
    ///
    /// ```
    /// use default_vec2::StaticFlagVec;
    /// let mut s: StaticFlagVec<4> = StaticFlagVec::default();
    /// s.or_assign(0, 11);
    /// s.and_assign(0, 5);
    /// assert_eq!(s.get(0), 1);
    /// ```
    pub fn and_assign(&mut self, x: I, v: u32) {
        let (chunk_idx, offset) = self.0.split(x.into());
        let mask = (self.0.base_mask() & !v) << offset;
        let chunk = self.1.get_mut(chunk_idx);
        *chunk &= !mask;
    }

    /// Sets the element and index `x` to the last `N` bits of `v`
    ///
    /// ```
    /// use default_vec2::StaticFlagVec;
    /// let mut s: StaticFlagVec<4> = StaticFlagVec::default();
    /// s.set(0, 3);
    /// assert_eq!(s.get(0), 3);
    /// s.set(0, 9);
    /// assert_eq!(s.get(0), 9);
    /// s.set(0, 18);
    /// assert_eq!(s.get(0), 2);
    /// ```
    pub fn set(&mut self, x: I, v: u32) {
        let (chunk_idx, offset) = self.0.split(x.into());
        let mask_or = (self.0.base_mask() & v) << offset;
        let mask_and = (self.0.base_mask() & !v) << offset;
        let chunk = self.1.get_mut(chunk_idx);
        *chunk |= mask_or;
        *chunk &= !mask_and;
    }

    /// Returns the element and index `x`
    pub fn get(&self, x: I) -> Elt {
        let (chunk_idx, offset) = self.0.split(x.into());
        let chunk = self.1.get(chunk_idx);
        (chunk >> offset) & self.0.base_mask()
    }

    /// Same as `get` but already reserves space for `x`
    pub fn get_reserve(&mut self, x: I) -> Elt {
        let (chunk_idx, offset) = self.0.split(x.into());
        let chunk = self.1.get_mut(chunk_idx);
        (*chunk >> offset) & self.0.base_mask()
    }

    /// Removes all elements from the set
    pub fn clear(&mut self) {
        self.1.clear()
    }

    pub fn capacity(&self) -> usize {
        self.1.capacity() * self.0.chunk_size() as usize
    }

    /// Iterate over the elements of `self`
    /// ```
    /// use default_vec2::StaticFlagVec;
    /// let mut s: StaticFlagVec<10> = StaticFlagVec::default();
    /// s.set(0, 42);
    /// s.set(2, 999);
    /// s.set(3, 365);
    /// let res: Vec<_> = s.iter().collect();
    /// assert_eq!(&res[..4], [42, 0, 999, 365])
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = Elt> + '_ {
        self.1.iter().flat_map(move |x| {
            let mut x = *x;
            (0..self.0.chunk_size()).map(move |_| {
                let res = x & self.0.base_mask();
                x >>= self.0.len();
                res
            })
        })
    }
}

#[test]
fn test() {
    use alloc::vec;
    let v1 = vec![
        145, 114, 177, 130, 57, 228, 108, 147, 117, 119, 102, 143, 216, 215, 2, 215, 191, 217, 96,
        157, 200, 82, 220, 211, 66, 183, 16, 173, 174, 246, 232, 248, 174, 40, 33, 169, 12, 191,
        171, 24, 32, 196, 104, 101, 216, 155, 132, 91, 32, 95, 122, 149, 64, 56, 218, 129, 41, 26,
        63, 87, 77, 120, 101, 213, 26, 141, 166, 167, 70, 16, 136, 159, 157, 144, 94, 52, 121, 188,
        219, 72, 75, 74, 223, 148, 160, 13, 126, 89, 148, 149, 24, 221, 120, 204, 148, 167, 179,
        120, 93, 126,
    ];
    let v2 = vec![
        177, 234, 192, 132, 135, 63, 26, 136, 95, 252, 137, 0, 200, 214, 60, 61, 93, 45, 218, 173,
        81, 162, 23, 188, 97, 228, 26, 159, 199, 238, 128, 59, 19, 143, 15, 133, 52, 182, 208, 56,
        54, 99, 83, 47, 80, 208, 8, 64, 142, 205, 248, 11, 71, 252, 101, 32, 219, 117, 160, 120,
        217, 111, 3, 69, 215, 49, 122, 147, 147, 7, 199, 157, 69, 73, 159, 250, 241, 136, 85, 101,
        14, 32, 118, 64, 123, 154, 236, 7, 239, 206, 159, 12, 170, 84, 101, 136, 54, 138, 182, 247,
    ];
    assert_eq!(v1.len(), v2.len());
    let vor: Vec<_> = v1.iter().zip(&v2).map(|(&x, &y)| x | y).collect();
    let vand: Vec<_> = v1.iter().zip(&v2).map(|(&x, &y)| x & y).collect();
    let mut fv1 = StaticFlagVec::<9>::default();
    for (i, &x) in v1.iter().enumerate() {
        fv1.set(i, x);
    }
    assert_eq!(&fv1.iter().collect::<Vec<_>>()[..100], &v1);
    let mut fv2 = fv1.clone();
    for (i, &x) in v2.iter().enumerate() {
        assert_eq!(fv2.get(i), v1[i]);
        fv2.set(i, x);
    }
    assert_eq!(&fv2.iter().collect::<Vec<_>>()[..100], &v2);
    let mut fvor = fv1.clone();
    for (i, &x) in v2.iter().enumerate() {
        fvor.or_assign(i, x);
    }
    assert_eq!(&fvor.iter().collect::<Vec<_>>()[..100], &vor);
    let mut fvand = fvor;
    fvand.clone_from(&fv1);
    for (i, &x) in v2.iter().enumerate() {
        fvand.and_assign(i, x);
    }
    assert_eq!(&fvand.iter().collect::<Vec<_>>()[..100], &vand);
}

impl<N: FlagLength, I: Into<usize>> Debug for FlagVec<N, I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
