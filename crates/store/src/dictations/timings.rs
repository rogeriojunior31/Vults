//! Milliseconds for delivered dictations; an unmeasured or unused stage stays unknown.

use rusqlite::Row;
use serde::Serialize;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Timings {
    pub total_ms: Option<i64>,
    pub stt_ms: Option<i64>,
    pub pipeline_ms: Option<i64>,
    pub cleanup_ms: Option<i64>,
    pub insert_ms: Option<i64>,
}

impl Timings {
    pub(super) fn read(row: &Row<'_>, at: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            total_ms: row.get(at)?,
            stt_ms: row.get(at + 1)?,
            pipeline_ms: row.get(at + 2)?,
            cleanup_ms: row.get(at + 3)?,
            insert_ms: row.get(at + 4)?,
        })
    }

    pub(super) fn valid(&self) -> bool {
        [
            self.total_ms,
            self.stt_ms,
            self.pipeline_ms,
            self.cleanup_ms,
            self.insert_ms,
        ]
        .into_iter()
        .flatten()
        .all(|ms| ms >= 0)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Distribution {
    pub count: usize,
    pub p50_ms: Option<i64>,
    pub p95_ms: Option<i64>,
}

fn distribution(values: impl Iterator<Item = i64>) -> Distribution {
    let mut values: Vec<_> = values.collect();
    values.sort_unstable();
    // Nearest rank: percentiles are observed durations, without invented interpolated values.
    let percentile = |p: usize| {
        values
            .get((values.len() * p).div_ceil(100).saturating_sub(1))
            .copied()
    };
    Distribution {
        count: values.len(),
        p50_ms: percentile(50),
        p95_ms: percentile(95),
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct TimingSummary {
    pub total: Distribution,
    pub stt: Distribution,
    pub pipeline: Distribution,
    pub cleanup: Distribution,
    pub insert: Distribution,
}

impl TimingSummary {
    pub(super) fn of(rows: &[Timings]) -> Self {
        Self {
            total: distribution(rows.iter().filter_map(|r| r.total_ms)),
            stt: distribution(rows.iter().filter_map(|r| r.stt_ms)),
            pipeline: distribution(rows.iter().filter_map(|r| r.pipeline_ms)),
            cleanup: distribution(rows.iter().filter_map(|r| r.cleanup_ms)),
            insert: distribution(rows.iter().filter_map(|r| r.insert_ms)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_handles_unknown_singleton_even_and_skewed_samples() {
        for (input, expected) in [
            (vec![], (None, None)),
            (vec![0], (Some(0), Some(0))),
            (vec![100, 10], (Some(10), Some(100))),
            (vec![1000, 1, 3, 2, 4], (Some(3), Some(1000))),
        ] {
            let result = distribution(input.iter().copied());
            assert_eq!(result.count, input.len());
            assert_eq!((result.p50_ms, result.p95_ms), expected);
        }
    }
}
