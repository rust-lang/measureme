use crate::serialization::{Addr, SerializationSink};
use std::path::Path;

pub struct MmapSerializationSink {}

impl SerializationSink for MmapSerializationSink {
    fn from_path(_path: &Path) -> Self {
        unimplemented!()
    }

    #[inline]
    fn write_atomic<W>(&self, _num_bytes: usize, _write: W) -> Addr
    where
        W: FnOnce(&mut [u8]),
    {
        unimplemented!()
    }
}
