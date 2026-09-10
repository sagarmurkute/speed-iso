use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

pub const DEFAULT_ALIGNMENT: usize = 4096;
pub const DEFAULT_BUFFER_SIZE: usize = 2 * 1024 * 1024; // 2MB

/// Sector-aligned memory buffer allocated via raw system allocator.
/// Guarantees physical sector alignment required for unbuffered raw block disk writes (`O_DIRECT`, `FILE_FLAG_NO_BUFFERING`).
pub struct AlignedBuffer {
    ptr: NonNull<u8>,
    layout: Layout,
}

impl AlignedBuffer {
    /// Creates a new sector-aligned buffer of the specified size (in bytes).
    /// Size will be rounded up to the nearest multiple of `DEFAULT_ALIGNMENT` (4096 bytes).
    pub fn new(size: usize) -> Result<Self, String> {
        let aligned_size = if size % DEFAULT_ALIGNMENT == 0 {
            size
        } else {
            size + (DEFAULT_ALIGNMENT - (size % DEFAULT_ALIGNMENT))
        };

        let layout = Layout::from_size_align(aligned_size, DEFAULT_ALIGNMENT)
            .map_err(|e| format!("Invalid memory layout: {}", e))?;

        let raw_ptr = unsafe { alloc_zeroed(layout) };
        let ptr = NonNull::new(raw_ptr)
            .ok_ok_or_else(|| "Failed to allocate sector-aligned memory buffer".to_string())?;

        Ok(Self { ptr, layout })
    }

    /// Creates a buffer with default size (2MB) and 4096-byte sector alignment.
    pub fn with_default_capacity() -> Result<Self, String> {
        Self::new(DEFAULT_BUFFER_SIZE)
    }

    pub fn capacity(&self) -> usize {
        self.layout.size()
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.layout.size()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.layout.size()) }
    }
}

// Helper trait implementation for option-like error handling without unwrap
trait OptionExt<T> {
    fn ok_ok_or_else<F: FnOnce() -> String>(self, err: F) -> Result<T, String>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_ok_or_else<F: FnOnce() -> String>(self, err: F) -> Result<T, String> {
        match self {
            Some(val) => Ok(val),
            None => Err(err()),
        }
    }
}

impl Deref for AlignedBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for AlignedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_creation_and_alignment() {
        let buf = AlignedBuffer::new(1024).expect("Allocation failed");
        assert_eq!(buf.capacity(), DEFAULT_ALIGNMENT); // Rounded up to 4096
        assert_eq!(buf.as_ptr() as usize % DEFAULT_ALIGNMENT, 0);

        let buf_2mb = AlignedBuffer::with_default_capacity().expect("Allocation failed");
        assert_eq!(buf_2mb.capacity(), 2 * 1024 * 1024);
        assert_eq!(buf_2mb.as_ptr() as usize % DEFAULT_ALIGNMENT, 0);
    }
}
