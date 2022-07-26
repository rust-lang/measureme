use analyzeme::{AnalysisResults, ArtifactSize, QueryData};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct DiffResults {
    pub query_data: Vec<QueryDataDiff>,
    pub artifact_sizes: Vec<ArtifactSizeDiff>,
    pub total_time: SignedDuration,
}

fn build_query_lookup(query_data: &[QueryData]) -> FxHashMap<&str, usize> {
    let mut lookup = FxHashMap::with_capacity_and_hasher(query_data.len(), Default::default());
    for (i, data) in query_data.iter().enumerate() {
        lookup.insert(&data.label[..], i);
    }

    lookup
}

fn build_artifact_lookup(artifact_sizes: &[ArtifactSize]) -> FxHashMap<&str, usize> {
    let mut lookup = FxHashMap::with_capacity_and_hasher(artifact_sizes.len(), Default::default());
    for (i, data) in artifact_sizes.iter().enumerate() {
        lookup.insert(&data.label[..], i);
    }

    lookup
}

pub fn calculate_diff(base: AnalysisResults, change: AnalysisResults) -> DiffResults {
    #[inline]
    fn sd(d: Duration) -> SignedDuration {
        d.into()
    }

    let base_data = build_query_lookup(&base.query_data);
    let change_data = build_query_lookup(&change.query_data);

    let mut all_labels = FxHashSet::with_capacity_and_hasher(
        base.query_data.len() + change.query_data.len(),
        Default::default(),
    );
    for query_data in base.query_data.iter().chain(&change.query_data) {
        all_labels.insert(&query_data.label[..]);
    }

    let mut query_data: Vec<_> = all_labels
        .iter()
        .map(|l| {
            let b = base_data.get(l).map(|i| &base.query_data[*i]);
            let c = change_data.get(l).map(|i| &change.query_data[*i]);

            match (b, c) {
                (Some(b), Some(c)) => QueryDataDiff::sub(c.clone(), b.clone()),
                (Some(b), None) => QueryDataDiff::invert_query_data(b),
                (None, Some(c)) => QueryDataDiff::query_data_as_diff(c),
                (None, None) => unreachable!(),
            }
        })
        .collect();

    query_data.sort_by(|l, r| r.self_time.duration.cmp(&l.self_time.duration));

    let base_data = build_artifact_lookup(&base.artifact_sizes);
    let change_data = build_artifact_lookup(&change.artifact_sizes);
    let all_labels = base
        .artifact_sizes
        .iter()
        .chain(&change.artifact_sizes)
        .map(|a| a.label.as_str())
        .collect::<HashSet<_>>();
    let mut artifact_sizes: Vec<_> = all_labels
        .iter()
        .map(|l| {
            let b = base_data.get(l).map(|i| &base.artifact_sizes[*i]);
            let c = change_data.get(l).map(|i| &change.artifact_sizes[*i]);

            match (b, c) {
                (Some(b), Some(c)) => ArtifactSizeDiff::sub(c.clone(), b.clone()),
                (Some(b), None) => ArtifactSizeDiff::invert_artifact_size(b),
                (None, Some(c)) => ArtifactSizeDiff::artifact_size_as_diff(c),
                (None, None) => unreachable!(),
            }
        })
        .collect();
    artifact_sizes.sort_by(|l, r| r.size_change.cmp(&l.size_change));

    DiffResults {
        query_data,
        artifact_sizes,
        total_time: sd(change.total_time) - sd(base.total_time),
    }
}

/// The diff between two `QueryData`
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

impl QueryDataDiff {
    fn sub(lhs: QueryData, rhs: QueryData) -> QueryDataDiff {
        #[inline(always)]
        fn sd(d: Duration) -> SignedDuration {
            d.into()
        }

        #[inline(always)]
        fn i(u: usize) -> i64 {
            u as i64
        }

        fn percentage_change(base: Duration, change: Duration) -> f64 {
            let nanos = change.as_nanos() as i128 - base.as_nanos() as i128;
            nanos as f64 / base.as_nanos() as f64 * 100.0
        }

        QueryDataDiff {
            label: lhs.label,
            time: sd(lhs.time) - sd(rhs.time),
            time_change: percentage_change(rhs.time, lhs.time),
            self_time: sd(lhs.self_time) - sd(rhs.self_time),
            self_time_change: percentage_change(rhs.self_time, lhs.self_time),
            number_of_cache_misses: i(lhs.number_of_cache_misses) - i(rhs.number_of_cache_misses),
            number_of_cache_hits: i(lhs.number_of_cache_hits) - i(rhs.number_of_cache_hits),
            invocation_count: i(lhs.invocation_count) - i(rhs.invocation_count),
            blocked_time: sd(lhs.blocked_time) - sd(rhs.blocked_time),
            incremental_load_time: sd(lhs.incremental_load_time) - sd(rhs.incremental_load_time),
            incremental_hashing_time: sd(lhs.incremental_hashing_time)
                - sd(rhs.incremental_hashing_time),
        }
    }

    pub fn invert_query_data(data: &QueryData) -> QueryDataDiff {
        fn invert(d: Duration) -> SignedDuration {
            SignedDuration {
                duration: d,
                is_positive: false,
            }
        }

        QueryDataDiff {
            label: data.label.clone(),
            time: invert(data.time),
            time_change: -100.0,
            self_time: invert(data.self_time),
            self_time_change: -100.0,
            number_of_cache_misses: -(data.number_of_cache_misses as i64),
            number_of_cache_hits: -(data.number_of_cache_hits as i64),
            invocation_count: -(data.invocation_count as i64),
            blocked_time: invert(data.blocked_time),
            incremental_load_time: invert(data.incremental_load_time),
            incremental_hashing_time: invert(data.incremental_hashing_time),
        }
    }

    pub fn query_data_as_diff(data: &QueryData) -> QueryDataDiff {
        QueryDataDiff {
            label: data.label.clone(),
            time: data.time.into(),
            time_change: std::f64::INFINITY,
            self_time: data.self_time.into(),
            self_time_change: std::f64::INFINITY,
            number_of_cache_misses: data.number_of_cache_misses as i64,
            number_of_cache_hits: data.number_of_cache_hits as i64,
            invocation_count: data.invocation_count as i64,
            blocked_time: data.blocked_time.into(),
            incremental_load_time: data.incremental_load_time.into(),
            incremental_hashing_time: data.incremental_hashing_time.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArtifactSizeDiff {
    pub label: String,
    pub size_change: i64,
}

impl ArtifactSizeDiff {
    pub fn invert_artifact_size(size: &ArtifactSize) -> ArtifactSizeDiff {
        ArtifactSizeDiff {
            label: size.label.clone(),
            size_change: -(size.value as i64),
        }
    }

    pub fn artifact_size_as_diff(size: &ArtifactSize) -> ArtifactSizeDiff {
        ArtifactSizeDiff {
            label: size.label.clone(),
            size_change: size.value as i64,
        }
    }
    fn sub(lhs: ArtifactSize, rhs: ArtifactSize) -> ArtifactSizeDiff {
        ArtifactSizeDiff {
            label: lhs.label,
            size_change: lhs.value as i64 - rhs.value as i64,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
pub struct SignedDuration {
    pub duration: Duration,
    pub is_positive: bool,
}

impl SignedDuration {
    pub fn as_nanos(&self) -> i128 {
        let sign = if self.is_positive { 1 } else { -1 };

        sign * (self.duration.as_nanos() as i128)
    }

    pub fn from_nanos(nanos: i128) -> SignedDuration {
        let is_positive = nanos >= 0;

        SignedDuration {
            duration: Duration::from_nanos(nanos.abs() as u64),
            is_positive,
        }
    }
}

impl From<Duration> for SignedDuration {
    fn from(d: Duration) -> SignedDuration {
        SignedDuration {
            duration: d,
            is_positive: true,
        }
    }
}

impl Ord for SignedDuration {
    fn cmp(&self, other: &SignedDuration) -> Ordering {
        self.as_nanos().cmp(&other.as_nanos())
    }
}

impl PartialOrd for SignedDuration {
    fn partial_cmp(&self, other: &SignedDuration) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::ops::Sub for SignedDuration {
    type Output = SignedDuration;

    fn sub(self, rhs: SignedDuration) -> SignedDuration {
        SignedDuration::from_nanos(self.as_nanos() - rhs.as_nanos())
    }
}

impl fmt::Debug for SignedDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_positive {
            write!(f, "+")?;
        } else {
            write!(f, "-")?;
        }

        write!(f, "{:?}", self.duration)
    }
}

#[cfg(test)]
mod test {
    use super::SignedDuration;
    use std::time::Duration;

    #[test]
    fn op_subtract() {
        let zero_d = Duration::from_nanos(0);
        let one_d = Duration::from_nanos(1);
        let two_d = Duration::from_nanos(2);

        let zero_sd = SignedDuration::from(zero_d);
        let one_sd = SignedDuration::from(one_d);
        let neg_one_sd = SignedDuration {
            duration: one_d,
            is_positive: false,
        };
        let two_sd = SignedDuration::from(two_d);
        let neg_two_sd = SignedDuration {
            duration: two_d,
            is_positive: false,
        };

        assert_eq!(zero_d, zero_sd.duration);
        assert_eq!(true, zero_sd.is_positive);

        assert_eq!(zero_sd, zero_sd - zero_sd);

        assert_eq!(one_d, one_sd.duration);
        assert_eq!(true, one_sd.is_positive);

        assert_eq!(one_sd, one_sd - zero_sd);

        assert_eq!(one_d, neg_one_sd.duration);
        assert_eq!(false, neg_one_sd.is_positive);

        assert_eq!(neg_one_sd, neg_one_sd - zero_sd);

        assert_eq!(zero_sd, one_sd - one_sd);

        assert_eq!(one_sd, two_sd - one_sd);

        assert_eq!(neg_one_sd, one_sd - two_sd);

        assert_eq!(neg_two_sd, neg_one_sd - one_sd);

        assert_eq!(zero_sd, neg_one_sd - neg_one_sd);
    }
}
