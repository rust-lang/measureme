use super::measureme_10_1_2::RawEvent;
use std::time::{Duration, SystemTime};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum EventPayload {
    Timestamp(Timestamp),
    Integer(u64),
}

impl EventPayload {
    pub fn from_raw_event(raw_event: &RawEvent, start_time: SystemTime) -> Self {
        if raw_event.is_integer() {
            Self::Integer(raw_event.value())
        } else {
            Self::Timestamp(Timestamp::from_raw_event(raw_event, start_time))
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Timestamp {
    Interval { start: SystemTime, end: SystemTime },
    Instant(SystemTime),
}

impl Timestamp {
    pub fn from_raw_event(raw_event: &RawEvent, start_time: SystemTime) -> Self {
        debug_assert!(!raw_event.is_integer());
        if raw_event.is_instant() {
            let t = start_time + Duration::from_nanos(raw_event.start_value());
            Self::Instant(t)
        } else {
            let start = start_time + Duration::from_nanos(raw_event.start_value());
            let end = start_time + Duration::from_nanos(raw_event.end_value());
            Timestamp::Interval { start, end }
        }
    }
}
