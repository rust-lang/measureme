use super::measureme_9_2_0::RawEvent;
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
}
