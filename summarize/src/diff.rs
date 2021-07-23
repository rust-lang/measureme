use crate::query_data::{ArtifactSize, ArtifactSizeDiff, QueryData, QueryDataDiff, Results};
use crate::signed_duration::SignedDuration;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

pub fn calculate_diff(base: Results, change: Results) -> DiffResults {
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
                (Some(b), Some(c)) => c.clone() - b.clone(),
                (Some(b), None) => b.invert(),
                (None, Some(c)) => c.as_query_data_diff(),
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
                (Some(b), Some(c)) => c.clone() - b.clone(),
                (Some(_b), None) => todo!(),
                (None, Some(_c)) => todo!(),
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
