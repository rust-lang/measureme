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
//!
//! [`ProfilingData`]: struct.ProfilingData.html
//! [`ProfilingData::iter()`]: struct.ProfilingData.html#method.iter

mod event;
mod profiling_data;
mod stack_collapse;
pub mod testing_common;

pub use crate::event::{Event, Timestamp};
pub use crate::profiling_data::{ProfilingData, ProfilingDataBuilder};
pub use crate::stack_collapse::collapse_stacks;
