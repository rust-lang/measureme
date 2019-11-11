use crate::query_data::{QueryData, QueryDataDiff, Results};
use crate::signed_duration::SignedDuration;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct DiffResults {
    pub query_data: Vec<QueryDataDiff>,
    pub total_time: SignedDuration,
}

fn build_query_lookup(query_data: &[QueryData]) -> FxHashMap<&str, usize> {
    let mut lookup = FxHashMap::with_capacity_and_hasher(query_data.len(), Default::default());
    for i in 0..query_data.len() {
        lookup.insert(&query_data[i].label[..], i);
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
                (Some(b), None) => b.as_query_data_diff(),
                (None, Some(c)) => c.invert(),
                (None, None) => unreachable!(),
            }
        })
        .collect();

    query_data.sort_by(|l, r| r.self_time.duration.cmp(&l.self_time.duration));

    DiffResults {
        query_data,
        total_time: sd(change.total_time) - sd(base.total_time),
    }
}
