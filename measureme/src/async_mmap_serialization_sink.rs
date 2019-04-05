use crate::serialization::{Addr, SerializationSink};
use std::fs::{File, OpenOptions};
use std::path::{Path};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::os::unix::io::AsRawFd;
use std::io;

/// Implements a `SerializationSink` that uses a file-backed mmap.
pub struct AsyncMmapSerializationSink {
    file: File,
    current_pos: AtomicUsize,
    mapping_start: *mut u8,
    mapping_len: usize,
}

impl SerializationSink for AsyncMmapSerializationSink {
    fn from_path(path: &Path) -> Self {

        // Lazily allocate 1 GB
        let file_size = 1 << 30;

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .unwrap();

        if let Err(e) = file.set_len(file_size as u64) {
            panic!("Error setting file length: {:?}", e);
        }

        //
        let ptr: *mut libc::c_void = unsafe {
            match libc::mmap(0 as *mut _, file_size, libc::PROT_WRITE, libc::MAP_SHARED, file.as_raw_fd(), 0) {
                libc::MAP_FAILED => {
                    panic!("Error creating mmap: {:?}", io::Error::last_os_error())
                }
                other => other,
            }
        };

        // Hint to the OS that it can write old pages to disk once they are
        // fully written.
        unsafe {
            if libc::madvise(ptr, file_size as _, libc::MADV_SEQUENTIAL) != 0 {
                eprintln!("Error during `madvise`: {:?}", io::Error::last_os_error());
            }
        }

        AsyncMmapSerializationSink {
            file,
            current_pos: AtomicUsize::new(0),
            mapping_start: ptr as *mut u8,
            mapping_len: file_size as usize,
        }
    }

    #[inline]
    fn write_atomic<W>(&self, num_bytes: usize, write: W) -> Addr
    where
        W: FnOnce(&mut [u8]),
    {
        // Reserve the range of bytes we'll copy to
        let pos = self.current_pos.fetch_add(num_bytes, Ordering::SeqCst);

        // Bounds checks
        assert!(pos.checked_add(num_bytes).unwrap() <= self.mapping_len);

        let bytes: &mut [u8] = unsafe {
            let start: *mut u8 = self.mapping_start.offset(pos as isize);
            std::slice::from_raw_parts_mut(start, num_bytes)
        };

        write(bytes);

        Addr(pos as u32)
    }
}

impl Drop for AsyncMmapSerializationSink {
    fn drop(&mut self) {
        let actual_size = *self.current_pos.get_mut();

        unsafe {
            // First use `mremap` to shrink the memory map. Otherwise `munmap`
            // would write everything to the backing file, including the
            // memory we never touched.
            let new_addr = libc::mremap(self.mapping_start as *mut _,
                         self.mapping_len as _,
                         actual_size as _,
                         0);

            if new_addr == libc::MAP_FAILED {
                eprintln!("mremap failed: {:?}", io::Error::last_os_error())
            }

            if libc::munmap(new_addr, actual_size as _) != 0 {
                eprintln!("munmap failed: {:?}", io::Error::last_os_error())
            }
        }

        if let Err(e) = self.file.set_len(actual_size as u64) {
            eprintln!("Error setting file length: {:?}", e);
        }
    }
}
