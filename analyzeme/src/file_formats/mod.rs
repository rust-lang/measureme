use decodeme::{event::Event, lightweight_event::LightweightEvent, Metadata};
use std::fmt::Debug;

pub mod v8;
pub mod v9;

pub use v9 as current;

/// The [EventDecoder] knows how to decode events for a specific file format.
pub trait EventDecoder: Debug + Send + Sync {
    fn num_events(&self) -> usize;
    fn metadata(&self) -> Metadata;
    fn decode_full_event<'a>(&'a self, event_index: usize) -> Event<'a>;
    fn decode_lightweight_event<'a>(&'a self, event_index: usize) -> LightweightEvent;
}
