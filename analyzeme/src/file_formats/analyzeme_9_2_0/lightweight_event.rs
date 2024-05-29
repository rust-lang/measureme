use super::event::Event;
use super::profiling_data::ProfilingData;
use super::timestamp::Timestamp;
use std::hash::{Hash, Hasher};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct LightweightEvent<'a> {
    pub data: &'a ProfilingData,
    pub event_index: usize,
    pub thread_id: u32,
    pub timestamp: Timestamp,
}

impl<'a> LightweightEvent<'a> {
    pub fn to_event(&self) -> Event<'a> {
        self.data.decode_full_event(self.event_index)
    }

    /// Returns true if the time interval of `self` completely contains the
    /// time interval of `other`.
    pub fn contains(&self, other: &LightweightEvent) -> bool {
        match self.timestamp {
            Timestamp::Interval {
                start: self_start,
                end: self_end,
            } => match other.timestamp {
                Timestamp::Interval {
                    start: other_start,
                    end: other_end,
                } => self_start <= other_start && other_end <= self_end,
                Timestamp::Instant(other_t) => self_start <= other_t && other_t <= self_end,
            },
            Timestamp::Instant(_) => false,
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        match self.timestamp {
            Timestamp::Interval { start, end } => end.duration_since(start).ok(),
            Timestamp::Instant(_) => None,
        }
    }
}

impl<'a> PartialEq for LightweightEvent<'a> {
    fn eq(&self, other: &LightweightEvent<'a>) -> bool {
        let LightweightEvent {
            data,
            event_index,
            thread_id,
            timestamp,
        } = *self;

        let LightweightEvent {
            data: other_data,
            event_index: other_event_index,
            thread_id: other_thread_id,
            timestamp: other_timestamp,
        } = *other;

        std::ptr::eq(data, other_data)
            && event_index == other_event_index
            && thread_id == other_thread_id
            && timestamp == other_timestamp
    }
}

impl<'a> Eq for LightweightEvent<'a> {}

impl<'a> Hash for LightweightEvent<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let LightweightEvent {
            data,
            event_index,
            thread_id,
            timestamp,
        } = *self;

        std::ptr::hash(data, state);
        event_index.hash(state);
        thread_id.hash(state);
        timestamp.hash(state);
    }
}
