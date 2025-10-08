use core::cmp::Ordering;
use core::fmt::Debug;
use core::ops::{Add, Sub};

mod seal {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
}

pub trait UInt: seal::Sealed + Copy + Clone + Debug + PartialEq + Eq + Ord {
    const BITS: u32;
    const MAX: Self;
    const ZERO: Self;
    const ONE: Self;

    fn wrapping_add(self, rhs: Self) -> Self;
    fn wrapping_sub(self, rhs: Self) -> Self;
    fn shl(self, by: u32) -> Self; // left shift (by < BITS)
    fn bit_and(self, rhs: Self) -> Self;
}

macro_rules! impl_uint {
    ($t:ty) => {
        impl UInt for $t {
            const BITS: u32 = <$t>::BITS;
            const MAX: $t = <$t>::MAX;
            const ZERO: $t = 0 as $t;
            const ONE: $t = 1 as $t;

            #[inline]
            fn wrapping_add(self, rhs: Self) -> Self {
                <$t>::wrapping_add(self, rhs)
            }
            #[inline]
            fn wrapping_sub(self, rhs: Self) -> Self {
                <$t>::wrapping_sub(self, rhs)
            }
            #[inline]
            fn shl(self, by: u32) -> Self {
                self.wrapping_shl(by)
            }
            #[inline]
            fn bit_and(self, rhs: Self) -> Self {
                self & rhs
            }
        }
    };
}

impl_uint!(u8);
impl_uint!(u16);
impl_uint!(u32);
impl_uint!(u64);

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct SequenceInt<T, const BITS: u8>(pub T)
where
    T: UInt;

impl<T, const BITS: u8> SequenceInt<T, BITS>
where
    T: UInt,
{
    #[inline]
    const fn is_full_width() -> bool {
        (BITS as u32) == T::BITS
    }

    #[inline]
    fn mod_mask() -> T {
        debug_assert!(BITS >= 1, "BITS must be >= 1");
        debug_assert!((BITS as u32) <= T::BITS, "BITS must be <= storage width");
        if Self::is_full_width() {
            T::MAX
        } else {
            // (1<<BITS) - 1 in T
            T::ONE.shl(BITS as u32).wrapping_sub(T::ONE)
        }
    }

    #[inline]
    fn half_range() -> T {
        // 1 << (BITS - 1)
        T::ONE.shl((BITS - 1) as u32)
    }

    #[inline]
    fn mask(v: T) -> T {
        if Self::is_full_width() {
            v
        } else {
            v.bit_and(Self::mod_mask())
        }
    }

    pub fn inc(&mut self) {
        self.0 = Self::mask(self.0.wrapping_add(T::ONE));
    }
    pub fn dec(&mut self) {
        self.0 = Self::mask(self.0.wrapping_sub(T::ONE));
    }
}

impl<T, const BITS: u8> From<T> for SequenceInt<T, BITS>
where
    T: UInt,
{
    #[inline]
    fn from(value: T) -> Self {
        Self(Self::mask(value))
    }
}

// Eq and Ord according to RFC 1982 (TCP-style)
impl<T, const BITS: u8> PartialEq for SequenceInt<T, BITS>
where
    T: UInt,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T, const BITS: u8> Eq for SequenceInt<T, BITS> where T: UInt {}

impl<T, const BITS: u8> PartialOrd for SequenceInt<T, BITS>
where
    T: UInt,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, const BITS: u8> Ord for SequenceInt<T, BITS>
where
    T: UInt,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let a = self.0;
        let b = other.0;

        // (b - a) mod 2^BITS
        let mut diff = b.wrapping_sub(a);
        if !Self::is_full_width() {
            diff = diff.bit_and(Self::mod_mask());
        }

        if diff == T::ZERO {
            Ordering::Equal
        } else if diff < Self::half_range() {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl<T, const BITS: u8> Add<T> for SequenceInt<T, BITS>
where
    T: UInt,
{
    type Output = Self;
    #[inline]
    fn add(self, rhs: T) -> Self::Output {
        let s = self.0.wrapping_add(rhs);
        Self(Self::mask(s))
    }
}

impl<T, const BITS: u8> Sub<T> for SequenceInt<T, BITS>
where
    T: UInt,
{
    type Output = Self;
    #[inline]
    fn sub(self, rhs: T) -> Self::Output {
        let d = self.0.wrapping_sub(rhs);
        Self(Self::mask(d))
    }
}

// A few common widths
pub type SeqU8 = SequenceInt<u8, 8>;
pub type SeqU16 = SequenceInt<u16, 16>;
pub type SeqU24 = SequenceInt<u32, 24>;
pub type SeqU32 = SequenceInt<u32, 32>;
pub type SeqU64 = SequenceInt<u64, 64>;

#[test]
fn test_serial_ints() {
    assert!(SeqU16::from(1000u16) > SeqU16::from(999u16));
    assert!(SeqU16::from(65530u16) < SeqU16::from(10u16));
    assert_eq!(SeqU16::from(7u16), SeqU16::from(7u16));

    assert!(SeqU24::from(16_777_206u32) < SeqU24::from(16_777_208u32));
    assert!(SeqU24::from(16_777_206u32) < SeqU24::from(10u32));
    assert_eq!(SeqU24::from(16_777_226u32), SeqU24::from(10u32));

    assert_eq!(SeqU32::from(1000u32) + 7u32, SeqU32::from(1007u32));
    assert_eq!(SeqU32::from(4_294_967_290u32) + 10u32, SeqU32::from(4u32));

    type S14 = SequenceInt<u32, 14>;
    let max = (1u32 << 14) - 1;
    assert!(S14::from(max - 2) < S14::from(5));
    assert_eq!(S14::from(max) + 2u32, S14::from(1));

    type S = SequenceInt<u64, 64>;
    let a = S::from(u64::MAX - 2);
    let b = S::from(3u64);
    assert!(a < b);
    assert_eq!(S::from(u64::MAX) + 2u64, S::from(1u64));

    assert!(SeqU16::from(1000u16) > SeqU16::from(999u16));
    assert!(SeqU16::from(65530u16) < SeqU16::from(10u16));
    assert!(SeqU16::from(7u16) == SeqU16::from(7u16));

    assert!(SeqU24::from(16777206u32) < SeqU24::from(16777208u32));
    assert!(SeqU24::from(16777206u32) < SeqU24::from(10u32));
    assert!(SeqU24::from(16777226u32) == SeqU24::from(10u32));

    assert_eq!(SeqU32::from(1000u32) + 7, SeqU32::from(1007u32));
    assert_eq!(SeqU32::from(4294967290u32) + 10, SeqU32::from(4u32));

    let mut x = SeqU32::from(0xffff_fffe);
    x.inc();
    assert_eq!(x, SeqU32::from(0xffff_ffff_u32));
    x.inc();
    assert_eq!(x, SeqU32::from(0));

    assert!(SeqU16::from(1000) < SeqU16::from(33000));
    assert!(SeqU16::from(1000) > SeqU16::from(34000));
}
