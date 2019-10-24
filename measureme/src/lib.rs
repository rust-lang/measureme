//! This crate provides a library for high-performance event tracing which is used by the Rust compiler's unstable `-Z self-profile` feature.
//!
//! There are two main parts to this library:
//!   - Writing event trace files
//!   - Reading event trace files
//!
//! The output of a tracing session will be three files:
//!   1. A `.events` file which contains all of the traced events.
//!   2. A `.string_data` file which contains all the strings referenced by events.
//!   3. A `.string_index` file which maps `StringId` values to offsets into the `.string_data` file.
//!
//! # Writing event trace files
//!
//! The main entry point for writing event trace files is the [`Profiler`] struct.
//!
//! To create a [`Profiler`], call the [`Profiler::new()`] function and provide a `Path` with the directory and file name for the trace files.
//!
//! To record an event, call the [`Profiler::record_event()`] method, passing a few arguments:
//!   - `event_kind`: a [`StringId`] which assigns an arbitrary category to the event
//!   - `event_id`: a [`StringId`] which specifies the name of the event
//!   - `thread_id`: a `u64` id of the thread which is recording this event
//!   - `timestamp_kind`: a [`TimestampKind`] which specifies how this event should be treated by `measureme` tooling
//!
//! Alternatively, events can also be recorded via the [`Profiler::start_recording_interval_event()`] method. This
//! method records a "start" event and returns a `TimingGuard` object that will automatically record
//! the corresponding "end" event when it is dropped.
//!
//! To create a [`StringId`], call one of the string allocation methods:
//!   - [`Profiler::alloc_string()`]: allocates a string and returns the [`StringId`] that refers to it
//!   - [`Profiler::alloc_string_with_reserved_id()`]: allocates a string using the specified [`StringId`].
//!     It is up to the caller to make sure the specified [`StringId`] hasn't already been used.
//!
//! # Reading event trace files
//!
//! The main entry point for reading trace files is the [`ProfilingData`] struct.
//!
//! To create a [`ProfilingData`], call the [`ProfilingData::new()`] function and provide a `Path` with the directory and file name for the trace files.
//!
//! To retrieve an `Iterator` of all of the events in the file, call the [`ProfilingData::iter()`] method.
//!
//! To retrieve an `Iterator` of only matching start/stop events, call the [`ProfilingData::iter_matching_events()`] method.
//!
//! [`Profiler`]: struct.Profiler.html
//! [`Profiler::alloc_string()`]: struct.Profiler.html#method.alloc_string
//! [`Profiler::alloc_string_with_reserved_id()`]: struct.Profiler.html#method.alloc_string_with_reserved_id
//! [`Profiler::new()`]: struct.Profiler.html#method.new
//! [`Profiler::record_event()`]: struct.Profiler.html#method.record_event
//! [`Profiler::start_recording_interval_event()`]: struct.Profiler.html#method.start_recording_interval_event
//! [`ProfilingData`]: struct.ProfilingData.html
//! [`ProfilingData::iter()`]: struct.ProfilingData.html#method.iter
//! [`ProfilingData::iter_matching_events()`]: struct.ProfilingData.html#method.iter_matching_events
//! [`StringId`]: struct.StringId.html
//! [`TimestampKind`]: enum.TimestampKind.html

#![deny(warnings)]

mod event;
mod file_header;
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
mod file_serialization_sink;
#[cfg(not(target_arch = "wasm32"))]
mod mmap_serialization_sink;
mod profiler;
mod profiling_data;
mod raw_event;
mod serialization;
mod stringtable;

pub mod rustc;
pub mod testing_common;

pub use crate::event::Event;
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub use crate::file_serialization_sink::FileSerializationSink;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::mmap_serialization_sink::MmapSerializationSink;
pub use crate::profiler::{Profiler, ProfilerFiles, TimingGuard};
pub use crate::profiling_data::{MatchingEvent, ProfilingData};
pub use crate::raw_event::{RawEvent, Timestamp, TimestampKind};
pub use crate::serialization::{Addr, SerializationSink};
pub use crate::stringtable::{
    SerializableString, StringId, StringRef, StringTable, StringTableBuilder,
};
