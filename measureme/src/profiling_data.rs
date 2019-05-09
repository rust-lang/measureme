use crate::file_header::FILE_HEADER_SIZE;
use crate::event::Event;
use crate::{ProfilerFiles, RawEvent, StringTable, TimestampKind};
use std::error::Error;
use std::fs;
use std::mem;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub struct ProfilingData {
    event_data: Vec<u8>,
    string_table: StringTable,
}

impl ProfilingData {
    pub fn new(path_stem: &Path) -> Result<ProfilingData, Box<dyn Error>> {
        let paths = ProfilerFiles::new(path_stem);

        let string_data = fs::read(paths.string_data_file).expect("couldn't read string_data file");
        let index_data = fs::read(paths.string_index_file).expect("couldn't read string_index file");
        let event_data = fs::read(paths.events_file).expect("couldn't read events file");

        let string_table = StringTable::new(string_data, index_data)?;

        Ok(ProfilingData {
            string_table,
            event_data,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = Event<'_>> {
        ProfilerEventIterator::new(&self)
    }

    pub fn iter_matching_events(&self) -> impl Iterator<Item = MatchingEvent<'_>> {
        MatchingEventsIterator::new(ProfilerEventIterator::new(&self))
    }
}

struct ProfilerEventIterator<'a> {
    data: &'a ProfilingData,
    curr_event_idx: usize,
}

impl<'a> ProfilerEventIterator<'a> {
    pub fn new(data: &'a ProfilingData) -> ProfilerEventIterator<'a> {
        ProfilerEventIterator {
            data,
            curr_event_idx: 0,
        }
    }
}

impl<'a> Iterator for ProfilerEventIterator<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        let event_start_addr = FILE_HEADER_SIZE +
            self.curr_event_idx * mem::size_of::<RawEvent>();
        let event_end_addr = event_start_addr + mem::size_of::<RawEvent>();
        if event_end_addr > self.data.event_data.len() {
            return None;
        }

        self.curr_event_idx += 1;

        let raw_event_bytes = &self.data.event_data[event_start_addr..event_end_addr];

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MatchingEvent<'a> {
    StartStop(Event<'a>, Event<'a>),
    Instant(Event<'a>),
}

struct MatchingEventsIterator<'a> {
    events: ProfilerEventIterator<'a>,
    thread_stacks: Vec<Vec<Event<'a>>>,
}

impl<'a> MatchingEventsIterator<'a> {
    pub fn new(events: ProfilerEventIterator<'a>) -> MatchingEventsIterator<'a> {
        MatchingEventsIterator {
            events,
            thread_stacks: vec![],
        }
    }
}

impl<'a> Iterator for MatchingEventsIterator<'a> {
    type Item = MatchingEvent<'a>;

    fn next(&mut self) -> Option<MatchingEvent<'a>> {
        while let Some(event) = self.events.next() {
            match event.timestamp_kind {
                TimestampKind::Start => {
                    let thread_id = event.thread_id as usize;
                    if thread_id >= self.thread_stacks.len() {
                        let growth_size = (thread_id + 1) - self.thread_stacks.len();
                        self.thread_stacks.append(
                            &mut vec![vec![]; growth_size]
                        )
                    }

                    self.thread_stacks[thread_id].push(event);
                },
                TimestampKind::Instant => {
                    return Some(MatchingEvent::Instant(event));
                },
                TimestampKind::End => {
                    let thread_id = event.thread_id as usize;
                    let previous_event = self.thread_stacks[thread_id].pop().expect("no previous event");
                    if previous_event.event_kind != event.event_kind ||
                        previous_event.label != event.label {
                        panic!("previous event on thread wasn't the start event");
                    }

                    return Some(MatchingEvent::StartStop(previous_event, event));
                }
            }
        }

        None
    }
}
