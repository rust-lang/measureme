use std::error::Error;
use std::path::Path;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Addr(pub u32);

impl Addr {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

pub trait SerializationSink: Sized {
    fn from_path(path: &Path) -> Result<Self, Box<dyn Error>>;

    fn write_atomic<W>(&self, num_bytes: usize, write: W) -> Addr
    where
        W: FnOnce(&mut [u8]);
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::sync::Mutex;

    pub struct TestSink {
        data: Mutex<Vec<u8>>,
    }

    impl TestSink {
        pub fn new() -> TestSink {
            TestSink {
                data: Mutex::new(Vec::new()),
            }
        }

        pub fn into_bytes(self) -> Vec<u8> {
            self.data.into_inner().unwrap()
        }
    }

    impl SerializationSink for TestSink {
        fn from_path(_path: &Path) -> Result<Self, Box<dyn Error>> {
            unimplemented!()
        }

        fn write_atomic<W>(&self, num_bytes: usize, write: W) -> Addr
        where
            W: FnOnce(&mut [u8]),
        {
            let mut data = self.data.lock().unwrap();

            let start = data.len();

            data.resize(start + num_bytes, 0);

            write(&mut data[start..]);

            Addr(start as u32)
        }
    }

    impl std::fmt::Debug for TestSink {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "TestSink")
        }
    }
}
