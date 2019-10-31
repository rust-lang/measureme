use crate::raw_event::RawEvent;
use std::borrow::Cow;
use std::time::{Duration, SystemTime};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Event<'a> {
    pub event_kind: Cow<'a, str>,
    pub label: Cow<'a, str>,
    pub additional_data: &'a [Cow<'a, str>],
    pub timestamp: Timestamp,
    pub thread_id: u64,
}

impl<'a> Event<'a> {
    /// Returns true if the time interval of `self` completely contains the
    /// time interval of `other`.
    pub fn contains(&self, other: &Event<'_>) -> bool {
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

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Timestamp {
    Interval { start: SystemTime, end: SystemTime },
    Instant(SystemTime),
}

impl Timestamp {
    pub fn from_raw_event(raw_event: &RawEvent, start_time: SystemTime) -> Timestamp {
        if raw_event.end_ns == std::u64::MAX {
            let t = start_time + Duration::from_nanos(raw_event.start_ns);
            Timestamp::Instant(t)
        } else {
            let start = start_time + Duration::from_nanos(raw_event.start_ns);
            let end = start_time + Duration::from_nanos(raw_event.end_ns);
            Timestamp::Interval { start, end }
        }
    }

    pub fn contains(&self, t: SystemTime) -> bool {
        match *self {
            Timestamp::Interval { start, end } => t >= start && t < end,
            Timestamp::Instant(_) => false,
        }
    }

    pub fn is_instant(&self) -> bool {
        match *self {
            Timestamp::Interval { .. } => false,
            Timestamp::Instant(_) => true,
        }
    }

    pub fn start(&self) -> SystemTime {
        match *self {
            Timestamp::Interval { start, .. } => start,
            Timestamp::Instant(t) => t,
        }
    }

    pub fn end(&self) -> SystemTime {
        match *self {
            Timestamp::Interval { end, .. } => end,
            Timestamp::Instant(t) => t,
        }
    }
}
