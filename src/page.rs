use crate::*;
use core::fmt;
use core::mem::{MaybeUninit, forget};

/// An owned, heap-backed, dynamically-sized data page comprising a user-chosen
/// header and data array packed into a single allocation. It is an owned object and
/// the internal representation is a [`NonNull`].
///
/// ## Example
///
/// ```
/// use pages::Page;
/// use core::mem::MaybeUninit;
/// // A really crappy replacement for Box<Option<usize>>
/// struct Maybe(Page::<bool, usize>);
/// impl Maybe {
///     fn new() -> Self { Maybe(Page::new(false, 1)) }
///     fn put(&mut self, value: usize) {
///         *self.0.header_mut() = true; // occupied
///         unsafe { self.0.data().write(MaybeUninit::new(value)) };
///     }
///     fn get(&mut self) -> Option<usize> {
///         if !(*self.0.header()) { return None; }
///         *self.0.header_mut() = false; // free
///         Some(unsafe { self.0.data().read().assume_init() })
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
/// Data is exposed as a [`MaybeUninit`] pointer for maximum flexibility.
/// Unfortunately this means we're unable to automatically drop the data
/// for you in our destructor. You could cause a memory leak if you don't.
#[repr(transparent)]
pub struct Page<H, T>(PageRef<H, T>);

impl<H, T> Page<H, T> {
    /// Creates a new [`Page`] on the heap with the provided header and capacity for
    /// `items` items.
    ///
    /// ## Notes
    ///
    /// Will panic if items is 0 or the header plus padding is extremely large
    /// (u32::MAX - 8 bytes)
    pub fn new(header: H, items: u32) -> Self { Page(PageRef::new(header, items)) }

    /// The capacity of this page's data array.
    #[inline(always)]
    pub fn capacity(&self) -> u32 { unsafe { self.0.capacity() } }

    /// Access to this page's header by reference.
    #[inline(always)]
    pub fn header(&self) -> &H { unsafe { self.0.header() } }

    /// Access to this page's header by mut reference.
    #[inline(always)]
    pub fn header_mut(&mut self) -> &mut H { unsafe { self.0.header_mut() } }

    /// Access to the start of the data array as a mut pointer.
    #[inline(always)]
    pub fn data(&self) -> *mut MaybeUninit<T> { unsafe { self.0.data() } }

    /// Returns the [`PageLayout`] describing the memory layout of this [`Page`]
    #[inline(always)]
    pub fn layout(&self) -> PageLayout<H, T> { PageLayout::with_capacity(self.0.desc().items) }

    /// Creates a new [`Page`] from a pointer to uninitialised memory, a header and
    /// a [`PageLayout`].
    ///
    /// ## Safety
    ///
    /// You must ensure:
    ///
    /// * The pointer was allocated according to the provided [`PageLayout`].
    ///   * Synchronise all reads and writes to 
    ///   * Suppress the destructor of all but one of them (e.g. by wrapping in [`ManuallyDrop`]).
    /// * If the pointer did not originate from the global allocator, you must
    ///   suppress the destructor (e.g. by wrapping in [`ManuallyDrop`]).
    #[inline(always)]
    pub unsafe fn from_uninit(raw_ptr: *mut u8, header: H, layout: PageLayout<H, T>) -> Self {
        Page(PageRef::from_uninit(raw_ptr, header, layout))
    }

    /// Creates an owned [`Page`] from a [`PageRef`].
    ///
    /// ## Example
    ///
    /// ```
    /// use pages::Page;
    /// let page = Page::<bool, usize>::new(false, 1);
    /// let page_ref = page.to_ref();
    /// let page = unsafe { Page::<bool, usize>::from_ref(page_ref) };
    /// ```
    ///
    /// ## Safety
    ///
    /// You must only have one live [`Page`] per page.
    pub unsafe fn from_ref(page_ref: PageRef<H, T>) -> Self { Page(page_ref) }

    /// Converts this [`Page`] to a PageRef, a mutable pointer structure,
    /// effectively leaking it.
    #[inline(always)]
    pub fn to_ref(self) -> PageRef<H, T> {
        let r = self.0;
        forget(self); // Disable our destructor.
        r
    }
}

unsafe impl<H: Send, T: Send> Send for Page<H, T> {}
unsafe impl<H: Sync, T: Sync> Sync for Page<H, T> {}

impl<H, T> fmt::Debug for Page<H, T> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Page[{}]", self.capacity())
    }
}

impl<H, T> Drop for Page<H, T> {
    #[inline(always)] fn drop(&mut self) { unsafe { PageRef::drop(self.0) } }
}
