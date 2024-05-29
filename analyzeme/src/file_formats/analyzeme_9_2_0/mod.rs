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
mod lightweight_event;
mod profiling_data;
mod stack_collapse;
mod stringtable;
pub mod testing_common;
mod timestamp;

pub use self::event::Event;
pub use self::lightweight_event::LightweightEvent;
pub use self::profiling_data::{ProfilingData, ProfilingDataBuilder};
pub use self::stack_collapse::collapse_stacks;
pub use self::stringtable::{StringRef, StringTable};
pub use self::timestamp::Timestamp;

// These are re-exported just for being used in v10.0.0 when supporting
// old file formats. Starting in v10.0.0 these re-exports will become
// part of the `decodeme` crate.
pub use super::measureme_9_2_0::file_header::CURRENT_FILE_FORMAT_VERSION;
pub use super::measureme_9_2_0::RawEvent;
use super::measureme_9_2_0;
