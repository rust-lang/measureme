use crate::stringtable::StringId;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum TimestampKind {
    Start = 0,
    End = 1,
    Instant = 2,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(C)]
pub struct Timestamp(u64);

impl Timestamp {
    #[inline]
    pub fn new(nanos: u64, kind: TimestampKind) -> Timestamp {
        Timestamp((nanos << 2) | kind as u64)
    }

    #[inline]
    pub fn nanos(self) -> u64 {
        self.0 >> 2
    }

    #[inline]
    pub fn kind(self) -> TimestampKind {
        match self.0 & 0b11 {
            0 => TimestampKind::Start,
            1 => TimestampKind::End,
            2 => TimestampKind::Instant,
            _ => unreachable!(),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
#[repr(C)]
pub struct RawEvent {
    pub event_kind: StringId,
    pub id: StringId,
    pub thread_id: u64,
    pub timestamp: Timestamp,
}

impl Default for RawEvent {
    fn default() -> Self {
        RawEvent {
            event_kind: StringId::reserved(0),
            id: StringId::reserved(0),
            thread_id: 0,
            timestamp: Timestamp::new(0, TimestampKind::Instant),
        }
    }
}
