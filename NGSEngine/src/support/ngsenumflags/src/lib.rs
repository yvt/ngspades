//! # Nightingales Enumflags
//!
//! Based on [`enumflags`] (dual licensed under Apache 2.0 and MIT).
//! See the `README.md` of `enumflags` for the detailed usage.
//!
//! [`enumflags`]: https://github.com/MaikKlein/enumflags
//!
//! ## Enhancements / Changes
//!
//!  - `BitFlags` implements compound-assignment operators.
//!  - `BitFlags` implements `Hash`.
//!  - Generated `InnerXXX` types are now marked as `#[doc(hidden)]` and you
//!    no longer have to hide them manually
//!  - Provides a helper macro named [`flags!`].
//!
//! [`flags!`]: flags
//!
//! ## Examples
//!
//!     extern crate ngsenumflags;
//!     #[macro_use]
//!     extern crate ngsenumflags_derive;
//!
//!     # fn main() {
//!     #[derive(NgsEnumFlags, Copy, Clone, PartialEq, Eq, Hash, Debug)]
//!     #[repr(u8)]
//!     pub enum Test {
//!         A = 0b0001,
//!         B = 0b0010,
//!         C = 0b0100,
//!         D = 0b1000,
//!     }
//!     # }
use std::ops::{BitAnd, BitOr, BitXor, Not};
use std::cmp;

pub trait EnumFlagSize {
    type Size: InnerBitFlags;
}

pub trait InnerBitFlags: BitOr<Self> + cmp::PartialEq + cmp::Eq + Clone + Copy
where
    Self: Sized,
{
    type Type;
    fn all() -> Self;
    fn empty() -> Self;
    fn is_empty(self) -> bool;
    fn is_all(self) -> bool;
    fn bits(self) -> Self::Type;
    fn intersects(self, other: Self) -> bool;
    fn contains(self, other: Self) -> bool;
    fn not(self) -> Self;
    fn from_bits(bits: Self::Type) -> Option<Self>;
    fn from_bits_truncate(bits: Self::Type) -> Self;
    fn insert(&mut self, other: Self);
    fn remove(&mut self, other: Self);
    fn toggle(&mut self, other: Self);
}

#[derive(Eq, Copy, Clone, Hash)]
pub struct BitFlags<T: EnumFlagSize> {
    val: T::Size,
}

impl<T> ::std::fmt::Debug for BitFlags<T>
where
    T: EnumFlagSize,
    T::Size: ::std::fmt::Debug,
{
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        fmt.write_str(&format!(
            "BitFlags {o} {inner:?} {c} ",
            o = "{",
            inner = self.val,
            c = "}"
        ))
    }
}

impl<T> BitFlags<T>
where
    T: EnumFlagSize,
{
    /// Create a new BitFlags unsafely. Consider using `from_bits` or `from_bits_truncate`.
    pub unsafe fn new(val: T::Size) -> Self {
        BitFlags { val: val }
    }
}

impl<T> BitFlags<T>
where
    T: EnumFlagSize,
    T::Size: InnerBitFlags + Into<BitFlags<T>>,
{
    /// Create an empty BitFlags. Empty means `0`.
    pub fn empty() -> Self {
        T::Size::empty().into()
    }

    /// Sets all flags.
    pub fn all() -> Self {
        T::Size::all().into()
    }

    /// Returns true if all flags are set
    pub fn is_all(self) -> bool {
        self.val.is_all()
    }

    /// Returns true if no flag is set
    pub fn is_empty(self) -> bool {
        self.val.is_empty()
    }

    /// Returns the underlying type value
    pub fn bits(self) -> <T::Size as InnerBitFlags>::Type {
        self.val.bits()
    }

    /// Returns true if at least one flag is shared.
    pub fn intersects<B: Into<BitFlags<T>>>(self, other: B) -> bool {
        T::Size::intersects(self.val, other.into().val)
    }

    /// Returns true iff all flags are contained.
    pub fn contains<B: Into<BitFlags<T>>>(self, other: B) -> bool {
        T::Size::contains(self.val, other.into().val)
    }

    /// Flips all flags
    pub fn not(self) -> Self {
        self.val.not().into()
    }

    /// Returns a BitFlags iff the bits value does not contain any illegal flags.
    pub fn from_bits(bits: <T::Size as InnerBitFlags>::Type) -> Option<Self> {
        T::Size::from_bits(bits).map(|v| v.into())
    }

    /// Truncates flags that are illegal
    pub fn from_bits_truncate(bits: <T::Size as InnerBitFlags>::Type) -> Self {
        T::Size::from_bits_truncate(bits).into()
    }

    pub fn toggle(&mut self, other: Self) {
        T::Size::toggle(&mut self.val, other.val);
    }

    pub fn insert(&mut self, other: Self) {
        T::Size::insert(&mut self.val, other.val);
    }

    pub fn remove(&mut self, other: Self) {
        T::Size::remove(&mut self.val, other.val);
    }
}

impl<T> std::cmp::PartialEq for BitFlags<T>
where
    T: EnumFlagSize,
{
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
    }
}

// impl<T> std::ops::BitOr for BitFlags<T>
//    where T: EnumFlagSize ,
//          T::Size: BitOr<T::Size, Output = T::Size> + Into<BitFlags<T>>
// {
//    type Output = BitFlags<T>;
//    fn bitor(self, other: Self) -> BitFlags<T> {
//        (self.val | other.val).into()
//    }
// }

impl<T, B> std::ops::BitOr<B> for BitFlags<T>
where
    T: EnumFlagSize,
    B: Into<BitFlags<T>>,
    T::Size: BitOr<T::Size, Output = T::Size> + Into<BitFlags<T>>,
{
    type Output = BitFlags<T>;
    fn bitor(self, other: B) -> BitFlags<T> {
        (self.val | other.into().val).into()
    }
}

impl<T, B> std::ops::BitAnd<B> for BitFlags<T>
where
    T: EnumFlagSize,
    B: Into<BitFlags<T>>,
    T::Size: BitAnd<T::Size, Output = T::Size> + Into<BitFlags<T>>,
{
    type Output = BitFlags<T>;
    fn bitand(self, other: B) -> BitFlags<T> {
        (self.val & other.into().val).into()
    }
}

impl<T, B> std::ops::BitXor<B> for BitFlags<T>
where
    T: EnumFlagSize,
    B: Into<BitFlags<T>>,
    T::Size: BitXor<T::Size, Output = T::Size> + Into<BitFlags<T>>,
{
    type Output = BitFlags<T>;
    fn bitxor(self, other: B) -> BitFlags<T> {
        (self.val ^ other.into().val).into()
    }
}

impl<T, B> std::ops::BitOrAssign<B> for BitFlags<T>
where
    T: EnumFlagSize,
    B: Into<BitFlags<T>>,
    T::Size: BitOr<T::Size, Output = T::Size> + Into<BitFlags<T>>,
{
    fn bitor_assign(&mut self, other: B) {
        *self = (self.val | other.into().val).into();
    }
}

impl<T, B> std::ops::BitAndAssign<B> for BitFlags<T>
where
    T: EnumFlagSize,
    B: Into<BitFlags<T>>,
    T::Size: BitAnd<T::Size, Output = T::Size> + Into<BitFlags<T>>,
{
    fn bitand_assign(&mut self, other: B) {
        *self = (self.val & other.into().val).into();
    }
}
impl<T, B> std::ops::BitXorAssign<B> for BitFlags<T>
where
    T: EnumFlagSize,
    B: Into<BitFlags<T>>,
    T::Size: BitXor<T::Size, Output = T::Size> + Into<BitFlags<T>>,
{
    fn bitxor_assign(&mut self, other: B) {
        *self = (self.val ^ other.into().val).into();
    }
}

impl<T> std::ops::Not for BitFlags<T>
where
    T: EnumFlagSize,
    T::Size: Not<Output = T::Size> + Into<BitFlags<T>>,
{
    type Output = BitFlags<T>;
    fn not(self) -> BitFlags<T> {
        (!self.val).into()
    }
}

/// Convenient macro for constructing a `BitFlags`.
///
/// # Examples
///
///     #[macro_use]
///     extern crate ngsenumflags;
///     # #[macro_use]
///     # extern crate ngsenumflags_derive;
///     # fn main() {
///     #[derive(NgsEnumFlags, Copy, Clone, PartialEq, Eq, Hash, Debug)]
///     #[repr(u8)]
///     pub enum Test {A = 0b0001, B = 0b0010}
///
///     let flags0 = flags![Test::{}];
///     let flags1 = flags![Test::{A}];
///     let flags2 = flags![Test::{A | B}];
///
///     assert_eq!(flags0, ngsenumflags::BitFlags::empty());
///     assert_eq!(flags1, Test::A.into());
///     assert_eq!(flags2, Test::A | Test::B);
///     # }
#[macro_export]
macro_rules! flags {
    ( $($ns:ident::)* {} ) => (
        $($ns::)*empty_bitflag()
    );

    ( $($ns:ident::)* {$tail:ident} ) => (
        $crate::BitFlags::from($($ns::)*$tail)
    );

    // Use `{}` instead of `()` to avoid `unused_parens` warning
    ( $($ns:ident::)* {$head:ident | $($rest:tt)*} ) => ({
        flags![$($ns::)*{$head}] |
        flags![$($ns::)*{$($rest)*}]
    })
}
