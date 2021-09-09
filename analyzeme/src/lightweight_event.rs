use crate::timestamp::Timestamp;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LightweightEvent {
    pub event_index: usize,
    pub thread_id: u32,
    pub timestamp: Timestamp,
}

impl LightweightEvent {
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
