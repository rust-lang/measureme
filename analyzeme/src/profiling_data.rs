use crate::file_formats::EventDecoder;
use crate::{file_formats, Event, LightweightEvent};
use decodeme::{read_file_header, Metadata};
use measureme::file_header::{
    write_file_header, FILE_EXTENSION, FILE_MAGIC_EVENT_STREAM, FILE_MAGIC_TOP_LEVEL,
};
use measureme::{
    EventId, PageTag, RawEvent, SerializationSink, SerializationSinkBuilder, StringTableBuilder,
};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::{error::Error, path::PathBuf};

#[derive(Debug)]
pub struct ProfilingData {
    event_decoder: Box<dyn EventDecoder>,
}

impl ProfilingData {
    pub fn new(path_stem: &Path) -> Result<ProfilingData, Box<dyn Error + Send + Sync>> {
        let paged_path = path_stem.with_extension(FILE_EXTENSION);

        if paged_path.exists() {
            let data = fs::read(&paged_path)?;
            ProfilingData::from_paged_buffer(data, Some(&paged_path))
        } else {
            let mut msg = format!(
                "Could not find profiling data file `{}`.",
                paged_path.display()
            );

            // Let's try to give a helpful error message if we encounter files
            // in the old three-file-format:
            let paths = ProfilerFiles::new(path_stem);

            if paths.events_file.exists()
                || paths.string_data_file.exists()
                || paths.string_index_file.exists()
            {
                msg += "It looks like your profiling data has been generated \
                        by an out-dated version of measureme (0.7 or older).";
            }

            return Err(From::from(msg));
        }
    }

    pub fn from_paged_buffer(
        data: Vec<u8>,
        diagnostic_file_path: Option<&Path>,
    ) -> Result<ProfilingData, Box<dyn Error + Send + Sync>> {
        // let event_decoder = EventDecoder::new(data, diagnostic_file_path)?;
        // Ok(ProfilingData { event_decoder })

        let file_format_version = read_file_header(
            &data,
            FILE_MAGIC_TOP_LEVEL,
            diagnostic_file_path,
            "top-level",
        )?;

        let event_decoder: Box<dyn file_formats::EventDecoder> = match file_format_version {
            file_formats::v7::FILE_FORMAT => Box::new(file_formats::v7::EventDecoder::new(
                data,
                diagnostic_file_path,
            )?),
            file_formats::v8::FILE_FORMAT => Box::new(file_formats::v8::EventDecoder::new(
                data,
                diagnostic_file_path,
            )?),
            unsupported_version => {
                let msg = format!(
                    "File version {} is not support by this version of measureme.",
                    unsupported_version
                );

                return Err(From::from(msg));
            }
        };

        Ok(ProfilingData { event_decoder })
    }

    pub fn metadata(&self) -> &Metadata {
        self.event_decoder.metadata()
    }

    pub fn iter<'a>(&'a self) -> ProfilerEventIterator<'a> {
        ProfilerEventIterator::new(&self)
    }

    pub fn iter_full<'a>(
        &'a self,
    ) -> impl Iterator<Item = Event<'a>> + DoubleEndedIterator + ExactSizeIterator + 'a {
        self.iter().map(move |e| self.to_full_event(&e))
    }

    pub fn num_events(&self) -> usize {
        self.event_decoder.num_events()
    }

    pub fn to_full_event<'a>(&'a self, light_weight_event: &LightweightEvent) -> Event<'a> {
        self.decode_full_event(light_weight_event.event_index)
    }

    pub(crate) fn decode_full_event<'a>(&'a self, event_index: usize) -> Event<'a> {
        self.event_decoder.decode_full_event(event_index)
    }

    fn decode_lightweight_event<'a>(&'a self, event_index: usize) -> LightweightEvent {
        self.event_decoder.decode_lightweight_event(event_index)
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
}

impl<'a> Iterator for ProfilerEventIterator<'a> {
    type Item = LightweightEvent;

    fn next(&mut self) -> Option<LightweightEvent> {
        if self.forward_event_idx == self.backward_event_idx {
            return None;
        }

        let event = Some(self.data.decode_lightweight_event(self.forward_event_idx));

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

        Some(self.data.decode_lightweight_event(self.backward_event_idx))
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
    event_sink: SerializationSink,
    string_table_data_sink: Arc<SerializationSink>,
    string_table_index_sink: Arc<SerializationSink>,
    string_table: StringTableBuilder,
}

impl ProfilingDataBuilder {
    pub fn new() -> ProfilingDataBuilder {
        let sink_builder = SerializationSinkBuilder::new_in_memory();

        let event_sink = sink_builder.new_sink(PageTag::Events);
        let string_table_data_sink = Arc::new(sink_builder.new_sink(PageTag::StringData));
        let string_table_index_sink = Arc::new(sink_builder.new_sink(PageTag::StringIndex));

        // The first thing in every file we generate must be the file header.
        write_file_header(&mut event_sink.as_std_write(), FILE_MAGIC_EVENT_STREAM).unwrap();

        let string_table = StringTableBuilder::new(
            string_table_data_sink.clone(),
            string_table_index_sink.clone(),
        )
        .unwrap();

        string_table.alloc_metadata(&*format!(
            r#"{{ "start_time": {}, "process_id": {}, "cmd": "{}" }}"#,
            0, 0, "test cmd",
        ));

        ProfilingDataBuilder {
            event_sink,
            string_table_data_sink,
            string_table_index_sink,
            string_table,
        }
    }

    /// Record an interval event. Provide an `inner` function for recording
    /// nested events.
    pub fn interval<F>(
        &mut self,
        event_kind: &str,
        event_id: &str,
        thread_id: u32,
        start_nanos: u64,
        end_nanos: u64,
        inner: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let event_kind = self.string_table.alloc(event_kind);
        let event_id = EventId::from_label(self.string_table.alloc(event_id));

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
        thread_id: u32,
        timestamp_nanos: u64,
    ) -> &mut Self {
        let event_kind = self.string_table.alloc(event_kind);
        let event_id = EventId::from_label(self.string_table.alloc(event_id));
        let raw_event = RawEvent::new_instant(event_kind, event_id, thread_id, timestamp_nanos);

        self.write_raw_event(&raw_event);

        self
    }

    /// Record and instant event with the given data.
    pub fn integer(
        &mut self,
        event_kind: &str,
        event_id: &str,
        thread_id: u32,
        value: u64,
    ) -> &mut Self {
        let event_kind = self.string_table.alloc(event_kind);
        let event_id = EventId::from_label(self.string_table.alloc(event_id));
        let raw_event = RawEvent::new_integer(event_kind, event_id, thread_id, value);

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
        let string_data = Arc::try_unwrap(self.string_table_data_sink)
            .unwrap()
            .into_bytes();
        let index_data = Arc::try_unwrap(self.string_table_index_sink)
            .unwrap()
            .into_bytes();

        ProfilingData {
            event_decoder: Box::new(
                file_formats::current::EventDecoder::from_separate_buffers(
                    string_data,
                    index_data,
                    event_data,
                    None,
                )
                .unwrap(),
            ),
        }
    }

    fn write_raw_event(&mut self, raw_event: &RawEvent) {
        self.event_sink
            .write_atomic(std::mem::size_of::<RawEvent>(), |bytes| {
                raw_event.serialize(bytes);
            });
    }
}

impl<'a> ExactSizeIterator for ProfilerEventIterator<'a> {}

// This struct reflects what filenames were in old versions of measureme. It is
// used only for giving helpful error messages now if a user tries to load old
// data.
struct ProfilerFiles {
    pub events_file: PathBuf,
    pub string_data_file: PathBuf,
    pub string_index_file: PathBuf,
}

impl ProfilerFiles {
    fn new<P: AsRef<Path>>(path_stem: P) -> ProfilerFiles {
        ProfilerFiles {
            events_file: path_stem.as_ref().with_extension("events"),
            string_data_file: path_stem.as_ref().with_extension("string_data"),
            string_index_file: path_stem.as_ref().with_extension("string_index"),
        }
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use super::*;
    use std::{borrow::Cow, time::SystemTime};
    use crate::{EventPayload, Timestamp};
    use std::time::Duration;

    fn full_interval(
        event_kind: &'static str,
        label: &'static str,
        thread_id: u32,
        start_nanos: u64,
        end_nanos: u64,
    ) -> Event<'static> {
        Event {
            event_kind: Cow::from(event_kind),
            label: Cow::from(label),
            additional_data: Vec::new(),
            payload: EventPayload::Timestamp(Timestamp::Interval {
                start: SystemTime::UNIX_EPOCH + Duration::from_nanos(start_nanos),
                end: SystemTime::UNIX_EPOCH + Duration::from_nanos(end_nanos),
            }),
            thread_id,
        }
    }

    fn full_instant(
        event_kind: &'static str,
        label: &'static str,
        thread_id: u32,
        timestamp_nanos: u64,
    ) -> Event<'static> {
        Event {
            event_kind: Cow::from(event_kind),
            label: Cow::from(label),
            additional_data: Vec::new(),
            payload: EventPayload::Timestamp(Timestamp::Instant(
                SystemTime::UNIX_EPOCH + Duration::from_nanos(timestamp_nanos),
            )),
            thread_id,
        }
    }

    fn full_integer(
        event_kind: &'static str,
        label: &'static str,
        thread_id: u32,
        value: u64,
    ) -> Event<'static> {
        Event {
            event_kind: Cow::from(event_kind),
            label: Cow::from(label),
            additional_data: Vec::new(),
            payload: EventPayload::Integer(value),
            thread_id,
        }
    }

    fn lightweight_interval<'a>(
        event_index: usize,
        thread_id: u32,
        start_nanos: u64,
        end_nanos: u64,
    ) -> LightweightEvent {
        LightweightEvent {
            event_index,
            thread_id,
            payload: EventPayload::Timestamp(Timestamp::Interval {
                start: SystemTime::UNIX_EPOCH + Duration::from_nanos(start_nanos),
                end: SystemTime::UNIX_EPOCH + Duration::from_nanos(end_nanos),
            }),
        }
    }

    fn lightweight_instant<'a>(
        event_index: usize,
        thread_id: u32,
        timestamp_nanos: u64,
    ) -> LightweightEvent {
        LightweightEvent {
            event_index,
            thread_id,
            payload: EventPayload::Timestamp(Timestamp::Instant(
                SystemTime::UNIX_EPOCH + Duration::from_nanos(timestamp_nanos),
            )),
        }
    }

    fn lightweight_integer<'a>(
        event_index: usize,
        thread_id: u32,
        value: u64,
    ) -> LightweightEvent {
        LightweightEvent {
            event_index,
            thread_id,
            payload: EventPayload::Integer(value),
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

        let events: Vec<LightweightEvent> = profiling_data.iter().collect();

        assert_eq!(events[0], lightweight_interval(0, 0, 10, 100));
        assert_eq!(events[1], lightweight_interval(1, 1, 100, 110));
        assert_eq!(events[2], lightweight_interval(2, 0, 120, 140));

        assert_eq!(profiling_data.to_full_event(&events[0]), full_interval("k1", "id1", 0, 10, 100));
        assert_eq!(profiling_data.to_full_event(&events[1]), full_interval("k2", "id2", 1, 100, 110));
        assert_eq!(profiling_data.to_full_event(&events[2]), full_interval("k3", "id3", 0, 120, 140));
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

        let events: Vec<LightweightEvent> = profiling_data.iter().collect();

        assert_eq!(events[0], lightweight_interval(0, 0, 30, 90));
        assert_eq!(events[1], lightweight_interval(1, 0, 20, 100));
        assert_eq!(events[2], lightweight_interval(2, 0, 10, 100));

        assert_eq!(profiling_data.to_full_event(&events[0]), full_interval("k3", "id3", 0, 30, 90));
        assert_eq!(profiling_data.to_full_event(&events[1]), full_interval("k2", "id2", 0, 20, 100));
        assert_eq!(profiling_data.to_full_event(&events[2]), full_interval("k1", "id1", 0, 10, 100));
    }

    #[test]
    fn build_intervals_and_instants() {
        let mut b = ProfilingDataBuilder::new();

        b.interval("k1", "id1", 0, 10, 100, |b| {
            b.interval("k2", "id2", 0, 20, 92, |b| {
                b.interval("k3", "id3", 0, 30, 90, |b| {
                    b.instant("k4", "id4", 0, 70);
                    b.integer("k5", "id5", 0, 42);
                    b.instant("k6", "id6", 0, 75);
                });
            });
            b.instant("k7", "id7", 0, 95);
        });

        let profiling_data = b.into_profiling_data();

        let events: Vec<LightweightEvent> = profiling_data.iter().collect();

        assert_eq!(events[0], lightweight_instant(0, 0, 70));
        assert_eq!(events[1], lightweight_integer(1, 0, 42));
        assert_eq!(events[2], lightweight_instant(2, 0, 75));
        assert_eq!(events[3], lightweight_interval(3, 0, 30, 90));
        assert_eq!(events[4], lightweight_interval(4, 0, 20, 92));
        assert_eq!(events[5], lightweight_instant(5, 0, 95));
        assert_eq!(events[6], lightweight_interval(6, 0, 10, 100));

        assert_eq!(profiling_data.to_full_event(&events[0]), full_instant("k4", "id4", 0, 70));
        assert_eq!(profiling_data.to_full_event(&events[1]), full_integer("k5", "id5", 0, 42));
        assert_eq!(profiling_data.to_full_event(&events[2]), full_instant("k6", "id6", 0, 75));
        assert_eq!(profiling_data.to_full_event(&events[3]), full_interval("k3", "id3", 0, 30, 90));
        assert_eq!(profiling_data.to_full_event(&events[4]), full_interval("k2", "id2", 0, 20, 92));
        assert_eq!(profiling_data.to_full_event(&events[5]), full_instant("k7", "id7", 0, 95));
        assert_eq!(profiling_data.to_full_event(&events[6]), full_interval("k1", "id1", 0, 10, 100));
    }
}
