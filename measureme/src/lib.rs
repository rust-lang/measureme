mod event;
mod file_header;
mod file_serialization_sink;
mod mmap_serialization_sink;
mod profiler;
mod profiling_data;
mod raw_event;
mod serialization;
mod stringtable;

pub mod rustc;
pub mod testing_common;

pub use crate::event::Event;
pub use crate::file_serialization_sink::FileSerializationSink;
pub use crate::mmap_serialization_sink::MmapSerializationSink;
pub use crate::profiler::{Profiler, ProfilerFiles};
pub use crate::profiling_data::{ProfilingData, MatchingEvent};
pub use crate::raw_event::{RawEvent, Timestamp, TimestampKind};
pub use crate::serialization::{Addr, SerializationSink};
pub use crate::stringtable::{
    SerializableString, StringId, StringRef, StringTable, StringTableBuilder,
};
