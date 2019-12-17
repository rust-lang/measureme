use crate::{Profiler, SerializationSink, StringComponent, StringId};

/// Event IDs are strings conforming to the following grammar:
///
/// ```ignore
///   <event_id> = <label> {<argument>}
///   <label> = <text>
///   <argument> = '\x1E' <text>
///   <text> = regex([^[[:cntrl:]]]+) // Anything but ASCII control characters
///  ```
///
/// This means there's always a "label", followed by an optional list of
/// arguments. Future versions my support other optional suffixes (with a tag
/// other than '\x11' after the '\x1E' separator), such as a "category".


/// The byte used to separate arguments from the label and each other.
pub const SEPARATOR_BYTE: &str = "\x1E";

pub struct EventIdBuilder<'p, S: SerializationSink> {
    profiler: &'p Profiler<S>,
}

impl<'p, S: SerializationSink> EventIdBuilder<'p, S> {
    pub fn new(profiler: &Profiler<S>) -> EventIdBuilder<'_, S> {
        EventIdBuilder { profiler }
    }

    pub fn from_label(&self, label: StringId) -> StringId {
        // Just forward the string ID, i single identifier is a valid event_id
        label
    }

    pub fn from_label_and_arg(&self, label: StringId, arg: StringId) -> StringId {
        self.profiler.alloc_string(&[
            // Label
            StringComponent::Ref(label),
            // Seperator and start tag for arg
            StringComponent::Value(SEPARATOR_BYTE),
            // Arg string id
            StringComponent::Ref(arg),
        ])
    }
}
