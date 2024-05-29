use super::event_payload::EventPayload;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LightweightEvent {
    pub event_index: usize,
    pub thread_id: u32,
    pub payload: EventPayload,
}
