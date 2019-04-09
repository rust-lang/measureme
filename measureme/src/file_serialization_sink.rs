use crate::serialization::{Addr, SerializationSink};
use std::error::Error;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;

pub struct FileSerializationSink {
    data: Mutex<(BufWriter<fs::File>, u32)>,
}

impl SerializationSink for FileSerializationSink {
    fn from_path(path: &Path) -> Result<Self, Box<dyn Error>> {
        fs::create_dir_all(path.parent().unwrap())?;

        let file = fs::File::create(path)?;

        Ok(FileSerializationSink {
            data: Mutex::new((BufWriter::new(file), 0)),
        })
    }

    #[inline]
    fn write_atomic<W>(&self, num_bytes: usize, write: W) -> Addr
    where
        W: FnOnce(&mut [u8]),
    {
        let mut buffer = vec![0; num_bytes];
        write(buffer.as_mut_slice());

        let mut data = self.data.lock().expect("couldn't acquire lock");
        let curr_addr = data.1;
        let file = &mut data.0;

        file.write_all(&buffer).expect("failed to write buffer");

        data.1 += num_bytes as u32;

        Addr(curr_addr)
    }
}
