use crate::event::Event;
use crate::file_header::{
    read_file_header, write_file_header, CURRENT_FILE_FORMAT_VERSION, FILE_HEADER_SIZE,
    FILE_MAGIC_EVENT_STREAM,
};
use crate::serialization::ByteVecSink;
use crate::{
    ProfilerFiles, RawEvent, SerializationSink, StringTable, StringTableBuilder, Timestamp,
};
use std::error::Error;
use std::fs;
use std::mem;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

const RAW_EVENT_SIZE: usize = mem::size_of::<RawEvent>();

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

    pub fn iter<'a>(&'a self) -> ProfilerEventIterator<'a> {
        ProfilerEventIterator::new(&self)
    }

    pub fn num_events(&self) -> usize {
        let event_byte_count = self.event_data.len() - FILE_HEADER_SIZE;
        assert!(event_byte_count % RAW_EVENT_SIZE == 0);
        event_byte_count / RAW_EVENT_SIZE
    }
}

pub struct ProfilerEventIterator<'a> {
    data: &'a ProfilingData,
    forward_event_idx: usize,
    backward_event_idx: usize,
}

impl<'a> ProfilerEventIterator<'a> {
    pub fn new(data: &'a ProfilingData) -> ProfilerEventIterator<'a> {
        ProfilerEventIterator {
            data,
            forward_event_idx: 0,
            backward_event_idx: data.num_events(),
        }
    }

    fn decode_event(&self, event_index: usize) -> Event<'a> {
        let event_start_addr = event_index_to_addr(event_index);
        let event_end_addr = event_start_addr.checked_add(RAW_EVENT_SIZE).unwrap();

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

        // FIXME: Lots of Rust code compiled in the seventies apparently.
        let timestamp = Timestamp::from_raw_event(&raw_event, SystemTime::UNIX_EPOCH);

        Event {
            event_kind: string_table.get(raw_event.event_kind).to_string(),
            label: string_table.get(raw_event.event_id).to_string(),
            additional_data: &[],
            timestamp,
            thread_id: raw_event.thread_id,
        }
    }
}

impl<'a> Iterator for ProfilerEventIterator<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        if self.forward_event_idx == self.backward_event_idx {
            return None;
        }

        let event = Some(self.decode_event(self.forward_event_idx));

        // Advance the index *after* reading the event
        self.forward_event_idx = self.forward_event_idx.checked_add(1).unwrap();

        event
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let items_left = self
            .backward_event_idx
            .checked_sub(self.forward_event_idx)
            .unwrap();
        (items_left, Some(items_left))
    }
}

impl<'a> DoubleEndedIterator for ProfilerEventIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.forward_event_idx == self.backward_event_idx {
            return None;
        }

        // Advance the index *before* reading the event
        self.backward_event_idx = self.backward_event_idx.checked_sub(1).unwrap();

        Some(self.decode_event(self.backward_event_idx))
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

        inner(self);

        let raw_event =
            RawEvent::new_interval(event_kind, event_id, thread_id, start_nanos, end_nanos);

        self.write_raw_event(&raw_event);

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

        let raw_event = RawEvent::new_instant(event_kind, event_id, thread_id, timestamp_nanos);

        self.write_raw_event(&raw_event);

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

impl<'a> ExactSizeIterator for ProfilerEventIterator<'a> {}

fn event_index_to_addr(event_index: usize) -> usize {
    FILE_HEADER_SIZE + event_index * mem::size_of::<RawEvent>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;
    use std::time::Duration;

    fn interval(
        event_kind: &'static str,
        label: &'static str,
        thread_id: u64,
        start_nanos: u64,
        end_nanos: u64,
    ) -> Event<'static> {
        Event {
            event_kind: Cow::from(event_kind),
            label: Cow::from(label),
            additional_data: &[],
            timestamp: Timestamp::Interval {
                start: SystemTime::UNIX_EPOCH + Duration::from_nanos(start_nanos),
                end: SystemTime::UNIX_EPOCH + Duration::from_nanos(end_nanos),
            },
            thread_id,
        }
    }

    fn instant(
        event_kind: &'static str,
        label: &'static str,
        thread_id: u64,
        timestamp_nanos: u64,
    ) -> Event<'static> {
        Event {
            event_kind: Cow::from(event_kind),
            label: Cow::from(label),
            additional_data: &[],
            timestamp: Timestamp::Instant(
                SystemTime::UNIX_EPOCH + Duration::from_nanos(timestamp_nanos),
            ),
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

        assert_eq!(events[0], interval("k1", "id1", 0, 10, 100));
        assert_eq!(events[1], interval("k2", "id2", 1, 100, 110));
        assert_eq!(events[2], interval("k3", "id3", 0, 120, 140));
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

        assert_eq!(events[0], interval("k3", "id3", 0, 30, 90));
        assert_eq!(events[1], interval("k2", "id2", 0, 20, 100));
        assert_eq!(events[2], interval("k1", "id1", 0, 10, 100));
    }

    #[test]
    fn build_intervals_and_instants() {
        let mut b = ProfilingDataBuilder::new();

        b.interval("k1", "id1", 0, 10, 100, |b| {
            b.interval("k2", "id2", 0, 20, 92, |b| {
                b.interval("k3", "id3", 0, 30, 90, |b| {
                    b.instant("k4", "id4", 0, 70);
                    b.instant("k5", "id5", 0, 75);
                });
            });
            b.instant("k6", "id6", 0, 95);
        });

        let profiling_data = b.into_profiling_data();

        let events: Vec<Event<'_>> = profiling_data.iter().collect();

        assert_eq!(events[0], instant("k4", "id4", 0, 70));
        assert_eq!(events[1], instant("k5", "id5", 0, 75));
        assert_eq!(events[2], interval("k3", "id3", 0, 30, 90));
        assert_eq!(events[3], interval("k2", "id2", 0, 20, 92));
        assert_eq!(events[4], instant("k6", "id6", 0, 95));
        assert_eq!(events[5], interval("k1", "id1", 0, 10, 100));
    }
}
