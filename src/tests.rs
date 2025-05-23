use core::mem::transmute;

use crate::{ShStr, ShortStr};

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

#[test]
fn long_str_facade() {
    let a = "1 2 3 4 5 6 7 8 9 10";
    let short = ShStr::from(a);
    assert!(
        short.is_str_ref(),
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
        !short.is_str_ref(),
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
    assert!(!short.is_str_ref(), "expected empty &str to become inlined in ShortStr");
    assert!(
        short.is_zero_len(),
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
