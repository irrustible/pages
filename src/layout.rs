use alloc::alloc::Layout;
use core::convert::TryInto;
use core::fmt;
use core::marker::PhantomData;

pub struct PageHeader<H> {
    pub header: H,
    pub desc:   PageDesc,
}

impl<H> fmt::Debug for PageHeader<H> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "PageHeader[{}]", self.desc.items)
    }
}

/// Just the numerics from a page layout. Very compact (64 bits).
#[derive(Clone,Copy)]
pub struct PageDesc {
    /// Our item capacity
    pub items: u32,
    /// The offset from the start of a page to the data array.
    pub data:  u32,
}

impl fmt::Debug for PageDesc {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "PageDesc[{}]", self.items)
    }
}

impl<H, T> From<PageLayout<H, T>> for PageDesc {
    #[inline(always)]
    fn from(layout: PageLayout<H, T>) -> Self { layout.desc }
}

/// Describes the memory layout for a Page.
pub struct PageLayout<H, T> {
    /// Information about how the layout is composed
    pub(crate) desc:   PageDesc,
    /// An allocator layout suitable for allocation/deallocation.
    layout: Layout,
    _phantom: PhantomData<(H, T)>,
}

impl<H, T> PageLayout<H, T> {
    /// Creates a [`PageLayout`] describing a page with the given item capacity.
    ///
    /// ## Note
    ///
    /// Will panic if items is 0 or the header plus padding is extremely large
    /// (u32::MAX - 8 bytes)
    #[inline(always)]
    pub fn with_capacity(items: u32) -> Self {
        assert!(items > 0); // Use a box.
        let header = Layout::new::<PageHeader<H>>();
        let array = Layout::array::<T>(items as usize).unwrap();
        let (layout, data) = header.extend(array).unwrap();
        let layout = layout.pad_to_align();
        let desc = PageDesc { items, data: data.try_into().unwrap() };
        Self { desc, layout, _phantom: PhantomData }
    }

    /// Returns a [`Layout`] suitable for passing to [`alloc`] / [`dealloc`].
    #[inline(always)]
    pub fn layout(self) -> Layout { self.layout }
}

impl<H, T> Clone for PageLayout<H, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        PageLayout { desc: self.desc, layout: self.layout, _phantom: self._phantom }
    }
}

impl<H, T> Copy for PageLayout<H, T> {}
