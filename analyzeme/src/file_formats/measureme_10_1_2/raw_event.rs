use super::event_id::EventId;
use super::stringtable::StringId;
#[cfg(target_endian = "big")]
use std::convert::TryInto;

/// `RawEvent` is how events are stored on-disk. If you change this struct,
/// make sure that you increment `file_header::CURRENT_FILE_FORMAT_VERSION`.
#[derive(Eq, PartialEq, Debug)]
#[repr(C)]
pub struct RawEvent {
    pub event_kind: StringId,
    pub event_id: EventId,
    pub thread_id: u32,

    // The following 96 bits store the payload values, using
    // 48 bits for each.
    // Interval:
    // Payload 1 is start value and payload 2 is end value
    // SSSSSSSSSSSSSSSSEEEEEEEEEEEEEEEESSSSSSSEEEEEEEEE
    // [payload1_lower][payload2_lower][payloads_upper]
    // Instant:
    // Payload2 is 0xFFFF_FFFF_FFFF
    // VVVVVVVVVVVVVVVV1111111111111111VVVVVVV11111111
    // [payload1_lower][payload2_lower][payloads_upper]
    // Integer:
    // Payload2 is 0xFFFF_FFFF_FFFE
    // VVVVVVVVVVVVVVVV1111111111111111VVVVVVV11111110
    // [payload1_lower][payload2_lower][payloads_upper]
    pub payload1_lower: u32,
    pub payload2_lower: u32,
    pub payloads_upper: u32,
}

/// `RawEvents` that have a payload 2 value with this value are instant events.
const INSTANT_MARKER: u64 = 0xFFFF_FFFF_FFFF;
/// `RawEvents` that have a payload 2 value with this value are integer events.
const INTEGER_MARKER: u64 = INSTANT_MARKER - 1;

impl RawEvent {

    /// The start value assuming self is an interval
    #[inline]
    pub fn start_value(&self) -> u64 {
        self.payload1_lower as u64 | (((self.payloads_upper & 0xFFFF_0000) as u64) << 16)
    }

    /// The end value assuming self is an interval
    #[inline]
    pub fn end_value(&self) -> u64 {
        self.payload2_lower as u64 | (((self.payloads_upper & 0x0000_FFFF) as u64) << 32)
    }

    /// The value assuming self is an interval or integer.
    #[inline]
    pub fn value(&self) -> u64 {
        self.payload1_lower as u64 | (((self.payloads_upper & 0xFFFF_0000) as u64) << 16)
    }

    #[inline]
    pub fn is_instant(&self) -> bool {
        self.end_value() == INSTANT_MARKER
    }

    #[inline]
    pub fn is_integer(&self) -> bool {
        self.end_value() == INTEGER_MARKER
    }

    #[inline]
    pub fn deserialize(bytes: &[u8]) -> RawEvent {
        assert!(bytes.len() == std::mem::size_of::<RawEvent>());

        #[cfg(target_endian = "little")]
        {
            let mut raw_event = RawEvent::default();
            unsafe {
                let raw_event = std::slice::from_raw_parts_mut(
                    &mut raw_event as *mut RawEvent as *mut u8,
                    std::mem::size_of::<RawEvent>(),
                );
                raw_event.copy_from_slice(bytes);
            };
            raw_event
        }

        #[cfg(target_endian = "big")]
        {
            RawEvent {
                event_kind: StringId::new(u32::from_le_bytes(bytes[0..4].try_into().unwrap())),
                event_id: EventId::from_u32(u32::from_le_bytes(bytes[4..8].try_into().unwrap())),
                thread_id: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
                payload1_lower: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
                payload2_lower: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
                payloads_upper: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
            }
        }
    }
}

impl Default for RawEvent {
    fn default() -> Self {
        RawEvent {
            event_kind: StringId::INVALID,
            event_id: EventId::INVALID,
            thread_id: 0,
            payload1_lower: 0,
            payload2_lower: 0,
            payloads_upper: 0,
        }
    }
}
