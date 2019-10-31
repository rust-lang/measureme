use crate::stringtable::StringId;

#[derive(Eq, PartialEq, Debug)]
#[repr(C)]
pub struct RawEvent {
    pub event_kind: StringId,
    pub event_id: StringId,
    pub thread_id: u64,
    pub start_ns: u64,
    pub end_ns: u64,
}

impl RawEvent {
    #[inline]
    pub fn new_interval(
        event_kind: StringId,
        event_id: StringId,
        thread_id: u64,
        start_ns: u64,
        end_ns: u64,
    ) -> RawEvent {
        RawEvent {
            event_kind,
            event_id,
            thread_id,
            start_ns,
            end_ns,
        }
    }

    #[inline]
    pub fn new_instant(
        event_kind: StringId,
        event_id: StringId,
        thread_id: u64,
        timestamp_ns: u64,
    ) -> RawEvent {
        RawEvent {
            event_kind,
            event_id,
            thread_id,
            start_ns: timestamp_ns,
            end_ns: std::u64::MAX,
        }
    }
}

impl Default for RawEvent {
    fn default() -> Self {
        RawEvent {
            event_kind: StringId::reserved(0),
            event_id: StringId::reserved(0),
            thread_id: 0,
            start_ns: 0,
            end_ns: 0,
        }
    }
}
