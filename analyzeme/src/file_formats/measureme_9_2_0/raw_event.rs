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

    // The following 96 bits store the start and the end counter value, using
    // 48 bits for each.
    // FIXME(eddyb) s/time/count/
    pub start_time_lower: u32,
    // FIXME(eddyb) s/time/count/
    pub end_time_lower: u32,
    pub start_and_end_upper: u32,
}

/// `RawEvents` that have an end counter value with this value are instant events.
const INSTANT_COUNT_MARKER: u64 = 0xFFFF_FFFF_FFFF;

impl RawEvent {
    #[inline]
    // FIXME(eddyb) s/nanos/count/
    pub fn start_nanos(&self) -> u64 {
        self.start_time_lower as u64 | (((self.start_and_end_upper & 0xFFFF_0000) as u64) << 16)
    }

    #[inline]
    // FIXME(eddyb) s/nanos/count/
    pub fn end_nanos(&self) -> u64 {
        self.end_time_lower as u64 | (((self.start_and_end_upper & 0x0000_FFFF) as u64) << 32)
    }

    #[inline]
    pub fn is_instant(&self) -> bool {
        self.end_nanos() == INSTANT_COUNT_MARKER
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
                start_time_lower: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
                end_time_lower: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
                start_and_end_upper: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
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
            start_time_lower: 0,
            end_time_lower: 0,
            start_and_end_upper: 0,
        }
    }
}
