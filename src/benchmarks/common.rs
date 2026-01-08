//! Common types and utilities shared across benchmarks

/// Hash size variants
#[derive(Clone, Copy, Debug)]
pub enum HashSize {
    Bit64,
    Bit256,
}

/// Statistics for a single modification level
#[derive(Clone, Debug)]
pub struct Stats {
    pub num_modifications: usize,
    pub mean: f64,
    pub stdev: f64,
}

/// Calculate percentile of a list of values
pub fn percentile(values: &[u32], pct: f64) -> u32 {
    let mut sorted: Vec<u32> = values.to_vec();
    sorted.sort();
    let idx = ((pct / 100.0) * (sorted.len() - 1) as f64) as usize;
    sorted[idx]
}
