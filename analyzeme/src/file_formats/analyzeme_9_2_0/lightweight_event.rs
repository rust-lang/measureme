use super::profiling_data::ProfilingData;
use super::timestamp::Timestamp;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct LightweightEvent<'a> {
    pub data: &'a ProfilingData,
    pub event_index: usize,
    pub thread_id: u32,
    pub timestamp: Timestamp,
}

impl<'a> PartialEq for LightweightEvent<'a> {
    fn eq(&self, other: &LightweightEvent<'a>) -> bool {
        let LightweightEvent {
            data,
            event_index,
            thread_id,
            timestamp,
        } = *self;

        let LightweightEvent {
            data: other_data,
            event_index: other_event_index,
            thread_id: other_thread_id,
            timestamp: other_timestamp,
        } = *other;

        std::ptr::eq(data, other_data)
            && event_index == other_event_index
            && thread_id == other_thread_id
            && timestamp == other_timestamp
    }
}

impl<'a> Eq for LightweightEvent<'a> {}

impl<'a> Hash for LightweightEvent<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let LightweightEvent {
            data,
            event_index,
            thread_id,
            timestamp,
        } = *self;

        std::ptr::hash(data, state);
        event_index.hash(state);
        thread_id.hash(state);
        timestamp.hash(state);
    }
}
