#![no_std]
use core::{
    convert::Infallible,
    fmt::{Debug, Display},
    marker::PhantomData,
    mem::transmute,
    ops::Deref,
    ptr::copy_nonoverlapping,
};

#[cfg(test)]
mod tests;

#[cfg(debug_assertions)]
const _: () = const {
    use const_panic::concat_assert;

    const REPO_URL: &'static str = "https://github.com/Tobiky/short-str";

    // Little Endian
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
        f.write_str(self)
    }
}

impl Display for ShortStr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self)
    }
}

enum Variant<'str_lt> {
    Inlined([u8; BYTE_SIZE]),
    Facade(&'str_lt str),
    Empty,
}

impl Variant<'_> {
    #[inline(always)]
    const fn from_short_str(value: ShortStr<'_>) -> Self {
        if value.is_str() {
            // Safety:
            // is_str_ref garantuees that `value` is indeed a &str
            let str_ref = unsafe { transmute(value) };
            Variant::Facade(str_ref)
        } else if value.is_empty_inlined() {
            Variant::Empty
        } else {
            Variant::Inlined(value.data)
        }
    }
}

impl From<ShortStr<'_>> for Variant<'_> {
    #[inline(always)]
    fn from(value: ShortStr<'_>) -> Self {
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
    pub const EMPTY: ShortStr<'str_lt> = const {
        let mut data = [0; BYTE_SIZE];
        data[BYTE_SIZE - 1] = -1i8 as u8;
        ShortStr { data, _lt: PhantomData }
    };

    #[inline(always)]
    const fn variant(self) -> Variant<'str_lt> {
        Variant::from_short_str(self)
    }

    #[inline(always)]
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
    const fn is_empty_inlined(self) -> bool {
        (self.length_marker() as i8).is_negative()
    }

    #[inline(always)]
    pub const fn is_str(self) -> bool {
        self.length_marker() == 0
    }

    #[inline(always)]
    pub const fn len(self) -> usize {
        match self.variant() {
            Variant::Inlined(data) => data[BYTE_SIZE - 1] as usize,
            Variant::Facade(str_ref) => str_ref.len(),
            Variant::Empty => 0,
        }
    }

    #[inline(always)]
    // unsafety:
    // while the transmutation is not UB for any &str, the representation might have differed if
    // ShortStr::from had been used (e.g. <= 15 bytes long str, on 64-bit platforms) which would
    // cause the Eq operation to fail even if they are the same
    // Example: ShortStr::from_str_unchecked("test") != ShortStr::from("test")
    pub const unsafe fn from_str_unchecked(other: &str) -> Self {
        // safety:
        // see ShortStr::length_marker(self)
        // any &str is a valid instance of ShortStr due to the nature of the struct
        unsafe { transmute(other) }
    }

    #[inline(always)]
    pub const fn from_str<'a>(value: &'a str) -> Self {
        // safety:
        // short_str is not &str, in which case its a ShortStr, and can thus be used as normal
        // short_str is a &str, in which case ShortStr is just handled like a facade
        let short_str = unsafe { Self::from_str_unchecked(value) };
        match short_str.variant() {
            Variant::Facade(_) if value.len() < INLINE_BYTE_SIZE => {
                // if it can fit into an inline str then convert
                let mut data = [0; BYTE_SIZE];
                // safety:
                // this is just copy_from_slice but that as const isn't stable yet
                // - not same locations
                // - amount of bytes to copy is garantueed < INLINE_BYTE_SIZE by condition
                unsafe {
                    copy_nonoverlapping(value.as_ptr(), data.as_mut_ptr(), value.len());
                }
                data[BYTE_SIZE - 1] = value.len() as u8;
                ShortStr { data, _lt: PhantomData }
            }
            // It's already a proper ShortStr
            // A: an inlined &str
            // B: a &str facade
            Variant::Facade(_) | Variant::Inlined(_) => short_str,
            // Special empty case
            Variant::Empty => ShortStr::EMPTY,
        }
    }

    #[inline(always)]
    pub const fn as_str(self) -> &'str_lt str {
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
}

impl From<&str> for ShortStr<'_> {
    #[inline(always)]
    fn from(value: &str) -> Self {
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
        // in case from_le_bytes has overhead
        // safety:
        // we are only comparing bytes and conflicted size differences are disallowed by transmute
        // unsafe { transmute::<ShortStr, CoveringInt>(*self) == transmute::<ShortStr, CoveringInt>(*other) }
        CoveringInt::from_le_bytes(self.data) == CoveringInt::from_le_bytes(other.data)
    }
}

impl PartialEq<&str> for ShortStr<'_> {
    #[inline(always)]
    fn eq(&self, other: &&str) -> bool {
        // compare as scalar values through PartialEq<ShortStr>
        *self == ShortStr::from_str(other)
    }
}

impl PartialEq<ShortStr<'_>> for &str {
    #[inline(always)]
    fn eq(&self, other: &ShortStr) -> bool {
        other.eq(self)
    }
}
