use std::error::Error;
use std::path::Path;
use parking_lot::Mutex;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Addr(pub u32);

impl Addr {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

pub trait SerializationSink: Sized + Send + Sync + 'static {
    fn from_path(path: &Path) -> Result<Self, Box<dyn Error>>;

    fn write_atomic<W>(&self, num_bytes: usize, write: W) -> Addr
    where
        W: FnOnce(&mut [u8]);
}

/// A `SerializationSink` that writes to an internal `Vec<u8>` and can be
/// converted into this raw `Vec<u8>`. This implementation is only meant to be
/// used for testing and is not very efficient.
pub struct ByteVecSink {
    data: Mutex<Vec<u8>>,
}

impl ByteVecSink {
    pub fn new() -> ByteVecSink {
        ByteVecSink {
            data: Mutex::new(Vec::new()),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data.into_inner()
    }
}

impl SerializationSink for ByteVecSink {
    fn from_path(_path: &Path) -> Result<Self, Box<dyn Error>> {
        unimplemented!()
    }

    fn write_atomic<W>(&self, num_bytes: usize, write: W) -> Addr
    where
        W: FnOnce(&mut [u8]),
    {
        let mut data = self.data.lock();

        let start = data.len();

        data.resize(start + num_bytes, 0);

        write(&mut data[start..]);

        Addr(start as u32)
    }
}

impl std::fmt::Debug for ByteVecSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ByteVecSink")
    }
}
