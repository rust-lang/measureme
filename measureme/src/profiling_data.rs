use crate::event::Event;
use crate::file_header::{
    read_file_header, write_file_header, CURRENT_FILE_FORMAT_VERSION, FILE_HEADER_SIZE,
    FILE_MAGIC_EVENT_STREAM,
};
use crate::serialization::ByteVecSink;
use crate::{
    ProfilerFiles, RawEvent, SerializationSink, StringTable, StringTableBuilder, Timestamp,
    TimestampKind,
};
use std::error::Error;
use std::fs;
use std::mem;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub struct ProfilingData {
    event_data: Vec<u8>,
    string_table: StringTable,
}

impl ProfilingData {
    pub fn new(path_stem: &Path) -> Result<ProfilingData, Box<dyn Error>> {
        let paths = ProfilerFiles::new(path_stem);

        let string_data = fs::read(paths.string_data_file).expect("couldn't read string_data file");
        let index_data =
            fs::read(paths.string_index_file).expect("couldn't read string_index file");
        let event_data = fs::read(paths.events_file).expect("couldn't read events file");

        let event_data_format = read_file_header(&event_data, FILE_MAGIC_EVENT_STREAM)?;
        if event_data_format != CURRENT_FILE_FORMAT_VERSION {
            Err(format!(
                "Event stream file format version '{}' is not supported
                 by this version of `measureme`.",
                event_data_format
            ))?;
        }

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
        let event_start_addr = FILE_HEADER_SIZE + self.curr_event_idx * mem::size_of::<RawEvent>();
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
                std::mem::size_of::<RawEvent>(),
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
                        self.thread_stacks.append(&mut vec![vec![]; growth_size])
                    }

                    self.thread_stacks[thread_id].push(event);
                }
                TimestampKind::Instant => {
                    return Some(MatchingEvent::Instant(event));
                }
                TimestampKind::End => {
                    let thread_id = event.thread_id as usize;
                    let previous_event = self.thread_stacks[thread_id]
                        .pop()
                        .expect("no previous event");
                    if previous_event.event_kind != event.event_kind
                        || previous_event.label != event.label
                    {
                        panic!(
                            "the event with label: \"{}\" went out of scope of the parent \
                             event with label: \"{}\"",
                            previous_event.label, event.label
                        );
                    }

                    return Some(MatchingEvent::StartStop(previous_event, event));
                }
            }
        }

        None
    }
}

/// A `ProfilingDataBuilder` allows for programmatically building
/// `ProfilingData` objects. This is useful for writing tests that expect
/// `ProfilingData` with predictable events (and especially timestamps) in it.
///
/// `ProfilingDataBuilder` provides a convenient interface but its
/// implementation might not be efficient, which why it should only be used for
/// writing tests and other things that are not performance sensitive.
pub struct ProfilingDataBuilder {
    event_sink: ByteVecSink,
    string_table_data_sink: Arc<ByteVecSink>,
    string_table_index_sink: Arc<ByteVecSink>,
    string_table: StringTableBuilder<ByteVecSink>,
}

impl ProfilingDataBuilder {
    pub fn new() -> ProfilingDataBuilder {
        let event_sink = ByteVecSink::new();
        let string_table_data_sink = Arc::new(ByteVecSink::new());
        let string_table_index_sink = Arc::new(ByteVecSink::new());

        // The first thing in every file we generate must be the file header.
        write_file_header(&event_sink, FILE_MAGIC_EVENT_STREAM);

        let string_table = StringTableBuilder::new(
            string_table_data_sink.clone(),
            string_table_index_sink.clone(),
        );

        ProfilingDataBuilder {
            event_sink,
            string_table_data_sink,
            string_table_index_sink,
            string_table,
        }
    }

    /// Record and interval event. Provide an `inner` function for recording
    /// nested events.
    pub fn interval<F>(
        &mut self,
        event_kind: &str,
        event_id: &str,
        thread_id: u64,
        start_nanos: u64,
        end_nanos: u64,
        inner: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let event_kind = self.string_table.alloc(event_kind);
        let event_id = self.string_table.alloc(event_id);

        self.write_raw_event(&RawEvent {
            event_kind,
            id: event_id,
            thread_id,
            timestamp: Timestamp::new(start_nanos, TimestampKind::Start),
        });

        inner(self);

        self.write_raw_event(&RawEvent {
            event_kind,
            id: event_id,
            thread_id,
            timestamp: Timestamp::new(end_nanos, TimestampKind::End),
        });

        self
    }

    /// Record and instant event with the given data.
    pub fn instant(
        &mut self,
        event_kind: &str,
        event_id: &str,
        thread_id: u64,
        timestamp_nanos: u64,
    ) -> &mut Self {
        let event_kind = self.string_table.alloc(event_kind);
        let event_id = self.string_table.alloc(event_id);

        self.write_raw_event(&RawEvent {
            event_kind,
            id: event_id,
            thread_id,
            timestamp: Timestamp::new(timestamp_nanos, TimestampKind::Instant),
        });

        self
    }

    /// Convert this builder into a `ProfilingData` object that can be iterated.
    pub fn into_profiling_data(self) -> ProfilingData {
        // Drop the string table, so that the `string_table_data_sink` and
        // `string_table_index_sink` fields are the only event-sink references
        // left. This enables us to unwrap the `Arc`s and get the byte data out.
        drop(self.string_table);

        let event_data = self.event_sink.into_bytes();
        let data_bytes = Arc::try_unwrap(self.string_table_data_sink)
            .unwrap()
            .into_bytes();
        let index_bytes = Arc::try_unwrap(self.string_table_index_sink)
            .unwrap()
            .into_bytes();

        assert_eq!(
            read_file_header(&event_data, FILE_MAGIC_EVENT_STREAM).unwrap(),
            CURRENT_FILE_FORMAT_VERSION
        );
        let string_table = StringTable::new(data_bytes, index_bytes).unwrap();

        ProfilingData {
            event_data,
            string_table,
        }
    }

    fn write_raw_event(&mut self, raw_event: &RawEvent) {
        let raw_event_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                raw_event as *const _ as *const u8,
                std::mem::size_of::<RawEvent>(),
            )
        };

        self.event_sink
            .write_atomic(std::mem::size_of::<RawEvent>(), |bytes| {
                debug_assert_eq!(bytes.len(), std::mem::size_of::<RawEvent>());
                bytes.copy_from_slice(raw_event_bytes);
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    fn event(
        event_kind: &'static str,
        label: &'static str,
        thread_id: u64,
        nanos: u64,
        timestamp_kind: TimestampKind,
    ) -> Event<'static> {
        let timestamp = SystemTime::UNIX_EPOCH + Duration::from_nanos(nanos);

        Event {
            event_kind: Cow::from(event_kind),
            label: Cow::from(label),
            additional_data: &[],
            timestamp,
            timestamp_kind,
            thread_id,
        }
    }

    #[test]
    fn build_interval_sequence() {
        let mut builder = ProfilingDataBuilder::new();

        builder
            .interval("k1", "id1", 0, 10, 100, |_| {})
            .interval("k2", "id2", 1, 100, 110, |_| {})
            .interval("k3", "id3", 0, 120, 140, |_| {});

        let profiling_data = builder.into_profiling_data();

        let events: Vec<Event<'_>> = profiling_data.iter().collect();

        assert_eq!(events[0], event("k1", "id1", 0, 10, TimestampKind::Start));
        assert_eq!(events[1], event("k1", "id1", 0, 100, TimestampKind::End));
        assert_eq!(events[2], event("k2", "id2", 1, 100, TimestampKind::Start));
        assert_eq!(events[3], event("k2", "id2", 1, 110, TimestampKind::End));
        assert_eq!(events[4], event("k3", "id3", 0, 120, TimestampKind::Start));
        assert_eq!(events[5], event("k3", "id3", 0, 140, TimestampKind::End));
    }

    #[test]
    fn build_nested_intervals() {
        let mut b = ProfilingDataBuilder::new();

        b.interval("k1", "id1", 0, 10, 100, |b| {
            b.interval("k2", "id2", 0, 20, 100, |b| {
                b.interval("k3", "id3", 0, 30, 90, |_| {});
            });
        });

        let profiling_data = b.into_profiling_data();

        let events: Vec<Event<'_>> = profiling_data.iter().collect();

        assert_eq!(events[0], event("k1", "id1", 0, 10, TimestampKind::Start));
        assert_eq!(events[1], event("k2", "id2", 0, 20, TimestampKind::Start));
        assert_eq!(events[2], event("k3", "id3", 0, 30, TimestampKind::Start));
        assert_eq!(events[3], event("k3", "id3", 0, 90, TimestampKind::End));
        assert_eq!(events[4], event("k2", "id2", 0, 100, TimestampKind::End));
        assert_eq!(events[5], event("k1", "id1", 0, 100, TimestampKind::End));
    }

    #[test]
    fn build_intervals_and_instants() {
        let mut b = ProfilingDataBuilder::new();

        b.interval("k1", "id1", 0, 10, 100, |b| {
            b.interval("k2", "id2", 0, 20, 100, |b| {
                b.interval("k3", "id3", 0, 30, 90, |b| {
                    b.instant("k4", "id4", 0, 70);
                    b.instant("k5", "id5", 0, 75);
                });
            })
            .instant("k6", "id6", 0, 95);
        });

        let profiling_data = b.into_profiling_data();

        let events: Vec<Event<'_>> = profiling_data.iter().collect();

        assert_eq!(events[0], event("k1", "id1", 0, 10, TimestampKind::Start));
        assert_eq!(events[1], event("k2", "id2", 0, 20, TimestampKind::Start));
        assert_eq!(events[2], event("k3", "id3", 0, 30, TimestampKind::Start));
        assert_eq!(events[3], event("k4", "id4", 0, 70, TimestampKind::Instant));
        assert_eq!(events[4], event("k5", "id5", 0, 75, TimestampKind::Instant));
        assert_eq!(events[5], event("k3", "id3", 0, 90, TimestampKind::End));
        assert_eq!(events[6], event("k2", "id2", 0, 100, TimestampKind::End));
        assert_eq!(events[7], event("k6", "id6", 0, 95, TimestampKind::Instant));
        assert_eq!(events[8], event("k1", "id1", 0, 100, TimestampKind::End));
    }

}
