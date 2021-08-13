# pages

[![License](https://img.shields.io/crates/l/pages.svg)](https://github.com/irrustible/pages/blob/main/LICENSE)
[![Package](https://img.shields.io/crates/v/pages.svg)](https://crates.io/crates/pages)
[![Documentation](https://docs.rs/pages/badge.svg)](https://docs.rs/pages)

A dynamically-sized heap-backed data page. Comprises a user-chosen header and
data array packed into a single allocation.

## Usage

```rust 
use pages::Page;
// A really crappy replacement for Box<Option<usize>>
struct Maybe(Page::<bool, usize>);
impl Maybe {
    fn new() -> Self { Maybe(Page::new(false, 1)) }
    fn put(&mut self, value: usize) {
        *self.0.header_mut() = true; // occupied
        let item: *mut usize = self.0.data_mut()[0].as_mut_ptr();
        unsafe { item.write(value); }
    }
    fn get(&mut self) -> Option<usize> {
        if !(*self.0.header()) { return None; }
        let item: *mut usize = self.0.data_mut()[0].as_mut_ptr();
        *self.0.header_mut() = false; // free
        Some(unsafe { *item })
    }
}

fn main() {
    let mut maybe = Maybe::new();
    assert_eq!(maybe.get(), None);
    maybe.put(42);
    assert_eq!(maybe.get(), Some(42));
}
```

## Copyright and License

Copyright (c) 2021 James Laver, pages contributors.

[Licensed](LICENSE) under Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0),
with LLVM Exceptions (https://spdx.org/licenses/LLVM-exception.html).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.
