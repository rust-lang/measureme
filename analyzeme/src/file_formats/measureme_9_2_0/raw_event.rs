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

/// The max instant counter value we can represent with the 48 bits available.
// FIXME(eddyb) s/TIMESTAMP/COUNT/
pub const MAX_INSTANT_TIMESTAMP: u64 = 0xFFFF_FFFF_FFFF;

/// The max interval counter value we can represent with the 48 bits available.
/// The highest value is reserved for the `INSTANT_COUNT_MARKER`.
// FIXME(eddyb) s/TIMESTAMP/COUNT/
pub const MAX_INTERVAL_TIMESTAMP: u64 = INSTANT_COUNT_MARKER - 1;

impl RawEvent {
    #[inline]
    pub fn new_interval(
        event_kind: StringId,
        event_id: EventId,
        thread_id: u32,
        start_count: u64,
        end_count: u64,
    ) -> RawEvent {
        assert!(start_count <= end_count);
        assert!(end_count <= MAX_INTERVAL_TIMESTAMP);

        let start_time_lower = start_count as u32;
        let end_time_lower = end_count as u32;

        let start_time_upper = (start_count >> 16) as u32 & 0xFFFF_0000;
        let end_time_upper = (end_count >> 32) as u32;

        let start_and_end_upper = start_time_upper | end_time_upper;

        RawEvent {
            event_kind,
            event_id,
            thread_id,
            start_time_lower,
            end_time_lower,
            start_and_end_upper,
        }
    }

    #[inline]
    pub fn new_instant(
        event_kind: StringId,
        event_id: EventId,
        thread_id: u32,
        count: u64,
    ) -> RawEvent {
        assert!(count <= MAX_INSTANT_TIMESTAMP);

        let start_time_lower = count as u32;
        let end_time_lower = 0xFFFF_FFFF;

        let start_time_upper = (count >> 16) as u32;
        let start_and_end_upper = start_time_upper | 0x0000_FFFF;

        RawEvent {
            event_kind,
            event_id,
            thread_id,
            start_time_lower,
            end_time_lower,
            start_and_end_upper,
        }
    }

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
    pub fn serialize(&self, bytes: &mut [u8]) {
        assert!(bytes.len() == std::mem::size_of::<RawEvent>());

        #[cfg(target_endian = "little")]
        {
            let raw_event_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    self as *const _ as *const u8,
                    std::mem::size_of::<RawEvent>(),
                )
            };

            bytes.copy_from_slice(raw_event_bytes);
        }

        #[cfg(target_endian = "big")]
        {
            // We always emit data as little endian, which we have to do
            // manually on big endian targets.
            bytes[0..4].copy_from_slice(&self.event_kind.as_u32().to_le_bytes());
            bytes[4..8].copy_from_slice(&self.event_id.as_u32().to_le_bytes());
            bytes[8..12].copy_from_slice(&self.thread_id.to_le_bytes());
            bytes[12..16].copy_from_slice(&self.start_time_lower.to_le_bytes());
            bytes[16..20].copy_from_slice(&self.end_time_lower.to_le_bytes());
            bytes[20..24].copy_from_slice(&self.start_and_end_upper.to_le_bytes());
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_event_has_expected_size() {
        // A test case to prevent accidental regressions of RawEvent's size.
        assert_eq!(std::mem::size_of::<RawEvent>(), 24);
    }

    #[test]
    fn is_instant() {
        assert!(RawEvent::new_instant(StringId::INVALID, EventId::INVALID, 987, 0,).is_instant());

        assert!(RawEvent::new_instant(
            StringId::INVALID,
            EventId::INVALID,
            987,
            MAX_INSTANT_TIMESTAMP,
        )
        .is_instant());

        assert!(!RawEvent::new_interval(
            StringId::INVALID,
            EventId::INVALID,
            987,
            0,
            MAX_INTERVAL_TIMESTAMP,
        )
        .is_instant());
    }

    #[test]
    #[should_panic]
    fn invalid_instant_count() {
        let _ = RawEvent::new_instant(
            StringId::INVALID,
            EventId::INVALID,
            123,
            // count too large
            MAX_INSTANT_TIMESTAMP + 1,
        );
    }

    #[test]
    #[should_panic]
    fn invalid_start_count() {
        let _ = RawEvent::new_interval(
            StringId::INVALID,
            EventId::INVALID,
            123,
            // start count too large
            MAX_INTERVAL_TIMESTAMP + 1,
            MAX_INTERVAL_TIMESTAMP + 1,
        );
    }

    #[test]
    #[should_panic]
    fn invalid_end_count() {
        let _ = RawEvent::new_interval(
            StringId::INVALID,
            EventId::INVALID,
            123,
            0,
            // end count too large
            MAX_INTERVAL_TIMESTAMP + 3,
        );
    }

    #[test]
    #[should_panic]
    fn invalid_end_count2() {
        let _ = RawEvent::new_interval(
            StringId::INVALID,
            EventId::INVALID,
            123,
            0,
            INSTANT_COUNT_MARKER,
        );
    }

    #[test]
    #[should_panic]
    fn start_greater_than_end_count() {
        let _ = RawEvent::new_interval(
            StringId::INVALID,
            EventId::INVALID,
            123,
            // start count greater than end count
            1,
            0,
        );
    }

    #[test]
    fn start_equal_to_end_count() {
        // This is allowed, make sure we don't panic
        let _ = RawEvent::new_interval(StringId::INVALID, EventId::INVALID, 123, 1, 1);
    }

    #[test]
    fn interval_count_decoding() {
        // Check the upper limits
        let e = RawEvent::new_interval(
            StringId::INVALID,
            EventId::INVALID,
            1234,
            MAX_INTERVAL_TIMESTAMP,
            MAX_INTERVAL_TIMESTAMP,
        );

        assert_eq!(e.start_nanos(), MAX_INTERVAL_TIMESTAMP);
        assert_eq!(e.end_nanos(), MAX_INTERVAL_TIMESTAMP);

        // Check the lower limits
        let e = RawEvent::new_interval(StringId::INVALID, EventId::INVALID, 1234, 0, 0);

        assert_eq!(e.start_nanos(), 0);
        assert_eq!(e.end_nanos(), 0);

        // Check that end does not bleed into start
        let e = RawEvent::new_interval(
            StringId::INVALID,
            EventId::INVALID,
            1234,
            0,
            MAX_INTERVAL_TIMESTAMP,
        );

        assert_eq!(e.start_nanos(), 0);
        assert_eq!(e.end_nanos(), MAX_INTERVAL_TIMESTAMP);

        // Test some random values
        let e = RawEvent::new_interval(
            StringId::INVALID,
            EventId::INVALID,
            1234,
            0x1234567890,
            0x1234567890A,
        );

        assert_eq!(e.start_nanos(), 0x1234567890);
        assert_eq!(e.end_nanos(), 0x1234567890A);
    }

    #[test]
    fn instant_count_decoding() {
        assert_eq!(
            RawEvent::new_instant(StringId::INVALID, EventId::INVALID, 987, 0,).start_nanos(),
            0
        );

        assert_eq!(
            RawEvent::new_instant(
                StringId::INVALID,
                EventId::INVALID,
                987,
                MAX_INSTANT_TIMESTAMP,
            )
            .start_nanos(),
            MAX_INSTANT_TIMESTAMP
        );
    }
}
