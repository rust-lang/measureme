use crate::event::Event;
use crate::event_payload::{EventPayload, Timestamp};
use crate::profiling_data::ProfilingData;
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct LightweightEvent<'a> {
    pub data: &'a ProfilingData,
    pub event_index: usize,
    pub thread_id: u32,
    pub payload: EventPayload,
}

impl<'a> LightweightEvent<'a> {
    pub fn to_event(&self) -> Event<'a> {
        self.data.decode_full_event(self.event_index)
    }

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

impl<'a> PartialEq for LightweightEvent<'a> {
    fn eq(&self, other: &LightweightEvent<'a>) -> bool {
        let LightweightEvent {
            data,
            event_index,
            thread_id,
            payload,
        } = *self;

        let LightweightEvent {
            data: other_data,
            event_index: other_event_index,
            thread_id: other_thread_id,
            payload: other_payload,
        } = *other;

        std::ptr::eq(data, other_data)
            && event_index == other_event_index
            && thread_id == other_thread_id
            && payload == other_payload
    }
}

impl<'a> Eq for LightweightEvent<'a> {}

impl<'a> Hash for LightweightEvent<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let LightweightEvent {
            data,
            event_index,
            thread_id,
            payload,
        } = *self;

        std::ptr::hash(data, state);
        event_index.hash(state);
        thread_id.hash(state);
        payload.hash(state);
    }
}
