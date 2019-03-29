use crate::event::Event;
use crate::{ProfilerFiles, RawEvent, StringTable};
use std::fs;
use std::mem;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub struct ProfilingData {
    event_data: Vec<u8>,
    string_table: StringTable,
}

impl ProfilingData {
    pub fn new(path_stem: &Path) -> ProfilingData {
        let paths = ProfilerFiles::new(path_stem);

        let string_data = fs::read(paths.string_data_file).expect("couldn't read string_data file");
        let index_data = fs::read(paths.string_index_file).expect("couldn't read string_index file");
        let event_data = fs::read(paths.events_file).expect("couldn't read events file");

        let string_table = StringTable::new(string_data, index_data);

        ProfilingData {
            string_table,
            event_data,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Event<'_>> {
        ProfilerEventIterator {
            data: self,
            curr_event_idx: 0,
        }
    }
}

struct ProfilerEventIterator<'a> {
    data: &'a ProfilingData,
    curr_event_idx: usize,
}

impl<'a> Iterator for ProfilerEventIterator<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        let raw_idx = self.curr_event_idx * mem::size_of::<RawEvent>();
        let raw_idx_end = raw_idx + mem::size_of::<RawEvent>();
        if raw_idx_end > self.data.event_data.len() {
            return None;
        }

        self.curr_event_idx += 1;

        let raw_event_bytes = &self.data.event_data[raw_idx..raw_idx_end];

        let mut raw_event = RawEvent::default();
        unsafe {
            let raw_event = std::slice::from_raw_parts_mut(
                &mut raw_event as *mut RawEvent as *mut u8,
                std::mem::size_of::<RawEvent>()
            );
            raw_event.copy_from_slice(raw_event_bytes);
        };

        let string_table = &self.data.string_table;

        let mut timestamp = SystemTime::UNIX_EPOCH;
        timestamp += Duration::from_nanos(raw_event.timestamp.nanos());

        Some(Event {
            event_kind: string_table.get(raw_event.event_kind).to_string(),
            label: string_table.get(raw_event.id).to_string(),
            additional_data: &[],
            timestamp: timestamp,
            timestamp_kind: raw_event.timestamp.kind(),
            thread_id: raw_event.thread_id,
        })
    }
}