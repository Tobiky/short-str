use core::mem::transmute;

use crate::{ShStr, ShortStr, BYTE_SIZE};

mod assumptions {
    use crate::{CoveringInt, ShStr};

    #[test]
    /// Verify that layout is indeed ptr then len
    fn verify_layout() {
        let test = "test";
        let [_, len] = unsafe { core::mem::transmute::<&str, [usize; 2]>(test) };
        assert_eq!(len, 4);
    }

    #[test]
    /// Verify that size is indeed 2x usize
    fn verify_size() {
        const STR_SIZE: usize = size_of::<&str>();
        const DUSIZE_SIZE: usize = size_of::<[usize; 2]>();
        const SHSTR_SIZE: usize = size_of::<ShStr>();
        const COVER_SIZE: usize = size_of::<CoveringInt>();
        assert_eq!(
            STR_SIZE, DUSIZE_SIZE,
            "expected &str to have the same size as [usize; 2] ({} vs. {})",
            STR_SIZE, DUSIZE_SIZE
        );
        assert_eq!(
            SHSTR_SIZE, DUSIZE_SIZE,
            "expected ShortStr/ShStr to have the same size as &str and [usize; 2] ({} vs. {})",
            SHSTR_SIZE, DUSIZE_SIZE
        );
        assert_eq!(
            COVER_SIZE,
            SHSTR_SIZE,
            "expected CoveringInt ({}) to have the same size as ShortStr/ShStr ({} vs. {})",
            core::any::type_name::<CoveringInt>(),
            COVER_SIZE,
            SHSTR_SIZE
        )
    }
}

macro_rules! str_assert {
    ($condition:expr, $short_str:ident, $message:literal $(, $($x:expr),+)?) => {
        assert!(
            $condition,
            concat!(
                $message,
                "\ndata:   {__data_slice__:?}",
                "\n        ({__data_slice_length__} bytes)",
                "\nmarker:  {__data_marker__:08b}",
                "\n        ({__data_marker__})",
            ),
            $($($x,)+)?
            __data_slice__ = &$short_str.data[..BYTE_SIZE - 1],
            __data_slice_length__ = BYTE_SIZE - 1,
            __data_marker__ = $short_str.length_marker(),
        );
    };
}
macro_rules! str_assert_eq {
    ($a:ident, $b:ident, $message:literal $(, $($x:expr),+)?) => {
        str_assert_eq!($a, $b, $a, $b, $message $(, $($x,)+)?);
    };
    ($a:expr, $b:expr, $shstr:ident, $message:literal $(, $($x:expr),+)?) => {
        assert_eq!(
            $a,
            $b,
            concat!(
                $message,
                "\ndata:   {__data_slice__:?}",
                "\n        ({__data_slice_length__} bytes)",
                "\nmarker:  {__data_marker__:08b}",
                "\n        ({__data_marker__})",
            ),
            $($($x,)+)?
            __data_slice__ = &$short_str.data[..BYTE_SIZE - 1],
            __data_slice_length__ = BYTE_SIZE - 1,
            __data_marker__ = $short_str.length_marker(),
        );
    };
    ($a:expr, $b:expr, $a_shstr:ident, $b_shstr:ident, $message:literal $(, $($x:tt),+)?) => {
        assert_eq!(
            $a,
            $b,
            concat!(
                $message,
                "\ndata A:   {__data_slice_a__:?}",
                "\n        ({__data_slice_length__} bytes)",
                "\nmarker A:  {__data_marker_a__:08b}",
                "\n        ({__data_marker_a__})",
                "\n",
                "\ndata B:   {__data_slice_b__:?}",
                "\n        ({__data_slice_length__} bytes)",
                "\nmarker B:  {__data_marker_b__:08b}",
                "\n        ({__data_marker_b__})",
            ),
            $($($x,)+)?
            __data_slice_length__ = BYTE_SIZE - 1,
            __data_slice_a__ = &$a_shstr.data[..BYTE_SIZE - 1],
            __data_marker_a__ = $a_shstr.length_marker(),
            __data_slice_b__ = &$b_shstr.data[..BYTE_SIZE - 1],
            __data_marker_b__ = $b_shstr.length_marker(),
        );
    };
}

#[test]
fn long_str_facade() {
    let a = "1 2 3 4 5 6 7 8 9 10";
    let short = ShStr::from(a);
    assert!(
        short.is_str(),
        "expected long &str (length: {}) to become a ShortStr facade",
        a.len()
    );
    assert_eq!(a, short, "expected &str and its facade ShortStr to be equal");
    #[rustfmt::skip]
    assert_eq!(
        unsafe { transmute::<&str, [u8; size_of::<&str>()]>(a) },
        unsafe { transmute::<ShortStr, [u8; size_of::<&str>()]>(short) }
    );
}

#[test]
fn short_str_inline() {
    let a = "hi";
    let short = ShStr::from(a);
    assert!(
        !short.is_str(),
        "expected short &str (length: {}) to become inlined in ShortStr",
        a.len()
    );
    assert_eq!(
        short.len(),
        a.len(),
        "expected inlined &str (ShortStr) to have same length as original ({} vs. {})",
        short.len(),
        a.len(),
    );
    assert_eq!(a, short, "expected inlined &str (ShortStr) to be equal to its original")
}

#[test]
fn empty_str_inline() {
    let a = "";
    let short = ShStr::from(a);
    assert!(!short.is_str(), "expected empty &str to become inlined in ShortStr");
    assert!(
        short.is_empty_inlined(),
        "expected empty &str to become empty ShortStr, but got one with length {}",
        short.len()
    );
    assert_eq!(
        short,
        ShStr::EMPTY,
        "expected converted empty &str to be equal to constant empty ShortStr"
    );
}

#[test]
fn inline_str_upper_slice_length() {
    let a = ShStr::from("abc");
    let b = a[1..];
    assert_eq!(b.len(), a[1..].len());
}
