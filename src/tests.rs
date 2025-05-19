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
