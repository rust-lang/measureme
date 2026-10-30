//! This module contains functionality specific to to the measureme integration with rustc

pub const QUERY_EVENT_KIND: &str = "Query";

pub const GENERIC_ACTIVITY_EVENT_KIND: &str = "GenericActivity";

pub const INCREMENTAL_LOAD_RESULT_EVENT_KIND: &str = "IncrementalLoadResult";

pub const INCREMENTAL_RESULT_HASHING_EVENT_KIND: &str = "IncrementalResultHashing";

pub const QUERY_BLOCKED_EVENT_KIND: &str = "QueryBlocked";

pub const QUERY_CACHE_HIT_EVENT_KIND: &str = "QueryCacheHit";

/// Aggregated count of query cache hits, stored as an integer event.
pub const QUERY_CACHE_HIT_COUNT_EVENT_KIND: &str = "QueryCacheHitCount";

pub const ARTIFACT_SIZE_EVENT_KIND: &str = "ArtifactSize";

/// Resident set size of the compiler process in bytes, stored as an integer event.
///
/// rustc records one of these events when a generic activity starts and one when it
/// ends, both with the event id of that activity. Consumers aggregate the samples per
/// label; see `analyzeme::ResidentSetSize`.
pub const RESIDENT_SET_SIZE_EVENT_KIND: &str = "ResidentSetSize";
