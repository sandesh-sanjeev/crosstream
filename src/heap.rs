use std::{
    alloc::{Layout, alloc, dealloc, handle_alloc_error},
    ptr::copy_nonoverlapping,
    slice::from_raw_parts,
};

unsafe impl<T> Sync for Array<T> {}
unsafe impl<T> Send for Array<T> {}

pub(crate) struct Array<T> {
    len: usize,
    ptr: *mut T,
    layout: Layout,
}

impl<T> Array<T> {
    pub(crate) fn alloc(len: usize) -> Self {
        let Ok(layout) = Layout::array::<T>(len) else {
            panic!("Invalid array allocation of size {len}");
        };

        // Safety: Made sure layout is valid above.
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            handle_alloc_error(layout);
        }

        Self {
            len,
            layout,
            ptr: ptr as *mut T,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn memcpy(
        &mut self,
        dst_index: usize,
        src: *const T,
        src_index: usize,
        count: usize,
    ) {
        // Safety: Safety must be upheld by the caller.
        unsafe {
            let src_ptr = src.add(src_index);
            let dst_ptr = self.ptr.add(dst_index);
            copy_nonoverlapping(src_ptr, dst_ptr, count);
        }
    }

    pub(crate) fn as_slice(&self, index: usize, len: usize) -> &[T] {
        // Safety: Safety must be upheld by the caller.
        unsafe {
            let ptr = self.ptr.add(index);
            from_raw_parts(ptr, len)
        }
    }
}

impl<T> Drop for Array<T> {
    fn drop(&mut self) {
        // Safety: Pointer is guaranteed to be non-null.
        unsafe {
            dealloc(self.ptr as *mut u8, self.layout);
        }
    }
}
