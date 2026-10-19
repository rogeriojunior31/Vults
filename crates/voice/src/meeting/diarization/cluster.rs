//! Fixed-threshold average-linkage baseline, deliberately not tuned against the test fixtures.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};
use std::sync::atomic::{AtomicBool, Ordering};

// Three local voices per five-second hop over a thirty-minute source.
const MAX_OBSERVATIONS: usize = 1080;

pub fn assign(
    embeddings: &[Vec<f32>],
    threshold: f64,
    cannot_link: &BTreeSet<(usize, usize)>,
    stop: &AtomicBool,
) -> Result<Option<Vec<usize>>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if !threshold.is_finite()
        || !(0.0..=2.0).contains(&threshold)
        || embeddings.len() > MAX_OBSERVATIONS
        || cannot_link.len() > MAX_OBSERVATIONS * (MAX_OBSERVATIONS - 1) / 2
    {
        return Err("invalid clustering threshold or observation count".into());
    }
    let normalized: Vec<Vec<f64>> = embeddings
        .iter()
        .map(|e| normalize(e))
        .collect::<Result<_, _>>()?;
    let mut distances = vec![vec![0.0; normalized.len()]; normalized.len()];
    for (i, left) in normalized.iter().enumerate() {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        for (j, right) in normalized.iter().enumerate().take(i) {
            if left.len() != right.len() {
                return Err("embedding dimensions changed".into());
            }
            let distance = (1.0 - left.iter().zip(right).map(|(a, b)| a * b).sum::<f64>()).clamp(0.0, 2.0);
            distances[i][j] = distance;
            distances[j][i] = distance;
        }
    }
    for &(left, right) in cannot_link {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if left == right || left >= embeddings.len() || right >= embeddings.len() {
            return Err("invalid simultaneous speaker constraint".into());
        }
        // Infinity keeps a forbidden pair apart after any number of intermediate group merges.
        distances[left][right] = f64::INFINITY;
        distances[right][left] = f64::INFINITY;
    }
    let mut groups: Vec<Vec<usize>> = (0..embeddings.len()).map(|i| vec![i]).collect();
    let mut candidates = BinaryHeap::new();
    for (right, row) in distances.iter().enumerate() {
        for (left, &distance) in row.iter().enumerate().take(right) {
            if distance <= threshold {
                // Nonnegative finite IEEE bits sort in distance order; IDs settle exact ties.
                candidates.push(Reverse((distance.to_bits(), right, left, 1, 1)));
            }
        }
    }
    while let Some(Reverse((_, right, left, right_size, left_size))) = candidates.pop() {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if groups[right].len() != right_size || groups[left].len() != left_size {
            continue;
        }
        for other in 0..groups.len() {
            if other == left || other == right || groups[other].is_empty() {
                continue;
            }
            // Weighted average linkage preserves the original pairwise mean; infinity carries
            // simultaneous-speaker exclusions through all later merges.
            let distance = (distances[left][other] * left_size as f64
                + distances[right][other] * right_size as f64)
                / (left_size + right_size) as f64;
            distances[left][other] = distance;
            distances[other][left] = distance;
            if distance <= threshold {
                let (a, b) = (other.min(left), other.max(left));
                let size = |id: usize| {
                    if id == left {
                        left_size + right_size
                    } else {
                        groups[id].len()
                    }
                };
                candidates.push(Reverse((distance.to_bits(), b, a, size(b), size(a))));
            }
        }
        let merged = std::mem::take(&mut groups[right]);
        groups[left].extend(merged);
    }
    groups.retain(|group| !group.is_empty());
    groups.sort_by_key(|group| group.iter().min().copied());
    let mut labels = vec![0; embeddings.len()];
    for (label, group) in groups.iter().enumerate() {
        for &index in group {
            labels[index] = label;
        }
    }
    Ok((!stop.load(Ordering::SeqCst)).then_some(labels))
}

fn normalize(embedding: &[f32]) -> Result<Vec<f64>, String> {
    if embedding.is_empty() || embedding.iter().any(|v| !v.is_finite()) {
        return Err("invalid speaker embedding".into());
    }
    let norm = embedding
        .iter()
        .map(|&v| f64::from(v).powi(2))
        .sum::<f64>()
        .sqrt();
    if norm <= f64::EPSILON {
        return Err("empty speaker embedding".into());
    }
    Ok(embedding.iter().map(|&v| f64::from(v) / norm).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thirty_minute_observations_keep_global_identity_and_coactivity() {
        let vectors: Vec<_> = (0..MAX_OBSERVATIONS)
            .map(|i| {
                if i % 2 == 0 {
                    vec![1.0, 0.0]
                } else {
                    vec![-1.0, 0.0]
                }
            })
            .collect();
        let labels = assign(&vectors, 0.7, &BTreeSet::from([(0, 1)]), &AtomicBool::new(false))
            .expect("long recording")
            .expect("active");
        assert_eq!(labels.len(), MAX_OBSERVATIONS);
        for (i, label) in labels.iter().enumerate() {
            assert_eq!(*label, i % 2);
        }
    }

    #[test]
    fn weighted_merges_match_direct_average_linkage() {
        // Independent exhaustive group means catch stale heap entries and unequal merge weights.
        for seed in 1..=24 {
            let vectors: Vec<Vec<f32>> = (0..16)
                .map(|i| {
                    let angle = ((i * 17 + seed * 31) % 101) as f32 * 0.062;
                    vec![angle.cos(), angle.sin()]
                })
                .collect();
            let normalized: Vec<_> = vectors.iter().map(|v| normalize(v).expect("unit")).collect();
            let forbidden = BTreeSet::from([(0, 1), (4, 8)]);
            for threshold in [0.0, 0.7, 2.0] {
                let mut groups: Vec<Vec<usize>> = (0..vectors.len()).map(|i| vec![i]).collect();
                loop {
                    let mut best = None;
                    for (right, b) in groups.iter().enumerate() {
                        for (left, a) in groups.iter().enumerate().take(right) {
                            let mut sum = 0.0;
                            for &i in a {
                                for &j in b {
                                    sum += if forbidden.contains(&(i.min(j), i.max(j))) {
                                        f64::INFINITY
                                    } else {
                                        (1.0 - normalized[i]
                                            .iter()
                                            .zip(&normalized[j])
                                            .map(|(x, y)| x * y)
                                            .sum::<f64>())
                                        .clamp(0.0, 2.0)
                                    };
                                }
                            }
                            let distance = sum / (a.len() * b.len()) as f64;
                            if distance <= threshold && best.is_none_or(|(_, _, d)| distance < d) {
                                best = Some((left, right, distance));
                            }
                        }
                    }
                    let Some((left, right, _)) = best else {
                        break;
                    };
                    let merged = groups.remove(right);
                    groups[left].extend(merged);
                }
                let mut expected = vec![0; vectors.len()];
                for (id, group) in groups.iter().enumerate() {
                    for &i in group {
                        expected[i] = id;
                    }
                }
                assert_eq!(
                    assign(&vectors, threshold, &forbidden, &AtomicBool::new(false))
                        .expect("groups")
                        .expect("active"),
                    expected,
                    "seed={seed}, threshold={threshold}"
                );
            }
        }
    }

    #[test]
    fn a_speaker_keeps_one_label_and_opposite_speakers_stay_separate() {
        assert_eq!(
            assign(
                &[vec![1.0, 0.0], vec![-1.0, 0.0], vec![0.9, 0.1], vec![-0.9, 0.1]],
                0.7,
                &BTreeSet::new(),
                &AtomicBool::new(false)
            )
            .expect("groups")
            .expect("not cancelled"),
            [0, 1, 0, 1]
        );
        assert!(
            assign(&[], 0.7, &BTreeSet::new(), &AtomicBool::new(false))
                .expect("silence")
                .expect("not cancelled")
                .is_empty()
        );
    }
    #[test]
    fn rejects_invalid_vectors_and_limits_instead_of_fabricating_speakers() {
        for embeddings in [
            vec![vec![]],
            vec![vec![0.0]],
            vec![vec![f32::NAN]],
            vec![vec![1.0], vec![1.0, 0.0]],
            vec![vec![1.0]; MAX_OBSERVATIONS + 1],
        ] {
            assert!(assign(&embeddings, 0.7, &BTreeSet::new(), &AtomicBool::new(false)).is_err());
        }
        assert!(assign(&[], f64::NAN, &BTreeSet::new(), &AtomicBool::new(false)).is_err());
    }
    #[test]
    fn average_linkage_does_not_use_single_linkage_chaining() {
        // Adjacent angles are close, but joining the far endpoint to the first pair exceeds 0.7.
        let vectors: Vec<_> = [0.0_f32, 1.1, 2.2]
            .iter()
            .map(|&angle| vec![angle.cos(), angle.sin()])
            .collect();
        let result = assign(&vectors, 0.7, &BTreeSet::new(), &AtomicBool::new(false))
            .expect("groups")
            .expect("not cancelled");
        assert_ne!(result[0], result[2]);
    }

    #[test]
    fn coactive_pairs_cannot_merge_directly_or_through_another_observation() {
        let vectors = vec![vec![1.0, 0.0]; 4];
        let stop = AtomicBool::new(false);
        assert_eq!(
            assign(&vectors, 0.7, &BTreeSet::new(), &stop)
                .expect("groups")
                .expect("active"),
            [0, 0, 0, 0]
        );
        for pair in [(0, 1), (0, 3), (3, 0), (1, 2)] {
            let result = assign(&vectors, 0.7, &BTreeSet::from([pair]), &stop)
                .expect("constrained groups")
                .expect("active");
            assert_ne!(result[pair.0], result[pair.1]);
            assert_eq!(result.iter().copied().max(), Some(1));
        }
        for pair in [(0, 0), (0, 4)] {
            assert!(assign(&vectors, 0.7, &BTreeSet::from([pair]), &stop).is_err());
        }
        assert!(
            assign(&vectors, 0.7, &BTreeSet::from([(0, 1)]), &AtomicBool::new(true))
                .expect("cancelled")
                .is_none()
        );
    }
}
