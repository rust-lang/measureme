use crate::event_payload::{EventPayload, Timestamp};
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LightweightEvent {
    pub event_index: usize,
    pub thread_id: u32,
    pub payload: EventPayload,
}

impl LightweightEvent {
    /// Returns true if the time interval of `self` completely contains the
    /// time interval of `other`.
    pub fn contains(&self, other: &LightweightEvent) -> bool {
        self.payload.contains(&other.payload)
    }

    pub fn duration(&self) -> Option<Duration> {
        self.payload.duration()
    }

    // Returns start time if event is a timestamp
    pub fn start(&self) -> Option<SystemTime> {
        self.payload.timestamp().map(|t| t.start())
    }

    // Returns end time if event is a timestamp
    pub fn end(&self) -> Option<SystemTime> {
        self.payload.timestamp().map(|t| t.end())
    }

    pub fn timestamp(&self) -> Option<Timestamp> {
        self.payload.timestamp()
    }
}
