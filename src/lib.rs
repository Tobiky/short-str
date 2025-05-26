#![no_std]
use core::{
    convert::Infallible,
    fmt::{Debug, Display},
    marker::PhantomData,
    mem::transmute,
    ops::{Deref, Range, RangeBounds},
    ptr::copy_nonoverlapping,
};

#[cfg(test)]
mod tests;

#[cfg(debug_assertions)]
const _: () = const {
    use const_panic::concat_assert;

    const REPO_URL: &'static str = "https://github.com/Tobiky/short-str";

    // Little Endian
    // Not supported by miri and #[ignore] is only on functions
    #[cfg(not(miri))]
    concat_assert!(
        unsafe { transmute::<&str, [u8; BYTE_SIZE]>("test") }[PTR_SIZE] as usize == "test".len(),
        "big endian architecture is currently unsupported for ShortStr's",
    );

    // &str Size
    concat_assert!(
        size_of::<&str>() == PTR_SIZE + LEN_SIZE,
        "expected &str to have size ",
        PTR_SIZE + LEN_SIZE,
        "(",
        PTR_SIZE,
        " + ",
        LEN_SIZE,
        ") but got size ",
        size_of::<&str>(),
        ", please file an issue at",
        REPO_URL
    );

    // core::any::type_name::<CoveringInt>(),
    // CoveringInt size coverage
    concat_assert!(
        size_of::<CoveringInt>() == size_of::<ShortStr>(),
        "expected CoveringInt to match byte size with ShortStr/ShStr (",
        size_of::<CoveringInt>(),
        " vs. ",
        size_of::<ShortStr>(),
        "), please file an issue at ",
        REPO_URL
    );
};

const PTR_SIZE: usize = size_of::<usize>();
const LEN_SIZE: usize = size_of::<usize>();
const BYTE_SIZE: usize = PTR_SIZE + LEN_SIZE;
const INLINE_BYTE_SIZE: usize = BYTE_SIZE - 1;

#[cfg(target_pointer_width = "64")]
type CoveringInt = u128;
#[cfg(target_pointer_width = "32")]
type CoveringInt = u64;
#[cfg(target_pointer_width = "16")]
type CoveringInt = u32;

// Size is MSB for little endian
const SIZE_MASK: CoveringInt = (0xff as CoveringInt).rotate_right(8);
const DATA_MASK: CoveringInt = !SIZE_MASK;

// layout of &str is ptr, len
// see `verify_layout` test
#[derive(Clone, Copy, Eq, PartialOrd, Ord)]
pub struct ShortStr<'str_lt> {
    _lt: PhantomData<&'str_lt Infallible>,
    data: [u8; BYTE_SIZE],
}
pub type ShStr<'str_lt> = ShortStr<'str_lt>;

impl Debug for ShortStr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl Display for ShortStr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
enum Variant<'str_lt> {
    Inlined([u8; BYTE_SIZE]),
    Facade(&'str_lt str),
    Empty,
}

impl<'str_lt> Variant<'str_lt> {
    #[inline(always)]
    const fn from_short_str(value: ShortStr<'str_lt>) -> Self {
        if value.is_str() {
            // Safety:
            // is_str_ref garantuees that `value` is indeed a &str
            // ERROR(miri): miri cannot figure out that this is an actual &'str_lt (since it would
            // have been constructed from that, necessarily)
            let str_ref = unsafe { transmute::<ShortStr, &'str_lt str>(value) };
            Variant::Facade(str_ref)
        } else if value.is_empty_inlined() {
            Variant::Empty
        } else {
            Variant::Inlined(value.data)
        }
    }
}

impl<'str_lt> From<ShortStr<'str_lt>> for Variant<'str_lt> {
    #[inline(always)]
    fn from(value: ShortStr<'str_lt>) -> Self {
        Self::from_short_str(value)
    }
}

/// inline str zero length flag
// useful only with more flags
// 2 * size_of::<usize>() - 1 = how many bytes that can be stored
// ilog2 = how many bits to represent size
// disregard - 1 to ensure no size bits are encroached (ilog2 rounds down)
// ilog2(byte size + 1) = how many bits to move over since 1 is already at first place
// const MASK_INLINE_ZERO_LEN: usize = 1 << usize::ilog2(BYTE_SIZE);

impl<'str_lt> ShortStr<'str_lt> {
    /// An empty inlined [`ShortStr`].
    ///
    /// # Examples
    ///
    /// ```
    /// use short_str::ShortStr;
    ///
    /// assert_eq!(ShortStr::EMPTY, ShortStr::from(""));
    /// ```
    pub const EMPTY: ShortStr<'str_lt> = const {
        let mut data = [0; BYTE_SIZE];
        data[BYTE_SIZE - 1] = -1i8 as u8;
        ShortStr { data, _lt: PhantomData }
    };

    #[inline(always)]
    /// Returns the [`Variant`] of the `self` (Empty, Inlined, or Facade).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use short_str::ShortStr;
    ///
    /// let string = ShortStr::from("im tired of writing these strings");
    /// match string {
    ///     Variant::Facade(string) => println!("{}", string);
    ///     Variant::Inlined(_data) => unreachable!(),
    ///     Variant::Empty => unreachable!(),
    /// }
    /// ```
    const fn variant(self) -> Variant<'str_lt> {
        Variant::from_short_str(self)
    }

    #[inline(always)]
    /// Returns the byte, or masked byte in smaller cases, of the marker containing the length.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use short_str::ShortStr;
    ///
    /// let string = ShortStr::from("hello");
    /// assert_eq!(string.length_marker, "hello".len());
    /// ```
    const fn length_marker(self) -> u8 {
        // assumptions: little endian

        // ------------------------------------------------------------------------------
        // in the case of little endian the last byte is unrealistic to be set
        // as that would require more than e.g. 2^58 "directly" addressed bytes of memory
        // and therefore can be used for the inline str mode size, and as a marker.
        // ------------------------------------------------------------------------------
        self.data[BYTE_SIZE - 1]
    }

    #[inline(always)]
    /// Returns if the [`ShortStr`] is an empty inlined [`&str`] or not.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use short_str::ShortStr;
    ///
    /// let string = ShortStr::from("i am too big to fit inside the structure");
    /// assert_eq!(string.is_empty_inlined(), false);
    /// let string = ShortStr::from("i am small");
    /// assert_eq!(string.is_empty_inlined(), false);
    /// let string = ShortStr::from("");
    /// assert_eq!(string.is_empty_inlined(), true);
    /// let string = ShortStr::EMPTY;
    /// assert_eq!(string.is_empty_inlined(), true);
    /// ```
    const fn is_empty_inlined(self) -> bool {
        (self.length_marker() as i8).is_negative()
    }

    #[inline(always)]
    /// Returns if the [`ShortStr`] is a [`&str`] facade (not inlined) or not.
    ///
    /// # Examples
    ///
    /// ```
    /// use short_str::ShortStr;
    ///
    /// let not_inlined = ShortStr::from("i am too big to fit inside the structure");
    /// assert_eq!(not_inlined.is_str(), true);
    /// let inlined = ShortStr::from("i am small");
    /// assert_eq!(inlined.is_str(), false);
    /// ```
    pub const fn is_str(self) -> bool {
        self.length_marker() == 0
    }

    #[inline(always)]
    /// Returns the number of bytes in the [`ShortStr`].
    ///
    /// # Examples
    /// ```
    /// let a = [1, 2, 3];
    /// assert_eq!(a.len(), 3);
    /// ```
    pub const fn len(self) -> usize {
        match self.variant() {
            Variant::Inlined(data) => data[BYTE_SIZE - 1] as usize,
            Variant::Facade(str_ref) => str_ref.len(),
            Variant::Empty => 0,
        }
    }

    #[inline(always)]
    /// Produce a [`ShortStr`] from a `&str` without checking if it should be Inlined or not. The
    /// function is marked unsafe for the assumptions made on correct convertion relied upon for
    /// other functions.
    ///
    /// The representation might have differed if [`ShortStr::from`] had been used (e.g. <= 15
    /// bytes long str, on 64-bit platforms) which would cause the [`Eq`] and [`PartialEq`]
    /// operations to fail even if they represent the same value.
    ///
    /// For example
    /// ```should_panic
    /// use short_str::ShortStr;
    ///
    /// assert_eq!(unsafe { ShortStr::from_str_unchecked("test") }, ShortStr::from("test"));
    /// ```
    ///
    /// # Examples
    /// ```
    /// use short_str::ShortStr;
    ///
    /// let string = "i am not inlined";
    /// assert_eq!(unsafe { ShortStr::from_str_unchecked(string) }, ShortStr::from(string));
    /// ```
    pub const unsafe fn from_str_unchecked(other: &str) -> Self {
        // safety:
        // see ShortStr::length_marker(self)
        // any &str is a valid instance of ShortStr due to the nature of the struct
        unsafe { transmute::<&str, ShortStr>(other) }
    }

    #[inline(always)]
    /// Produce a [`ShortStr`] from a [`&str`], gaurantueeing that any [`&str`] from the
    /// [`ShortStr`] will have the same lifetime.
    ///
    /// # Examples
    /// These examples assume a little endian 64-bit architecture.
    /// ```
    /// use short_str::ShortStr;
    ///
    /// let inlined = ShortStr::from("hello, world!");
    /// assert_eq!(inlined.is_str(), false);
    /// let not_inlined = ShortStr::from("hello, world! i am big");
    /// assert_eq!(not_inlined.is_str(), true);
    /// ```
    pub const fn from_str(value: &'str_lt str) -> Self {
        // safety:
        // short_str is not &str, in which case its a ShortStr, and can thus be used as normal
        // short_str is a &str, in which case ShortStr is just handled like a facade
        let short_str = unsafe { Self::from_str_unchecked(value) };
        match short_str.variant() {
            // Special empty case
            Variant::Facade(facade) if facade.is_empty() => ShortStr::EMPTY,
            // It can fit into an inline str so convert
            Variant::Facade(facade) if facade.len() <= INLINE_BYTE_SIZE => {
                let mut data = [0; BYTE_SIZE];
                // safety:
                // this is just copy_from_slice but that as const isn't stable yet
                // - not same locations
                // - amount of bytes to copy is garantueed < INLINE_BYTE_SIZE by condition
                unsafe {
                    copy_nonoverlapping(facade.as_ptr(), data.as_mut_ptr(), facade.len());
                }
                data[BYTE_SIZE - 1] = facade.len() as u8;
                ShortStr { data, _lt: PhantomData }
            }
            // It's already a proper ShortStr
            // A: an inlined &str
            // B: a &str facade with len > INLINE_BYTE_SIZE
            // C: an empty inlined &str
            Variant::Facade(_) | Variant::Inlined(_) | Variant::Empty => short_str,
        }
    }

    #[inline(always)]
    /// View the [`ShortStr`] as a [`&str`]. There are two cases:
    /// - (not inlined) [`ShortStr`] is a facade for [`&str`]: the original `&str` is returned with
    ///   its original lifetime.
    /// - (inlined) [`ShortStr`] is true: a new [`&str`] is constructed to point at `self` with the
    ///   appropriate length.
    ///
    /// # Examples
    /// These examples assume a little endian 64-bit architecture.
    /// ```
    /// use short_str::ShortStr;
    ///
    /// let string    = "this is not inlined";
    /// let short_str = ShortStr::from(string);
    /// assert_eq!(short_str.is_str(), true);
    /// let re_string = short_str.as_str();
    /// assert_eq!(string, short_str);
    /// ```
    pub const fn as_str(&self) -> &'str_lt str {
        match self.variant() {
            Variant::Inlined(_) | Variant::Empty => {
                // safety:
                // the ShortStr is an inline str, starting at the same place as data and with length
                // we get from len
                unsafe {
                    let slice = core::slice::from_raw_parts(self.data.as_ptr(), self.len());
                    core::str::from_utf8_unchecked(slice)
                }
            }
            Variant::Facade(str_ref) => str_ref,
        }
    }

    /// Performs the slicing operation using `slice` on `self` without checking for logical
    /// consistencies between `slice` and `self`. See [`ShortStr::slice`] for the checked variant.
    /// There are four cases:
    /// - `slice.len() == 0`: a special empty [`ShortStr`] is produced
    /// - `slice.len() == self.len()`: just produces self
    /// - `self.is_str()` (plain [`&str`]): perform slice using native [`&str`] slicing into
    ///   [`ShortStr::from`]
    /// - `!self.is_str()` (inlined [`&str`]): bitbash the bytes into the correct sliced version,
    ///   for which an incorrect `slice` is UB.
    ///
    /// # Panics
    /// If `slice` is illogical (descending order or out of bounds values) or it splits graphemes,
    /// the operation only panics for the case of when its representing an actual [`&str`] (i.e.
    /// not inlined). See [`&str`] for more details (particularly on splitting graphemes).
    ///
    /// To be clear; splitting graphemes inside the inlined [`&str`] does currently not produce a
    /// panic.
    ///
    /// # Examples
    /// These examples assume a little endian 64-bit architecture.
    /// ```
    /// use short_str::ShortStr;
    ///
    /// let string = ShortStr::from("hello, world!");
    /// let slice  = unsafe { string.slice_unchecked(..5) };
    /// assert_eq!(slice, "hello");
    /// ```
    ///
    /// Attempting to use an incorrect slice on a [`&str`] facade results in a panic from the
    /// [`&str`]:
    /// ```should_panic
    /// use short_str::ShortStr;
    ///
    /// let not_inlined = ShortStr::from("hello, world! yet another inlined string");
    /// let slice       = unsafe { not_inlined.slice_unchecked(..not_inlined.len() + 1) };
    /// ```
    /// or splitting a grapheme:
    /// ```should_panic
    /// use short_str::ShortStr;
    ///
    /// let not_inlined = ShortStr::from("this string contains a unicode symbol 🈁s");
    /// let slice       = unsafe { not_inlined.slice_unchecked(..not_inlined.len() - 2) };
    /// ```
    ///
    /// But the same kind of operation on an inlined [`&str`] does not panic as it just performs
    /// bit operations using the range bounds as values:
    /// ```
    /// use short_str::ShortStr;
    ///
    /// let inlined = ShortStr::from("hello, world!");
    /// assert_eq!(inlined.len(), 13);
    /// let slice   = unsafe { inlined.slice_unchecked(..inlined.len() + 1) };
    /// assert_eq!(slice.len(), 14);
    /// assert_eq!(slice, "hello, world!\0");
    /// ```
    /// Neither does it panic on splitting graphemes, though this will panic since its invalid
    /// UTF-8 (the equality also fails but you get the point):
    /// ```should_panic
    /// use short_str::ShortStr;
    ///
    /// let inlined = ShortStr::from("unicode 🈁s");
    /// let slice   = unsafe { inlined.slice_unchecked(..inlined.len() - 2) };
    /// assert_eq!(slice, "unicode �");
    /// ```
    pub unsafe fn slice_unchecked(self, slice: impl RangeBounds<usize>) -> Self {
        let range = self.bounds_to_range(slice);

        // assumptions:
        // `slice` is correctly ordered (no end < start) and sized (no end > self.len())
        match self.variant() {
            // include these if statements here just cause its prettier :p
            // if the slice is zero length then its just the empty case
            _ if range.len() == 0 => Self::EMPTY,
            // if they are the same length then its a nop
            _ if self.len() == range.len() => self,
            // &str facades should be handled by &str, then handle &str as ShortStr in case its
            // short enough to inline
            Variant::Facade(str_ref) => Self::from_str(&str_ref[range]),
            // bit manipulate the inlined data
            Variant::Inlined(data) => {
                // if its a ShortStr we manipulate the bytes to the correct state
                // NOTE: may be worth to handle this in the eq instead as its the only place where it
                // matters currently, or create a function to handle process and use it where
                // necessary. Slicing is common so using decreasing the length would be optimal for
                // performance.
                // safety:
                // CoveringInt is ensured to have the same size as ShortStr/Str
                // turn into bit representation for bit manipulation
                // Ex: start = 1
                //     end   = 3
                //     int   = 0x03_EF_CD_AB
                let int = CoveringInt::from_ne_bytes(data);
                // get new length
                let len = range.len() as i8;
                let int = if len == 0 {
                    // Ex: len = 0x80
                    // set to -1 if data is zero length
                    let len = -1i8;
                    // don't bother using the actual data
                    // Ex: len = 0x00_00_00_80 (cast)
                    //     len = 0x80_00_00_00 (rotate)
                    //     int = len
                    (len as CoveringInt).rotate_right(8)
                } else {
                    // remove the length
                    // Ex: int  = 0x03_EF_CD_AB
                    //     mask = 0x00_FF_FF_FF
                    //     data = 0x00_EF_CD_AB
                    let data = int & DATA_MASK;
                    // mask the bytes to the left of slice.end (or right in integer representation)
                    // Ex: upper = 0x00_FF_FF_FF (mask)
                    //     upper = 0xFF_00_00_00 (lsh end = 3 bytes)
                    //     upper = 0x00_FF_FF_FF (invert)
                    let upper_data_mask = !(DATA_MASK /* or CoveringInt::MAX */ << range.end * 8);
                    // Ex: data = 0x00_EF_CD_AB
                    //     data = 0x00_00_CD_AB (mask)
                    let data = data & upper_data_mask;
                    // move over data between slice.start and slice.end to be at the start of data
                    // Ex: data = 0x00_00_CD_AB
                    //     data = 0x00_00_00_CD (rsh start = 1 bytes)
                    let data = data >> range.start * 8;

                    // meld back together
                    // Ex: data = 0x00_00_00_AB
                    //     len  = 0x01
                    //     len  = 0x00_00_00_01 (cast)
                    //     len  = 0x01_00_00_00 (rotate)
                    //     int  = 0x01_00_00_AB
                    data | (len as CoveringInt).rotate_right(8)
                };
                // turn back into correct data type
                // safety:
                // CoveringInt is garantueed to be equal size to ShortStr
                // Using the masks above we garantuee we only meddle with specific parts
                //
                unsafe { transmute::<CoveringInt, ShortStr>(int) }
            }
            _ => unreachable!(),
        }
    }

    /// Converts the range `bounds` to an actual range based on `self` as a context.
    /// The RangeBounds trait is implemented for all range operators (`..`, `..N`, etc) but any
    /// other value that does support the trait is also welcome.
    ///
    /// Does not perform any checks on the resulting range.
    ///
    /// # Examples
    /// These examples assume a little endian 64-bit architecture.
    /// ```rust,ignore
    /// use short_str::ShortStr;
    ///
    /// let string = ShortStr::from("hello, world");
    /// let range  = string.bounds_to_range(..);
    /// assert_eq!(range, 0..string.len());
    /// ```
    fn bounds_to_range(self, bounds: impl RangeBounds<usize>) -> Range<usize> {
        // If this isn't optimized away by monomorphism I'm going to shoot myself and the compiler
        let realized_start = match bounds.start_bound() {
            core::ops::Bound::Included(&x) => x,
            core::ops::Bound::Unbounded => 0,
            _ => unreachable!(),
        };

        let realized_end_exclusive = match bounds.end_bound() {
            core::ops::Bound::Included(&x) => x + 1,
            core::ops::Bound::Excluded(&x) => x,
            core::ops::Bound::Unbounded => self.len(),
        };

        realized_start..realized_end_exclusive
    }

    /// Performs the slicing operation using `slice` on `self`, ensuring logical consistencies
    /// between `slice` and `self`. See [`ShortStr::slice_unchecked`] for the unchecked variant.
    /// There are four cases:
    /// - `slice.len() == 0`: a special empty [`ShortStr`] is produced
    /// - `slice.len() == self.len()`: just produces self
    /// - `self.is_str()` (plain [`&str`]): perform slice using native [`&str`] slicing into [`ShortStr::from`]
    /// - `!self.is_str()` (inlined [`&str`]): bitbash the bytes into the correct sliced version.
    ///
    /// # Panics
    /// If `slice` is illogical (descending order or out of bounds values) the function panics.
    /// Otherwise, if `slice` splits graphemes, the operation only panics for the case of when its
    /// representing an actual [`&str`] (i.e. not inlined). See [`&str`] for more details
    /// (particularly on splitting graphemes).
    ///
    /// To be clear; splitting graphemes inside the inlined [`&str`] does currently not produce a
    /// panic.
    ///
    /// # Examples
    /// These examples assume a little endian 64-bit architecture.
    /// ```rust
    /// use short_str::ShortStr;
    ///
    /// let string = ShortStr::from("hello, world!");
    /// let slice  = string.slice(..5) ;
    /// assert_eq!(slice, "hello");
    /// ```
    ///
    /// Attempting to use an incorrect slice on a [`ShortStr`] results in a panic:
    /// ```should_panic
    /// use short_str::ShortStr;
    ///
    /// let not_inlined = ShortStr::from("hello, world! yet another inlined string");
    /// let slice       = not_inlined.slice(..not_inlined.len() + 1);
    /// ```
    pub fn slice(self, slice: impl RangeBounds<usize>) -> Self {
        let range = self.bounds_to_range(slice);

        assert!(
            range.start <= range.end,
            "expected slice on ShortStr to have {{start}} <= {{end}}"
        );

        assert!(
            range.end <= self.len(),
            "expected slice on ShortStr to have {{end}} < {{length}}"
        );

        // safety:
        // slice bounds have been verified to be correct above
        unsafe { self.slice_unchecked(range) }
    }
}

impl<'str_lt> From<&'str_lt str> for ShortStr<'str_lt> {
    #[inline(always)]
    fn from(value: &'str_lt str) -> Self {
        Self::from_str(value)
    }
}

impl Deref for ShortStr<'_> {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl PartialEq<ShortStr<'_>> for ShortStr<'_> {
    #[inline(always)]
    fn eq(&self, other: &ShortStr) -> bool {
        // by using an int type that covers all bytes the compiler can determine what
        // the optimal bit-size to use on instruction level (best case its actually e.g. 128-bit
        // cmp instruction)
        CoveringInt::from_ne_bytes(self.data) == CoveringInt::from_ne_bytes(other.data)
    }
}

impl PartialEq<&str> for ShortStr<'_> {
    #[inline(always)]
    fn eq(&self, other: &&str) -> bool {
        // compare as scalar values through PartialEq<ShortStr> for ShortStr
        *self == ShortStr::from_str(other)
    }
}

impl PartialEq<ShortStr<'_>> for &str {
    #[inline(always)]
    fn eq(&self, other: &ShortStr) -> bool {
        // reuse PartialEq<&str> for ShortStr
        other.eq(self)
    }
}
