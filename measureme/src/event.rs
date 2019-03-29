use crate::raw_event::TimestampKind;
use std::borrow::Cow;
use std::time::SystemTime;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Event<'a> {
    pub event_kind: Cow<'a, str>,
    pub label: Cow<'a, str>,
    pub additional_data: &'a [Cow<'a, str>],
    pub timestamp: SystemTime,
    pub timestamp_kind: TimestampKind,
    pub thread_id: u64,
}
