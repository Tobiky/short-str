#![no_std]
use core::{fmt::Debug, mem::transmute, ops::Deref, ptr::copy_nonoverlapping};

#[cfg(debug_assertions)]
use const_panic::concat_assert;

#[cfg(debug_assertions)]
const REPO_URL: &'static str = "https://github.com/Tobiky/short-str";

#[cfg(test)]
mod tests;

const PTR_SIZE: usize = size_of::<usize>();
const LEN_SIZE: usize = size_of::<usize>();
const BYTE_SIZE: usize = PTR_SIZE + LEN_SIZE;
const INLINE_BYTE_SIZE: usize = BYTE_SIZE - 1;

#[inline(always)]
const fn assert_little_endian() {
    concat_assert!(
        unsafe { transmute::<&str, [u8; BYTE_SIZE]>("test") }[PTR_SIZE] as usize == "test".len(),
        "big endian architecture is currently unsupported for ShortStr's",
    );
}

#[inline(always)]
const fn assert_str_size() {
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
}

#[cfg(debug_assertions)]
#[inline(always)]
const fn assert_assumptions() {
    assert_str_size();
    assert_little_endian();
}

// layout of &str is ptr, len
// see `verify_layout` test
#[derive(Clone, Copy, Eq, PartialOrd, Ord)]
pub struct ShortStr {
    data: [u8; BYTE_SIZE],
}
pub type ShStr = ShortStr;

impl Debug for ShortStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self)
    }
}

/// inline str zero length flag
// useful only with more flags
// 2 * size_of::<usize>() - 1 = how many bytes that can be stored
// ilog2 = how many bits to represent size
// disregard - 1 to ensure no size bits are encroached (ilog2 rounds down)
// ilog2(byte size + 1) = how many bits to move over since 1 is already at first place
// const MASK_INLINE_ZERO_LEN: usize = 1 << usize::ilog2(BYTE_SIZE);

impl ShortStr {
    pub const EMPTY: ShortStr = const {
        let mut data = [0; BYTE_SIZE];
        data[BYTE_SIZE - 1] = -1i8 as u8;
        ShortStr { data }
    };

    #[inline(always)]
    pub const fn is_zero_len(self) -> bool {
        // self.data[BYTE_SIZE - 1] & (MASK_INLINE_ZERO_LEN as u8) != 0
        // just use signed bit as flag, no other flags atm
        (self.data[BYTE_SIZE - 1] as i8).is_negative()
    }

    #[inline(always)]
    pub const fn is_str_ref(self) -> bool {
        self.data[BYTE_SIZE - 1] == 0
    }

    #[inline(always)]
    pub const fn len(self) -> usize {
        #[cfg(debug_assertions)]
        assert_assumptions();
        // assumptions: little endian

        // ------------------------------------------------------------------------------
        // in the case of little endian the last byte is unrealistic to be set
        // as that would require more than e.g. 2^58 "directly" addressed bytes of memory
        // and therefore can be used for the inline str mode size, and as a marker.
        // ------------------------------------------------------------------------------

        let inline_size = self.data[BYTE_SIZE - 1];

        // the str is inline with zero length flag set
        if self.is_zero_len() {
            0
        }
        // the str is inline with non-zero length
        else if inline_size != 0 {
            inline_size as usize
        }
        // the length byte is zero and no zero length flag is set; regular &str
        else {
            // safety:
            // case A: ShStr is inlined str
            // proof:  That case is handled above
            // case B: ShStr is pointer and length
            // proof:  Second half is length, as per tests::assumptions::*
            let [_, len]: [usize; 2] = unsafe { transmute(self) };
            len
        }
    }

    #[inline(always)]
    // unsafety:
    // while the transmutation is not UB for any &str, the representation might have differed if
    // ShortStr::from had been used (e.g. <= 15 bytes long str, on 64-bit platforms) which would
    // cause the Eq operation to fail even if they are the same
    // Example: ShortStr::from_str_unchecked("test") != ShortStr::from("test")
    pub const unsafe fn from_str_unchecked(other: &str) -> Self {
        #[cfg(debug_assertions)]
        assert_assumptions();
        // safety:
        // see ShortStr::len(self)
        // brief: last byte is 0 for regular &str and is used for &str detection so this
        // transmutation will just go around the potential inline str savings in the worst case
        unsafe { transmute(other) }
    }
}

impl<'a> From<&'a str> for ShortStr {
    fn from(value: &'a str) -> Self {
        #[cfg(debug_assertions)]
        assert_assumptions();
        // its already a ShortStr
        // safety:
        // see ShortStr::len(self)
        // brief: ShortStr is either fully a &str or only uses last byte to determine storage which
        // is unrealistic to be used anyway
        let short_str = unsafe { transmute::<&str, ShortStr>(value) };
        if !short_str.is_str_ref() {
            short_str
        }
        // zero length special case
        else if value.len() == 0 {
            ShortStr::EMPTY
        }
        // if it can fit into an inline str then convert
        else if value.len() <= INLINE_BYTE_SIZE {
            let mut data = [0; BYTE_SIZE];
            // safety:
            // value and data will never be overlapping and we cap the amount of bytes to copy by
            // using len() which is already garantueed to fit into the data buffer as per the
            // conditional
            unsafe {
                copy_nonoverlapping(value.as_ptr(), data.as_mut_ptr(), value.len());
            }
            data[BYTE_SIZE - 1] = value.len() as u8;
            ShortStr { data }
        }
        // otherwise just leave alone (ShortStr facade for &str)
        else {
            short_str
        }
    }
}

impl Deref for ShortStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        #[cfg(debug_assertions)]
        assert_assumptions();
        // assumptions: little endian
        if self.is_str_ref() {
            // safety:
            // see ShortStr::len(self)
            // brief: last byte is unrealistic to be set, used for inline str size and flags 0
            // being no flags or size and thus a regular &str
            unsafe { transmute(*self) }
        } else {
            // safety:
            // the ShortStr is an inline str, starting at the same place as data and with length
            // we get from len
            unsafe {
                let slice = core::slice::from_raw_parts(self.data.as_ptr(), self.len());
                core::str::from_utf8_unchecked(slice)
            }
        }
    }
}

#[cfg(target_pointer_width = "64")]
type CoveringInt = u128;
#[cfg(target_pointer_width = "32")]
type CoveringInt = u64;
#[cfg(target_pointer_width = "16")]
type CoveringInt = u32;

impl PartialEq<ShortStr> for ShortStr {
    fn eq(&self, other: &ShortStr) -> bool {
        debug_assert_eq!(
            size_of::<CoveringInt>(),
            size_of::<ShortStr>(),
            "expected {} to match byte size with ShortStr/ShStr ({} vs. {}), please file an issue at {}",
            core::any::type_name::<CoveringInt>(),
            size_of::<CoveringInt>(),
            size_of::<ShortStr>(),
            REPO_URL
        );
        // by using an int type that covers all bytes the compiler can determine what
        // the optimal bit-size to use on instruction level (best case its actually e.g. 128-bit
        // cmp instruction)
        // safety:
        // we are only comparing bytes and conflicted size differences are disallowed by transmute
        unsafe { transmute::<ShortStr, CoveringInt>(*self) == transmute::<ShortStr, CoveringInt>(*other) }
    }
}

impl PartialEq<&str> for ShortStr {
    fn eq(&self, other: &&str) -> bool {
        #[cfg(debug_assertions)]
        assert_assumptions();
        // safety:
        // see ShortStr::len(self)
        // brief: ShortStr is either fully a &str or only uses last byte to determine storage which
        // is unrealistic to be used anyway
        let other_str_ref = unsafe { transmute::<&str, ShortStr>(other) }.is_str_ref();
        let other = if other_str_ref {
            // other is actually a &str and not accidentally through Deref, coerce into ShortStr
            ShortStr::from(*other)
        } else {
            // other is not actually a &str but a ShortStr, probably from deref, so we just transform
            unsafe { transmute::<&str, ShortStr>(other) }
        };

        // compare as scalar values through PartialEq<ShortStr>
        *self == other
    }
}

impl PartialEq<ShortStr> for &str {
    fn eq(&self, other: &ShortStr) -> bool {
        other.eq(self)
    }
}
