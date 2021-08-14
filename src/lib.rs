//! An owned, heap-backed, dynamically-sized data page comprising a user-chosen
//! header and data array packed into a single allocation. It is an owned object and
//! the internal representation is a [`NonNull`].
//!
//! ## Example
//!
//! ```
//! use pages::Page;
//! use core::mem::MaybeUninit;
//! // A really crappy replacement for Box<Option<usize>>
//! struct Maybe(Page::<bool, usize>);
//! impl Maybe {
//!     fn new() -> Self { Maybe(Page::new(false, 1)) }
//!     fn put(&mut self, value: usize) {
//!         *self.0.header_mut() = true; // occupied
//!         unsafe { self.0.data().write(MaybeUninit::new(value)) };
//!     }
//!     fn get(&mut self) -> Option<usize> {
//!         if !(*self.0.header()) { return None; }
//!         *self.0.header_mut() = false; // free
//!         Some(unsafe { self.0.data().read().assume_init() })
//!     }
//! }
//!
//! let mut maybe = Maybe::new();
//! assert_eq!(maybe.get(), None);
//! maybe.put(42);
//! assert_eq!(maybe.get(), Some(42));
//! ```
#![no_std]
extern crate alloc;

mod layout;
pub use layout::PageLayout;

mod page;
pub use page::*;

mod page_ref;
pub use page_ref::*;
