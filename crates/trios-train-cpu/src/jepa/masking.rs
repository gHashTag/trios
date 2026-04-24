//! TASK-5A.1 — Span Masking for T-JEPA
//!
//! Pure functions, zero dependencies beyond rand.
//! Theory: https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/

use rand::{Rng, rngs::StdRng, SeedableRng};

/// Mask configuration
#[derive(Debug, Clone, Copy)]
pub struct MaskConfig {
    pub ratio: f64,
    pub min_span: usize,
    pub max_span: usize,
    pub num_spans: usize,
}

impl Default for MaskConfig {
    fn default() -> Self {
        Self { ratio: 0.3, min_span: 3, max_span: 9, num_spans: 2 }
    }
}

#[derive(Debug, Clone)]
pub struct MaskResult {
    pub mask: Vec<bool>,
    pub spans: Vec<(usize, usize)>,
}

pub fn mask_spans(seq_len: usize, config: MaskConfig, rng: &mut impl Rng) -> MaskResult {
    let mut mask = vec![false; seq_len];
    let mut spans = Vec::with_capacity(config.num_spans);
    for _ in 0..config.num_spans {
        if seq_len < config.min_span { break; }
        let span_len = rng.gen_range(config.min_span..=config.max_span.min(seq_len));
        let max_start = seq_len.saturating_sub(span_len);
        let start = if max_start == 0 { 0 } else { rng.gen_range(0..max_start) };
        let end = (start + span_len).min(seq_len);
        for i in start..end { mask[i] = true; }
        spans.push((start, end));
    }
    MaskResult { mask, spans }
}

pub fn get_unmasked(mask: &[bool]) -> Vec<usize> {
    mask.iter().enumerate().filter_map(|(i, &m)| if !m { Some(i) } else { None }).collect()
}

pub fn get_masked(mask: &[bool]) -> Vec<usize> {
    mask.iter().enumerate().filter_map(|(i, &m)| if m { Some(i) } else { None }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_ratio_approximate() {
        let mut rng = StdRng::seed_from_u64(42);
        let result = mask_spans(100, MaskConfig::default(), &mut rng);
        let ratio = result.mask.iter().filter(|&&m| m).count() as f64 / 100.0;
        assert!(ratio > 0.0 && ratio < 0.5, "ratio={ratio:.2}");
    }

    #[test]
    fn test_span_bounds() {
        let mut rng = StdRng::seed_from_u64(42);
        let config = MaskConfig::default();
        let result = mask_spans(100, config, &mut rng);
        for (start, end) in result.spans {
            assert!(start < end);
            assert!(end <= 100);
            let len = end - start;
            assert!(len >= config.min_span && len <= config.max_span);
        }
    }

    #[test]
    fn test_context_target_partition() {
        let mut rng = StdRng::seed_from_u64(99);
        let result = mask_spans(64, MaskConfig::default(), &mut rng);
        let mut all: Vec<usize> = get_unmasked(&result.mask).into_iter()
            .chain(get_masked(&result.mask)).collect();
        all.sort_unstable();
        assert_eq!(all, (0..64).collect::<Vec<_>>());
    }

    #[test]
    fn test_short_sequence() {
        let mut rng = StdRng::seed_from_u64(1);
        let result = mask_spans(2, MaskConfig::default(), &mut rng);
        assert_eq!(result.mask.len(), 2);
    }
}
