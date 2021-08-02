//! This crate provides a library with common functionality for measureme tools
//!
//! # Reading event trace files
//!
//! The main entry point for reading trace files is the [`ProfilingData`] struct.
//!
//! To create a [`ProfilingData`], call the [`ProfilingData::new()`] function and
//! provide a `Path` with the directory and file name for the trace files.
//!
//! To retrieve an `Iterator` of all of the events in the file,
//! call the [`ProfilingData::iter()`] method.

mod event;
mod event_payload;
mod lightweight_event;
mod profiling_data;
mod stack_collapse;
mod stringtable;
pub mod testing_common;

pub use crate::event::Event;
pub use crate::event_payload::{EventPayload, Timestamp};
pub use crate::lightweight_event::LightweightEvent;
pub use crate::profiling_data::{ProfilingData, ProfilingDataBuilder};
pub use crate::stack_collapse::collapse_stacks;
pub use crate::stringtable::{StringRef, StringTable};
