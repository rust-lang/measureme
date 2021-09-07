use crate::signed_duration::SignedDuration;
use serde::{Deserialize, Serialize};
use std::ops::Sub;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueryData {
    pub label: String,
    pub time: Duration,
    pub self_time: Duration,
    pub number_of_cache_misses: usize,
    pub number_of_cache_hits: usize,
    pub invocation_count: usize,
    pub blocked_time: Duration,
    pub incremental_load_time: Duration,
    pub incremental_hashing_time: Duration,
}

impl QueryData {
    pub fn new(label: String) -> QueryData {
        QueryData {
            label,
            time: Duration::from_nanos(0),
            self_time: Duration::from_nanos(0),
            number_of_cache_misses: 0,
            number_of_cache_hits: 0,
            invocation_count: 0,
            blocked_time: Duration::from_nanos(0),
            incremental_load_time: Duration::from_nanos(0),
            incremental_hashing_time: Duration::from_nanos(0),
        }
    }

    pub fn invert(&self) -> QueryDataDiff {
        fn invert(d: Duration) -> SignedDuration {
            SignedDuration {
                duration: d,
                is_positive: false,
            }
        }

        QueryDataDiff {
            label: self.label.clone(),
            time: invert(self.time),
            time_change: -100.0,
            self_time: invert(self.self_time),
            self_time_change: -100.0,
            number_of_cache_misses: -(self.number_of_cache_misses as i64),
            number_of_cache_hits: -(self.number_of_cache_hits as i64),
            invocation_count: -(self.invocation_count as i64),
            blocked_time: invert(self.blocked_time),
            incremental_load_time: invert(self.incremental_load_time),
            incremental_hashing_time: invert(self.incremental_hashing_time),
        }
    }

    pub fn as_query_data_diff(&self) -> QueryDataDiff {
        QueryDataDiff {
            label: self.label.clone(),
            time: self.time.into(),
            time_change: std::f64::INFINITY,
            self_time: self.self_time.into(),
            self_time_change: std::f64::INFINITY,
            number_of_cache_misses: self.number_of_cache_misses as i64,
            number_of_cache_hits: self.number_of_cache_hits as i64,
            invocation_count: self.invocation_count as i64,
            blocked_time: self.blocked_time.into(),
            incremental_load_time: self.incremental_load_time.into(),
            incremental_hashing_time: self.incremental_hashing_time.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct QueryDataDiff {
    pub label: String,
    pub time: SignedDuration,
    pub time_change: f64,
    pub self_time: SignedDuration,
    pub self_time_change: f64,
    pub number_of_cache_misses: i64,
    pub number_of_cache_hits: i64,
    pub invocation_count: i64,
    pub blocked_time: SignedDuration,
    pub incremental_load_time: SignedDuration,
    pub incremental_hashing_time: SignedDuration,
}

impl Sub for QueryData {
    type Output = QueryDataDiff;

    fn sub(self, rhs: QueryData) -> QueryDataDiff {
        #[inline(always)]
        fn sd(d: Duration) -> SignedDuration {
            d.into()
        }

        #[inline(always)]
        fn i(u: usize) -> i64 {
            u as i64
        }

        QueryDataDiff {
            label: self.label,
            time: sd(self.time) - sd(rhs.time),
            time_change: percentage_change(rhs.time, self.time),
            self_time: sd(self.self_time) - sd(rhs.self_time),
            self_time_change: percentage_change(rhs.self_time, self.self_time),
            number_of_cache_misses: i(self.number_of_cache_misses) - i(rhs.number_of_cache_misses),
            number_of_cache_hits: i(self.number_of_cache_hits) - i(rhs.number_of_cache_hits),
            invocation_count: i(self.invocation_count) - i(rhs.invocation_count),
            blocked_time: sd(self.blocked_time) - sd(rhs.blocked_time),
            incremental_load_time: sd(self.incremental_load_time) - sd(rhs.incremental_load_time),
            incremental_hashing_time: sd(self.incremental_hashing_time)
                - sd(rhs.incremental_hashing_time),
        }
    }
}

fn percentage_change(base: Duration, change: Duration) -> f64 {
    let nanos = change.as_nanos() as i128 - base.as_nanos() as i128;
    nanos as f64 / base.as_nanos() as f64 * 100.0
}

#[derive(Serialize, Deserialize)]
pub struct Results {
    pub query_data: Vec<QueryData>,
    pub artifact_sizes: Vec<ArtifactSize>,
    pub total_time: Duration,
}

// For now this is only needed for tests it seems
#[cfg(test)]
impl Results {
    pub fn query_data_by_label(&self, label: &str) -> &QueryData {
        self.query_data.iter().find(|qd| qd.label == label).unwrap()
    }

    pub fn artifact_size_by_label(&self, label: &str) -> &ArtifactSize {
        self.artifact_sizes
            .iter()
            .find(|qd| qd.label == label)
            .unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArtifactSize {
    pub label: String,
    pub value: u64,
}

impl ArtifactSize {
    pub fn new(label: String) -> Self {
        Self { label, value: 0 }
    }

    pub fn invert(&self) -> ArtifactSizeDiff {
        ArtifactSizeDiff {
            label: self.label.clone(),
            size_change: -(self.value as i64),
        }
    }

    pub fn as_artifact_size_diff(&self) -> ArtifactSizeDiff {
        ArtifactSizeDiff {
            label: self.label.clone(),
            size_change: self.value as i64,
        }
    }

    pub(crate) fn add_value(&mut self, value: u64) {
        self.value += value;
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArtifactSizeDiff {
    pub label: String,
    pub size_change: i64,
}

impl Sub for ArtifactSize {
    type Output = ArtifactSizeDiff;

    fn sub(self, rhs: ArtifactSize) -> ArtifactSizeDiff {
        ArtifactSizeDiff {
            label: self.label,
            size_change: self.value as i64 - rhs.value as i64,
        }
    }
}
