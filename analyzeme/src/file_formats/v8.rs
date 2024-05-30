//! This module implements file loading for the v8 file format used until
//! crate version 10.0.0.
//!
//! The difference from v8 to v9 copes with the expansion of StringId and Addr
//! types from u32 to u64. Most of the EventDecoder interface is actually
//! unchanged, but the construction of "EventDecoder::new", which parses
//! the stream of events, varies based on these sizes.
//!
//! This file provides conversions to current interfaces, relying on an
//! old version of this crate to parse the u32-based v8 version.

use crate::{Event, EventPayload, LightweightEvent, Timestamp};
use decodeme::Metadata;
use decodeme_10::event_payload::EventPayload as OldEventPayload;
use decodeme_10::event_payload::Timestamp as OldTimestamp;
use decodeme_10::lightweight_event::LightweightEvent as OldLightweightEvent;
pub use decodeme_10::EventDecoder;
use decodeme_10::Metadata as OldMetadata;

pub const FILE_FORMAT: u32 = measureme_10::file_header::CURRENT_FILE_FORMAT_VERSION;

// NOTE: These are functionally a hand-rolled "impl From<Old> -> New", but
// given orphan rules, it seems undesirable to spread version-specific
// converters around the codebase.
//
// In lieu of an idiomatic type conversion, we at least centralize compatibility
// with the old "v8" version to this file.

fn v8_metadata_as_current(old: &OldMetadata) -> Metadata {
    Metadata {
        start_time: old.start_time,
        process_id: old.process_id,
        cmd: old.cmd.clone(),
    }
}

fn v8_timestamp_as_current(old: OldTimestamp) -> Timestamp {
    match old {
        OldTimestamp::Interval { start, end } => Timestamp::Interval { start, end },
        OldTimestamp::Instant(t) => Timestamp::Instant(t),
    }
}

fn v8_event_payload_as_current(old: OldEventPayload) -> EventPayload {
    match old {
        OldEventPayload::Timestamp(t) => EventPayload::Timestamp(v8_timestamp_as_current(t)),
        OldEventPayload::Integer(t) => EventPayload::Integer(t),
    }
}

fn v8_lightweightevent_as_current(old: OldLightweightEvent) -> LightweightEvent {
    LightweightEvent {
        event_index: old.event_index,
        thread_id: old.thread_id,
        payload: v8_event_payload_as_current(old.payload),
    }
}

impl super::EventDecoder for EventDecoder {
    fn num_events(&self) -> usize {
        self.num_events()
    }

    fn metadata(&self) -> Metadata {
        let old = self.metadata();
        v8_metadata_as_current(&old)
    }

    fn decode_full_event(&self, event_index: usize) -> Event<'_> {
        let old = self.decode_full_event(event_index);

        Event {
            event_kind: old.event_kind,
            label: old.label,
            additional_data: old.additional_data,
            payload: v8_event_payload_as_current(old.payload),
            thread_id: old.thread_id,
        }
    }

    fn decode_lightweight_event(&self, event_index: usize) -> LightweightEvent {
        v8_lightweightevent_as_current(self.decode_lightweight_event(event_index))
    }
}
