use std::{
    error::Error,
    mem,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use event::Event;
use event_payload::EventPayload;
use lightweight_event::LightweightEvent;
use super::measureme_10_1_2::file_header::{verify_file_header, FILE_MAGIC_EVENT_STREAM};
use super::measureme_10_1_2;

pub mod event;
pub mod event_payload;
pub mod lightweight_event;
pub mod stringtable;

// These re-exports allow us to use some types from the measureme version tied to this
// version of decodeme, with explicitly mentioning that measureme version in downstream
// Cargo.tomls.
pub use super::measureme_10_1_2::file_header::FILE_HEADER_SIZE;
pub use super::measureme_10_1_2::file_header::FILE_MAGIC_TOP_LEVEL;
pub use super::measureme_10_1_2::PageTag;
pub use super::measureme_10_1_2::RawEvent;

use serde::{Deserialize, Deserializer};
use stringtable::StringTable;

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

const RAW_EVENT_SIZE: usize = std::mem::size_of::<RawEvent>();

#[derive(Debug)]
pub struct EventDecoder {
    event_data: Vec<u8>,
    stringtable: StringTable,
    metadata: Metadata,
}

impl EventDecoder {
    pub fn new(
        entire_file_data: Vec<u8>,
        diagnostic_file_path: Option<&Path>,
    ) -> Result<EventDecoder, Box<dyn Error + Send + Sync>> {
        verify_file_header(
            &entire_file_data,
            FILE_MAGIC_TOP_LEVEL,
            diagnostic_file_path,
            "top-level",
        )?;

        let mut split_data = super::measureme_10_1_2::split_streams(&entire_file_data[FILE_HEADER_SIZE..]);

        let string_data = split_data.remove(&PageTag::StringData).expect("Invalid file: No string data found");
        let index_data = split_data.remove(&PageTag::StringIndex).expect("Invalid file: No string index data found");
        let event_data = split_data.remove(&PageTag::Events).expect("Invalid file: No event data found");

        Self::from_separate_buffers(string_data, index_data, event_data, diagnostic_file_path)
    }

    pub fn from_separate_buffers(
        string_data: Vec<u8>,
        index_data: Vec<u8>,
        event_data: Vec<u8>,
        diagnostic_file_path: Option<&Path>,
    ) -> Result<EventDecoder, Box<dyn Error + Send + Sync>> {
        verify_file_header(
            &event_data,
            FILE_MAGIC_EVENT_STREAM,
            diagnostic_file_path,
            "event",
        )?;

        let stringtable = StringTable::new(string_data, index_data, diagnostic_file_path)?;

        let metadata = stringtable.get_metadata().to_string();
        let metadata: Metadata = serde_json::from_str(&metadata)?;

        Ok(EventDecoder {
            event_data,
            stringtable,
            metadata,
        })
    }

    pub fn num_events(&self) -> usize {
        let event_byte_count = self.event_data.len() - FILE_HEADER_SIZE;
        assert!(event_byte_count % RAW_EVENT_SIZE == 0);
        event_byte_count / RAW_EVENT_SIZE
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn decode_full_event<'a>(&'a self, event_index: usize) -> Event<'a> {
        let event_start_addr = event_index_to_addr(event_index);
        let event_end_addr = event_start_addr.checked_add(RAW_EVENT_SIZE).unwrap();

        let raw_event_bytes = &self.event_data[event_start_addr..event_end_addr];
        let raw_event = RawEvent::deserialize(raw_event_bytes);

        let stringtable = &self.stringtable;

        let payload = EventPayload::from_raw_event(&raw_event, self.metadata.start_time);

        let event_id = stringtable
            .get(raw_event.event_id.to_string_id())
            .to_string();

        // Parse out the label and arguments from the `event_id`.
        let (label, additional_data) = Event::parse_event_id(event_id);

        Event {
            event_kind: stringtable.get(raw_event.event_kind).to_string(),
            label,
            additional_data,
            payload,
            thread_id: raw_event.thread_id,
        }
    }

    pub fn decode_lightweight_event<'a>(&'a self, event_index: usize) -> LightweightEvent {
        let event_start_addr = event_index_to_addr(event_index);
        let event_end_addr = event_start_addr.checked_add(RAW_EVENT_SIZE).unwrap();

        let raw_event_bytes = &self.event_data[event_start_addr..event_end_addr];
        let raw_event = RawEvent::deserialize(raw_event_bytes);

        let payload = EventPayload::from_raw_event(&raw_event, self.metadata.start_time);

        LightweightEvent {
            event_index,
            payload,
            thread_id: raw_event.thread_id,
        }
    }
}

fn event_index_to_addr(event_index: usize) -> usize {
    FILE_HEADER_SIZE + event_index * mem::size_of::<RawEvent>()
}
