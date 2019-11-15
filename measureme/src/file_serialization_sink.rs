use crate::serialization::{Addr, SerializationSink};
use std::error::Error;
use std::fs;
use std::io::{Write};
use std::path::Path;
use parking_lot::Mutex;

pub struct FileSerializationSink {
    data: Mutex<Inner>,
}

struct Inner {
    file: fs::File,
    buffer: Vec<u8>,
    buf_pos: usize,
    addr: u32,
}

impl SerializationSink for FileSerializationSink {
    fn from_path(path: &Path) -> Result<Self, Box<dyn Error>> {
        fs::create_dir_all(path.parent().unwrap())?;

        let file = fs::File::create(path)?;

        Ok(FileSerializationSink {
            data: Mutex::new(Inner {
                file,
                buffer: vec![0; 1024*512],
                buf_pos: 0,
                addr: 0
            }),
        })
    }

    #[inline]
    fn write_atomic<W>(&self, num_bytes: usize, write: W) -> Addr
    where
        W: FnOnce(&mut [u8]),
    {
        let mut data = self.data.lock();
        let Inner {
            ref mut file,
            ref mut buffer,
            ref mut buf_pos,
            ref mut addr
        } = *data;

        assert!(num_bytes <= buffer.len());
        let mut buf_start = *buf_pos;
        let mut buf_end = buf_start + num_bytes;

        if buf_end > buffer.len() {
            file.write_all(&buffer[..buf_start]).expect("failed to write buffer");
            buf_start = 0;
            buf_end = num_bytes;
        }

        write(&mut buffer[buf_start .. buf_end]);
        *buf_pos = buf_end;

        let curr_addr = *addr;
        *addr += num_bytes as u32;
        Addr(curr_addr)
    }
}

impl Drop for FileSerializationSink {
    fn drop(&mut self) {
        let mut data = self.data.lock();
        let Inner {
            ref mut file,
            ref mut buffer,
            ref mut buf_pos,
            addr: _,
        } = *data;

        if *buf_pos > 0 {
            file.write_all(&buffer[..*buf_pos]).expect("failed to write buffer");
        }
    }
}
