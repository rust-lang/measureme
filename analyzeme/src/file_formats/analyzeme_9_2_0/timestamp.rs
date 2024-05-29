use measureme_9_2_0::RawEvent;
use std::time::{Duration, SystemTime};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Timestamp {
    Interval { start: SystemTime, end: SystemTime },
    Instant(SystemTime),
}

impl Timestamp {
    pub fn from_raw_event(raw_event: &RawEvent, start_time: SystemTime) -> Timestamp {
        if raw_event.is_instant() {
            let t = start_time + Duration::from_nanos(raw_event.start_nanos());
            Timestamp::Instant(t)
        } else {
            let start = start_time + Duration::from_nanos(raw_event.start_nanos());
            let end = start_time + Duration::from_nanos(raw_event.end_nanos());
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
