use super::event::Event;
use super::lightweight_event::LightweightEvent;
use super::timestamp::Timestamp;
use super::StringTable;
use super::measureme_9_2_0::file_header::{
    verify_file_header, FILE_HEADER_SIZE,
    FILE_MAGIC_EVENT_STREAM, FILE_MAGIC_TOP_LEVEL,
};
use super::measureme_9_2_0::{PageTag, RawEvent};
use serde::{Deserialize, Deserializer};
use std::mem;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::error::Error;

const RAW_EVENT_SIZE: usize = mem::size_of::<RawEvent>();

fn system_time_from_nanos<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
    D: Deserializer<'de>,
{
    let duration_from_epoch = Duration::from_nanos(u64::deserialize(deserializer)?);
    Ok(UNIX_EPOCH
        .checked_add(duration_from_epoch)
        .expect("a time that can be represented as SystemTime"))
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    #[serde(deserialize_with = "system_time_from_nanos")]
    pub start_time: SystemTime,
    pub process_id: u32,
    pub cmd: String,
}

#[derive(Debug)]
pub struct ProfilingData {
    event_data: Vec<u8>,
    string_table: StringTable,
    pub metadata: Metadata,
}

impl ProfilingData {
    pub fn from_paged_buffer(data: Vec<u8>) -> Result<ProfilingData, Box<dyn Error + Send + Sync>> {
        verify_file_header(&data, FILE_MAGIC_TOP_LEVEL, None, "top-level")?;

        let mut split_data = super::measureme_9_2_0::split_streams(&data[FILE_HEADER_SIZE..]);

        let string_data = split_data.remove(&PageTag::StringData).unwrap();
        let index_data = split_data.remove(&PageTag::StringIndex).unwrap();
        let event_data = split_data.remove(&PageTag::Events).unwrap();

        ProfilingData::from_buffers(string_data, index_data, event_data, None)
    }

    pub fn from_buffers(
        string_data: Vec<u8>,
        string_index: Vec<u8>,
        events: Vec<u8>,
        diagnostic_file_path: Option<&Path>,
    ) -> Result<ProfilingData, Box<dyn Error + Send + Sync>> {
        let index_data = string_index;
        let event_data = events;

        verify_file_header(
            &event_data,
            FILE_MAGIC_EVENT_STREAM,
            diagnostic_file_path,
            "event",
        )?;

        let string_table = StringTable::new(string_data, index_data, diagnostic_file_path)?;

        let metadata = string_table.get_metadata().to_string();
        let metadata: Metadata = serde_json::from_str(&metadata)?;

        Ok(ProfilingData {
            string_table,
            event_data,
            metadata,
        })
    }

    pub fn num_events(&self) -> usize {
        let event_byte_count = self.event_data.len() - FILE_HEADER_SIZE;
        assert!(event_byte_count % RAW_EVENT_SIZE == 0);
        event_byte_count / RAW_EVENT_SIZE
    }

    pub fn decode_full_event<'a>(&'a self, event_index: usize) -> Event<'a> {
        let event_start_addr = event_index_to_addr(event_index);
        let event_end_addr = event_start_addr.checked_add(RAW_EVENT_SIZE).unwrap();

        let raw_event_bytes = &self.event_data[event_start_addr..event_end_addr];
        let raw_event = RawEvent::deserialize(raw_event_bytes);

        let string_table = &self.string_table;

        let timestamp = Timestamp::from_raw_event(&raw_event, self.metadata.start_time);

        let event_id = string_table
            .get(raw_event.event_id.to_string_id())
            .to_string();
        // Parse out the label and arguments from the `event_id`.
        let (label, additional_data) = Event::parse_event_id(event_id);

        Event {
            event_kind: string_table.get(raw_event.event_kind).to_string(),
            label,
            additional_data,
            timestamp,
            thread_id: raw_event.thread_id,
        }
    }

    pub fn decode_lightweight_event<'a>(&'a self, event_index: usize) -> LightweightEvent<'a> {
        let event_start_addr = event_index_to_addr(event_index);
        let event_end_addr = event_start_addr.checked_add(RAW_EVENT_SIZE).unwrap();

        let raw_event_bytes = &self.event_data[event_start_addr..event_end_addr];
        let raw_event = RawEvent::deserialize(raw_event_bytes);

        let timestamp = Timestamp::from_raw_event(&raw_event, self.metadata.start_time);

        LightweightEvent {
            data: self,
            event_index,
            timestamp,
            thread_id: raw_event.thread_id,
        }
    }
}

pub struct ProfilerEventIterator<'a> {
    data: &'a ProfilingData,
    forward_event_idx: usize,
    backward_event_idx: usize,
}

impl<'a> Iterator for ProfilerEventIterator<'a> {
    type Item = LightweightEvent<'a>;

    fn next(&mut self) -> Option<LightweightEvent<'a>> {
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

impl<'a> ExactSizeIterator for ProfilerEventIterator<'a> {}

fn event_index_to_addr(event_index: usize) -> usize {
    FILE_HEADER_SIZE + event_index * mem::size_of::<RawEvent>()
}
