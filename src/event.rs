use crate::raw_event::TimestampKind;
use std::borrow::Cow;
use std::time::Instant;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Event<'a> {
    pub event_kind: Cow<'a, str>,
    pub label: Cow<'a, str>,
    pub additional_data: &'a [Cow<'a, str>],
    pub timestamp: Instant,
    pub timestamp_kind: TimestampKind,
}
