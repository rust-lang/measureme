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

mod analysis;
mod file_formats;
mod profiling_data;
mod stack_collapse;
pub mod testing_common;

pub use crate::profiling_data::{ProfilingData, ProfilingDataBuilder};
pub use crate::stack_collapse::collapse_stacks;
pub use analysis::{AnalysisResults, ArtifactSize, QueryData};
pub use decodeme::event::Event;
pub use decodeme::event_payload::{EventPayload, Timestamp};
pub use decodeme::lightweight_event::LightweightEvent;
