use crate::event::Event;
use std::path::Path;

pub struct ProfilingData {}

impl ProfilingData {
    pub fn new(_path_stem: &Path) -> ProfilingData {
        unimplemented!()
    }

    pub fn iter_events<'a, F>(&'a self, mut _f: F)
    where
        F: FnMut(&Event<'a>),
    {
        unimplemented!()
    }
}
