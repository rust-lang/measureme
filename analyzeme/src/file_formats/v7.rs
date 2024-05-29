//! This module implements file loading for the v7 file format used until
//! crate version 9.2.0

use std::error::Error;

use super::analyzeme_9_2_0::ProfilingData;
use decodeme::{
    event::Event,
    event_payload::{EventPayload, Timestamp},
    lightweight_event::LightweightEvent,
    Metadata,
};

pub const FILE_FORMAT: u32 = super::analyzeme_9_2_0::CURRENT_FILE_FORMAT_VERSION;

#[derive(Debug)]
pub struct EventDecoder {
    legacy_profiling_data: ProfilingData,
    metadata: Metadata,
}

impl EventDecoder {
    pub fn new(entire_file_data: Vec<u8>) -> Result<EventDecoder, Box<dyn Error + Send + Sync>> {
        let legacy_profiling_data = ProfilingData::from_paged_buffer(entire_file_data)?;

        let metadata = Metadata {
            start_time: legacy_profiling_data.metadata.start_time,
            cmd: legacy_profiling_data.metadata.cmd.clone(),
            process_id: legacy_profiling_data.metadata.process_id,
        };

        Ok(EventDecoder {
            legacy_profiling_data,
            metadata,
        })
    }
}

impl super::EventDecoder for EventDecoder {
    fn num_events(&self) -> usize {
        self.legacy_profiling_data.num_events()
    }

    fn metadata(&self) -> Metadata {
        self.metadata.clone()
    }

    fn decode_full_event(&self, event_index: usize) -> Event<'_> {
        let legacy_event = self.legacy_profiling_data.decode_full_event(event_index);
        let timestamp = convert_timestamp(legacy_event.timestamp);

        Event {
            event_kind: legacy_event.event_kind,
            label: legacy_event.label,
            additional_data: legacy_event.additional_data,
            thread_id: legacy_event.thread_id,
            payload: EventPayload::Timestamp(timestamp),
        }
    }

    fn decode_lightweight_event(&self, event_index: usize) -> LightweightEvent {
        let legacy_event = self
            .legacy_profiling_data
            .decode_lightweight_event(event_index);
        LightweightEvent {
            event_index,
            thread_id: legacy_event.thread_id,
            payload: EventPayload::Timestamp(convert_timestamp(legacy_event.timestamp)),
        }
    }
}

fn convert_timestamp(legacy_timestamp: super::analyzeme_9_2_0::Timestamp) -> Timestamp {
    match legacy_timestamp {
        super::analyzeme_9_2_0::Timestamp::Interval { start, end } => Timestamp::Interval { start, end },
        super::analyzeme_9_2_0::Timestamp::Instant(t) => Timestamp::Instant(t),
    }
}
