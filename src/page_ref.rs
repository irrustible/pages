use crate::layout::*;
use alloc::alloc::{alloc, dealloc};
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::{NonNull, drop_in_place};

/// A mutable pointer to a dynamically-sized heap-backed data page
/// comprising a user-chosen header and data array packed into a
/// single allocation. The internal representation is a [`NonNull`].
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
pub struct PageRef<H, T> {
    inner: NonNull<u8>,
    _phantom: PhantomData<(H,T)>,
}

impl<H, T> Eq for PageRef<H, T> {}

impl<H, T> PartialEq  for PageRef<H, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool { self.inner == other.inner }
}

impl<H, T> Clone for PageRef<H, T> {
    #[inline(always)]
    fn clone(&self) -> Self { PageRef { inner: self.inner, _phantom: self._phantom } }
}

impl<H, T> Copy for PageRef<H, T> {}

impl<H, T> PageRef<H, T> {
    /// Creates a new [`PageRef`] on the heap with the provided header and capacity for
    /// `items` items.
    ///
    /// ## Notes
    ///
    /// Will panic if items is 0 or the header plus padding is extremely large
    /// (u32::MAX - 8 bytes)
    #[inline(always)]
    pub fn new(header: H, items: u32) -> Self {
        // In order to safely allocate and use the memory, we create a `PageLayout`,
        // which encapsulates all the knowledge we need. The safety of everything
        // hinges on the correctness of the `PageLayout`.
        let layout = PageLayout::<H, T>::with_capacity(items);
        let ptr = unsafe { alloc(layout.layout()) }; // Allocate.
        unsafe { Self::from_uninit(ptr, header, layout) }   // Initialise.
    }

    /// The capacity of this page's data array.
    ///
    /// ## Safety
    ///
    /// You must synchronise all reads and writes.
    #[inline(always)]
    pub unsafe fn capacity(self) -> u32 { self.desc().items }

    /// Access to this page's header by reference.
    ///
    /// ## Safety
    ///
    /// You must synchronise all reads and writes.
    #[inline(always)]
    pub unsafe fn header(&self) -> &H { &(*self.page_header()).header }

    /// Access to this page's header by mut reference.
    ///
    /// ## Safety
    ///
    /// You must synchronise all reads and writes.
    #[inline(always)]
    pub unsafe fn header_mut(&mut self) -> &mut H { &mut (*self.page_header()).header }

    /// Access to the start of the data array for this page as a mut pointer.
    ///
    /// ## Safety
    ///
    /// You must synchronise all reads and writes.
    #[inline(always)]
    pub unsafe fn data(self) -> *mut MaybeUninit<T> {
        let raw = self.inner.as_ptr();
        let offset = (*raw.cast::<PageHeader<H>>()).desc.data;
        raw.add(offset as usize).cast()
    }

    /// Returns this page's layout information as a [`PageLayout`].
    ///
    /// ## Safety
    ///
    /// You must synchronise all reads and writes.
    #[inline(always)]
    pub unsafe fn layout(self) -> PageLayout<H, T> { PageLayout::with_capacity(self.desc().items) }

    /// Drops the page pointed to by the provided [`PageRef`]
    ///
    /// ## Safety
    ///
    /// You must no longer access this page via other [`PageRef`]s.
    pub unsafe fn drop(page: Self) {
        let raw = page.inner.as_ptr();
        drop_in_place(raw.cast::<PageHeader<H>>());
        let layout = PageLayout::<H, T>::with_capacity(page.desc().items);
        dealloc(raw, layout.layout());
    }

    /// Creates a new [`PageRef`] from a pointer to uninitialised memory, a header and
    /// a [`PageLayout`].
    ///
    /// ## Safety
    ///
    /// The pointer must have been allocated according to the provided [`PageLayout`].
    #[inline(always)]
    pub unsafe fn from_uninit(raw_ptr: *mut u8, header: H, layout: PageLayout<H, T>) -> Self {
        // Prepare pointers to what we need to initialise. All safe if
        // you trust the layout is correct, which is presumed throughout.
        let header_ptr = raw_ptr.cast::<PageHeader<H>>();
        let header = PageHeader { header, desc: PageDesc::from(layout) };
        // Now we need to do that initialisation.
        header_ptr.write(header);
        let inner = NonNull::new_unchecked(header_ptr.cast());
        PageRef { inner, _phantom: PhantomData }
    }

    #[inline(always)]
    /// Returns the descriptor for this page.
    pub(crate) fn desc(self) -> PageDesc { unsafe { &*self.page_header() }.desc }

    #[inline(always)]
    /// Returns the page header for this page.
    pub(crate) fn page_header(self) -> *mut PageHeader<H> { self.inner.as_ptr().cast::<PageHeader<H>>() }
}

impl<H, T> fmt::Debug for PageRef<H, T> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result { write!(fmt, "PageRef {{}}") }
}
