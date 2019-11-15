use crate::signed_duration::SignedDuration;
use serde::{Deserialize, Serialize};
use std::ops::Sub;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone)]
pub struct QueryData {
    pub label: String,
    pub self_time: Duration,
    pub number_of_cache_misses: usize,
    pub number_of_cache_hits: usize,
    pub invocation_count: usize,
    pub blocked_time: Duration,
    pub incremental_load_time: Duration,
}

impl QueryData {
    pub fn new(label: String) -> QueryData {
        QueryData {
            label,
            self_time: Duration::from_nanos(0),
            number_of_cache_misses: 0,
            number_of_cache_hits: 0,
            invocation_count: 0,
            blocked_time: Duration::from_nanos(0),
            incremental_load_time: Duration::from_nanos(0),
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
            self_time: invert(self.self_time),
            self_time_change: -100.0,
            number_of_cache_misses: -(self.number_of_cache_misses as i64),
            number_of_cache_hits: -(self.number_of_cache_hits as i64),
            invocation_count: -(self.invocation_count as i64),
            blocked_time: invert(self.blocked_time),
            incremental_load_time: invert(self.incremental_load_time),
        }
    }

    pub fn as_query_data_diff(&self) -> QueryDataDiff {
        QueryDataDiff {
            label: self.label.clone(),
            self_time: self.self_time.into(),
            self_time_change: std::f64::INFINITY,
            number_of_cache_misses: self.number_of_cache_misses as i64,
            number_of_cache_hits: self.number_of_cache_hits as i64,
            invocation_count: self.invocation_count as i64,
            blocked_time: self.blocked_time.into(),
            incremental_load_time: self.incremental_load_time.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct QueryDataDiff {
    pub label: String,
    pub self_time: SignedDuration,
    pub self_time_change: f64,
    pub number_of_cache_misses: i64,
    pub number_of_cache_hits: i64,
    pub invocation_count: i64,
    pub blocked_time: SignedDuration,
    pub incremental_load_time: SignedDuration,
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
            self_time: sd(self.self_time) - sd(rhs.self_time),
            self_time_change: percentage_change(rhs.self_time, self.self_time),
            number_of_cache_misses: i(self.number_of_cache_misses) - i(rhs.number_of_cache_misses),
            number_of_cache_hits: i(self.number_of_cache_hits) - i(rhs.number_of_cache_hits),
            invocation_count: i(self.invocation_count) - i(rhs.invocation_count),
            blocked_time: sd(self.blocked_time) - sd(rhs.blocked_time),
            incremental_load_time: sd(self.incremental_load_time) - sd(rhs.incremental_load_time),
        }
    }
}

fn percentage_change(base: Duration, change: Duration) -> f64 {
    let self_time_nanos = change.as_nanos() as i128 - base.as_nanos() as i128;
    self_time_nanos as f64 / base.as_nanos() as f64 * 100.0
}

#[derive(Serialize, Deserialize)]
pub struct Results {
    pub query_data: Vec<QueryData>,
    pub total_time: Duration,
}

// For now this is only needed for tests it seems
#[cfg(test)]
impl Results {
    pub fn query_data_by_label(&self, label: &str) -> &QueryData {
        self.query_data.iter().find(|qd| qd.label == label).unwrap()
    }
}
