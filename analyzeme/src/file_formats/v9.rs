//! This module implements file loading for the v9 file format

use crate::{Event, LightweightEvent};
pub use decodeme::EventDecoder;
use decodeme::Metadata;

pub const FILE_FORMAT: u32 = decodeme::CURRENT_FILE_FORMAT_VERSION;

impl super::EventDecoder for EventDecoder {
    fn num_events(&self) -> usize {
        self.num_events()
    }

    fn metadata(&self) -> Metadata {
        self.metadata()
    }

    fn decode_full_event(&self, event_index: usize) -> Event<'_> {
        self.decode_full_event(event_index)
    }

    fn decode_lightweight_event(&self, event_index: usize) -> LightweightEvent {
        self.decode_lightweight_event(event_index)
    }
}
