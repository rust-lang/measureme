use crate::serialization::{Addr, SerializationSink};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

pub struct FileSerializationSink {
    data: Mutex<(fs::File, u32)>,
}

impl SerializationSink for FileSerializationSink {
    fn from_path(path: &Path) -> Self {
        FileSerializationSink {
            data: Mutex::new((fs::File::create(path).expect("couldn't open file: {}"), 0)),
        }
    }

    #[inline]
    fn write_atomic<W>(&self, num_bytes: usize, write: W) -> Addr
        where W: FnOnce(&mut [u8]),
    {
        let mut buffer = vec![0; num_bytes];
        write(buffer.as_mut_slice());

        let mut data = self.data.lock().expect("couldn't acquire lock");
        let mut file = &data.0;
        let curr_addr = data.1;

        file.write_all(&buffer).expect("failed to write buffer");

        data.1 += num_bytes as u32;

        Addr(curr_addr)
    }
}
