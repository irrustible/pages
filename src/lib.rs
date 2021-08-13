//! A dynamically-sized heap-backed data page. Comprises a user-chosen header and
//! data array packed into a single allocation.
//!
//! ## Example
//!
//! ```
//! use pages::Page;
//! // A really crappy replacement for Box<Option<usize>>
//! struct Maybe(Page::<bool, usize>);
//! impl Maybe {
//!     fn new() -> Self { Maybe(Page::new(false, 1)) }
//!     fn put(&mut self, value: usize) {
//!         *self.0.header_mut() = true; // occupied
//!         let item: *mut usize = self.0.data_mut()[0].as_mut_ptr();
//!         unsafe { item.write(value); }
//!     }
//!     fn get(&mut self) -> Option<usize> {
//!         if !(*self.0.header()) { return None; }
//!         let item: *mut usize = self.0.data_mut()[0].as_mut_ptr();
//!         *self.0.header_mut() = false; // free
//!         Some(unsafe { *item })
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

use alloc::alloc::{Layout, alloc, dealloc};
use core::convert::TryInto;
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::{NonNull, drop_in_place};
use core::slice;

/// A dynamically-sized heap-backed data page. Comprises a user-chosen header and
/// data array packed into a single allocation. It is an owned object and the
/// internal representation is a [`NonNull`].
///
/// ## Example
///
/// ```
/// use pages::Page;
/// // A really crappy replacement for Box<Option<usize>>
/// struct Maybe(Page::<bool, usize>);
/// impl Maybe {
///     fn new() -> Self { Maybe(Page::new(false, 1)) }
///     fn put(&mut self, value: usize) {
///         *self.0.header_mut() = true; // occupied
///         let item: *mut usize = self.0.data_mut()[0].as_mut_ptr();
///         unsafe { item.write(value); }
///     }
///     fn get(&mut self) -> Option<usize> {
///         if !(*self.0.header()) { return None; }
///         let item: *mut usize = self.0.data_mut()[0].as_mut_ptr();
///         *self.0.header_mut() = false; // free
///         Some(unsafe { *item })
///     }
/// }
///
/// let mut maybe = Maybe::new();
/// assert_eq!(maybe.get(), None);
/// maybe.put(42);
/// assert_eq!(maybe.get(), Some(42));
/// ```
///
/// ## Notes
///
/// Data is exposed as a [`MaybeUninit`] slice for maximum flexibility.
/// Unfortunately this means we're unable to automatically drop the data
/// for you in our destructor. You could cause a memory leak if you don't.
#[repr(transparent)]
pub struct Page<H, T> {
    inner: NonNull<u8>,
    _phantom: PhantomData<(H,T)>,
}

impl<H, T> Page<H, T> {
    /// Creates a new [`Page`] with the capacity for the provided number of items on
    /// the heap and a header.
    ///
    /// ## Notes
    ///
    /// Will panic if the header plus padding is extremely large (around 2^32 bytes)
    pub fn new(header: H, items: u32) -> Self {
        // Get a layout and prepare a book describing it. We do this before
        // allocation in case it fails.
        let layout = PageLayout::for_capacity::<H, T>(items);
        let book = Book { items, data: layout.data.try_into().unwrap() };
        // Allocate and prepare pointers to what we need to initialise. All safe if
        // you trust the layout is correct, which is presumed throughout.
        let raw_ptr = unsafe { alloc(layout.layout) };
        let header_ptr = raw_ptr.cast::<H>();
        let book_ptr = unsafe { raw_ptr.add(layout.book) }.cast::<Book>();
        // Now we need to do that initialisation.
        let inner = unsafe {
            header_ptr.write(header);
            book_ptr.write(book);
            NonNull::new_unchecked(header_ptr.cast())
        };
        Page { inner, _phantom: PhantomData }
    }

    /// The capacity of our data array.
    #[inline(always)]
    pub fn capacity(&self) -> u32 { self.book().items }

    /// Access to the header by reference.
    #[inline(always)]
    pub fn header(&self) -> &H { unsafe { &*self.inner.as_ptr().cast() } }

    /// Access to the header by mut reference.
    #[inline(always)]
    pub fn header_mut(&mut self) -> &mut H { unsafe { &mut *self.inner.as_ptr().cast() } }

    /// Access to the raw data as a slice.
    #[inline(always)]
    pub fn data(&self) -> &[MaybeUninit<T>] {
        unsafe { slice::from_raw_parts(self.data_ptr(), self.book().items as usize) }
    }

    /// Access to the raw data as a mut slice.
    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe { slice::from_raw_parts_mut(self.data_ptr(), self.book().items as usize) }
    }

    #[inline(always)]
    /// Returns a copy of our Book.
    fn book(&self) -> Book {
        let raw = self.inner.as_ptr();
        let book = PageLayout::static_prefix::<H>().1;
        unsafe { *raw.add(book).cast()  }
    }

    #[inline(always)]
    /// Returns a pointer to the start of the data section.
    fn data_ptr(&self) -> *mut MaybeUninit<T> {
        let raw = self.inner.as_ptr();
        unsafe { raw.add(self.book().data as usize) }.cast()
    }
}

unsafe impl<H: Send, T: Send> Send for Page<H, T> {}
unsafe impl<H: Sync, T: Sync> Sync for Page<H, T> {}

impl<H, T> fmt::Debug for Page<H, T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Page[{}]", self.capacity())
    }
}

impl<H, T> Drop for Page<H, T> {
    // Safety: we have exclusive access.
    fn drop(&mut self) {
        // Drop the header. 
        let raw = self.inner.as_ptr();
        unsafe { drop_in_place(raw.cast::<H>()); }
        // Deallocate the memory.
        let layout = PageLayout::for_capacity::<H, T>(self.book().items);
        unsafe { dealloc(raw, layout.layout); }
    }
}

/// Just the numerics from a page layout. Very compact (64 bits).
#[derive(Clone,Copy)]
struct Book {
    /// Our item capacity
    items: u32,
    /// The offset from the start of a page to the data array.
    data:  u32,
}

/// Describes the memory layout for a Page. This unassuming struct with no unsafe in
/// sight is actually the most important thing for safety.
#[derive(Clone,Copy)]
struct PageLayout {
    /// Offset from the start of a page where the book is located.
    book:   usize,
    /// Offset from the start of a page where the data is located.
    data:   usize,
    /// A layout suitable for allocation/deallocation of a page.
    layout: Layout,
}

impl PageLayout {
    /// Creates a `PageLayout` with the given item capacity.
    #[inline(always)]
    fn for_capacity<H, T>(items: u32) -> Self {
        let (prefix, book) = Self::static_prefix::<H>();
        let array = Layout::array::<T>(items as usize).unwrap();
        let (layout, data) = prefix.extend(array).unwrap();
        let layout = layout.pad_to_align();
        Self { book, data, layout }
    }

    /// Info about the statically sized prefix of the page.
    /// The usize returned is book's offset.
    #[inline(always)]
    fn static_prefix<H>() -> (Layout, usize) {
        let book = Layout::new::<Book>();
        let header = Layout::new::<H>();
        header.extend(book).unwrap()
    }
}
