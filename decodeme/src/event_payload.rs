use measureme::RawEvent;
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

    /// Returns true if the time interval of `self` completely contains the
    /// time interval of `other`.
    pub fn contains(&self, other: &Self) -> bool {
        match self {
            EventPayload::Timestamp(Timestamp::Interval {
                start: self_start,
                end: self_end,
            }) => match other {
                EventPayload::Timestamp(Timestamp::Interval {
                    start: other_start,
                    end: other_end,
                }) => self_start <= other_start && other_end <= self_end,
                EventPayload::Timestamp(Timestamp::Instant(other_t)) => {
                    self_start <= other_t && other_t <= self_end
                }
                EventPayload::Integer(_) => false,
            },
            EventPayload::Timestamp(Timestamp::Instant(_)) | EventPayload::Integer(_) => false,
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        if let EventPayload::Timestamp(t) = *self {
            t.duration()
        } else {
            None
        }
    }

    pub fn is_interval(&self) -> bool {
        matches!(self, &Self::Timestamp(Timestamp::Interval { .. }))
    }

    pub fn is_instant(&self) -> bool {
        matches!(self, &Self::Timestamp(Timestamp::Instant(_)))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, &Self::Integer(_))
    }

    pub fn timestamp(&self) -> Option<Timestamp> {
        match self {
            Self::Timestamp(t) => Some(*t),
            Self::Integer(_) => None,
        }
    }

    pub fn integer(&self) -> Option<u64> {
        match self {
            Self::Timestamp(_) => None,
            Self::Integer(i) => Some(*i),
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

    pub fn contains(&self, t: SystemTime) -> bool {
        match *self {
            Timestamp::Interval { start, end } => t >= start && t < end,
            Timestamp::Instant(_) => false,
        }
    }

    pub fn is_instant(&self) -> bool {
        matches!(self, &Timestamp::Instant(_))
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

    pub fn duration(&self) -> Option<Duration> {
        if let Timestamp::Interval { start, end } = *self {
            end.duration_since(start).ok()
        } else {
            None
        }
    }
}
