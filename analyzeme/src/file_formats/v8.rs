//! This module implements file loading for the v8 file format used until
//! crate version 10.0.0

use crate::{Event, LightweightEvent};
pub use decodeme::EventDecoder;
use decodeme::Metadata;

pub const FILE_FORMAT: u32 = decodeme::internal::CURRENT_FILE_FORMAT_VERSION;

impl super::EventDecoder for EventDecoder {
    fn num_events(&self) -> usize {
        self.num_events()
    }

    fn metadata(&self) -> &Metadata {
        self.metadata()
    }

    fn decode_full_event<'a>(&'a self, event_index: usize) -> Event<'a> {
        self.decode_full_event(event_index)
    }

    fn decode_lightweight_event<'a>(&'a self, event_index: usize) -> LightweightEvent {
        self.decode_lightweight_event(event_index)
    }
}
