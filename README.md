# ShortStr
Another inlined string library, really? Yepp.

`ShortStr` is meant to be a full-stop replacement for `&str`, i.e. an immutable slice of character data, with the additional service of inlining data that can fit into the fat pointer.

## Safety
While many functions are marked as safe because of their realistic viability, the usage isn't completely garantueed. `ShortStr` uses the MSB in the length part of a `&str`'s fat pointer since a `&str` is unlikely to be longer than 2^56 on 64-bit machines or 2^24 on 32-bit machines, for example. However, unlikely doesn't mean impossible. If you forsee that it might become an issue you should not use this crate in its current form.

## MSRV
Rust `1.85.1` or above is required.

## Features & Goals
Features, guarantuees, and goals are pretty much synonymous for this crate so they will be listed here simply as a TODO list of sorts.

- Size 
    - [x] Equal size to `&str`
    - [x] Little endian size optimization (Use MSG of length portion, statically asserted)
    - [ ] Allow other `&str` size than layouts `(usize, usize)`
    - [ ] No endian optimization feature (Only use pointer bytes as storage, forced on Big-endian)
    - [ ] NPO (Possible if niches become stable, may become a seperate unstable crate)
    - [ ] Struct alignment features
- Safety
    - [x] Assumptions are asserted at compile-time
    - [x] Immutable data
    - [ ] Formally verified
    - [ ] Miri checked
- Usage/Ergonomics
    - [ ] Inline slice on MSB for Little endian inlined variant
        - [ ] Use length MSB to contain range of valid bytes
        - [ ] Feature set to set when normalization (moving internal bytes) occurs
    - [ ] Slicing
        - [x] Dedicated slicing functions
        - [ ] `Index` trait implementation
        - [ ] Identical semantics (currently not panicking on splitting graphemes)
    - [x] `ShortStr` and `&str` comparison
        - [x] Scalar comparison between `ShortStr`
        - [x] Comparison on `&str` via cast (Copies on inlinable `&str`)
    - [x] `Deref` to `str`
    - [ ] Inlined optimized `Hash` impl
    - [ ] Inlined optimized `Ord` impl
