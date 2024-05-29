use super::StringId;

/// Event IDs are strings conforming to the following grammar:
///
/// ```ignore
///   <event_id> = <label> {<argument>}
///   <label> = <text>
///   <argument> = '\x1E' <text>
///   <text> = regex([[[:^cntrl:]][[:space:]]]+) // Anything but ASCII control characters except for whitespace.
///  ```
///
/// This means there's always a "label", followed by an optional list of
/// arguments. Future versions may support other optional suffixes (with a tag
/// other than '\x11' after the '\x1E' separator), such as a "category".

/// An `EventId` is a `StringId` with the additional guarantee that the
/// corresponding string conforms to the event_id grammar.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
#[repr(C)]
pub struct EventId(StringId);

impl EventId {
    pub const INVALID: EventId = EventId(StringId::INVALID);

    #[inline]
    pub fn to_string_id(self) -> StringId {
        self.0
    }

    /// deserialization.
    /// Create an EventId from a raw u32 value. Only used internally for
    #[inline]
    pub fn from_u32(raw_id: u32) -> EventId {
        EventId(StringId::new(raw_id))
    }
}
