//! Evicting fuzzy hash database with stochastic garbage collection
//!
//! Fixed-size database that evicts lowest-score entries when full.
//! Designed for abuse detection where losing some entries is acceptable.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use parking_lot::RwLock;
use rand::Rng;

use crate::hash::hamming_distance_u64x4;

/// Record tracking hit count and recency for eviction scoring
#[repr(C)]
#[derive(Default)]
pub struct HotRecord {
    pub last_hit: AtomicU32,   // Seconds since epoch (or startup)
    pub hit_count: AtomicU32,
}

impl HotRecord {
    #[inline]
    pub fn hit(&self, now: u32) {
        self.last_hit.store(now, Ordering::Relaxed);
        self.hit_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn eviction_score(&self, now: u32) -> u32 {
        let age = now.wrapping_sub(self.last_hit.load(Ordering::Relaxed));
        let hits = self.hit_count.load(Ordering::Relaxed);
        // Higher score = less likely to evict
        // Each hit buys 60 seconds of protection
        hits.saturating_mul(60).saturating_sub(age)
    }

    #[inline]
    pub fn reset(&self, now: u32) {
        self.last_hit.store(now, Ordering::Relaxed);
        self.hit_count.store(1, Ordering::Relaxed);
    }
}

/// Fixed-size evicting database
pub struct EvictingBucket {
    hashes: RwLock<Vec<[u64; 4]>>,
    records: RwLock<Vec<HotRecord>>,
    len: AtomicUsize,
    capacity: usize,
    threshold: u32,
}

impl EvictingBucket {
    pub fn new(capacity: usize, threshold: u32) -> Self {
        let mut hashes = Vec::with_capacity(capacity);
        let mut records = Vec::with_capacity(capacity);

        // Pre-allocate to capacity
        hashes.resize_with(capacity, || [0u64; 4]);
        records.resize_with(capacity, HotRecord::default);

        Self {
            hashes: RwLock::new(hashes),
            records: RwLock::new(records),
            len: AtomicUsize::new(0),
            capacity,
            threshold,
        }
    }

    /// Combined single-pass: search for match AND track victim in 1/8 window
    /// Returns (hit_count, was_new_insert)
    pub fn find_or_insert_combined(&self, query: &[u64; 4], now: u32) -> (u32, bool) {
        let len = self.len.load(Ordering::Relaxed);

        // If not at capacity, try simple insert first
        if len < self.capacity {
            return self.find_or_append(query, now);
        }

        // At capacity: combined search + victim scan
        let victim_start = rand::thread_rng().gen_range(0..len);
        let victim_window = len / 8;

        let mut victim_idx = victim_start;
        let mut min_score = u32::MAX;

        // Combined pass under read lock
        {
            let hashes = self.hashes.read();
            let records = self.records.read();

            for i in 0..len {
                // ALWAYS: check for match
                if hamming_distance_u64x4(query, &hashes[i]) <= self.threshold {
                    records[i].hit(now);
                    return (records[i].hit_count.load(Ordering::Relaxed), false);
                }

                // ONLY in victim window: track score
                let offset = i.wrapping_sub(victim_start);
                if offset < victim_window {
                    let score = records[i].eviction_score(now);
                    if score < min_score {
                        min_score = score;
                        victim_idx = i;
                    }
                }
            }
        }

        // Miss - evict victim under write lock
        {
            let mut hashes = self.hashes.write();
            let records = self.records.read();

            hashes[victim_idx] = *query;
            records[victim_idx].reset(now);
        }

        (1, true)
    }

    /// Separate passes: full search, then 1/8 victim scan if miss
    /// Returns (hit_count, was_new_insert)
    pub fn find_or_insert_separate(&self, query: &[u64; 4], now: u32) -> (u32, bool) {
        let len = self.len.load(Ordering::Relaxed);

        // If not at capacity, try simple insert first
        if len < self.capacity {
            return self.find_or_append(query, now);
        }

        // Pass 1: Search for match (read lock)
        {
            let hashes = self.hashes.read();
            let records = self.records.read();

            for i in 0..len {
                if hamming_distance_u64x4(query, &hashes[i]) <= self.threshold {
                    records[i].hit(now);
                    return (records[i].hit_count.load(Ordering::Relaxed), false);
                }
            }
        }

        // Pass 2: Find victim in 1/8 window (read lock)
        let victim_idx = {
            let records = self.records.read();

            let victim_start = rand::thread_rng().gen_range(0..len);
            let victim_window = len / 8;

            let mut victim_idx = victim_start;
            let mut min_score = u32::MAX;

            for i in 0..victim_window {
                let idx = (victim_start + i) % len;
                let score = records[idx].eviction_score(now);
                if score < min_score {
                    min_score = score;
                    victim_idx = idx;
                }
            }

            victim_idx
        };

        // Evict victim (write lock - just the overwrite)
        {
            let mut hashes = self.hashes.write();
            let records = self.records.read();

            hashes[victim_idx] = *query;
            records[victim_idx].reset(now);
        }

        (1, true)
    }

    /// Helper: find or append when not at capacity
    fn find_or_append(&self, query: &[u64; 4], now: u32) -> (u32, bool) {
        // Fast path: search
        {
            let hashes = self.hashes.read();
            let records = self.records.read();
            let len = self.len.load(Ordering::Relaxed);

            for i in 0..len {
                if hamming_distance_u64x4(query, &hashes[i]) <= self.threshold {
                    records[i].hit(now);
                    return (records[i].hit_count.load(Ordering::Relaxed), false);
                }
            }
        }

        // Slow path: append
        {
            let mut hashes = self.hashes.write();
            let records = self.records.read();
            let len = self.len.load(Ordering::Relaxed);

            // Double-check
            for i in 0..len {
                if hamming_distance_u64x4(query, &hashes[i]) <= self.threshold {
                    records[i].hit(now);
                    return (records[i].hit_count.load(Ordering::Relaxed), false);
                }
            }

            // Append if still room
            if len < self.capacity {
                hashes[len] = *query;
                records[len].reset(now);
                self.len.fetch_add(1, Ordering::Relaxed);
                return (1, true);
            }
        }

        // Race: became full while we waited, fall back to eviction
        self.find_or_insert_combined(query, now)
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// Fill the database with random entries (for benchmarking)
    pub fn fill_random(&self, now: u32) {
        let mut hashes = self.hashes.write();
        let records = self.records.read();

        for i in 0..self.capacity {
            hashes[i] = [
                rand::random::<u64>(),
                rand::random::<u64>(),
                rand::random::<u64>(),
                rand::random::<u64>(),
            ];
            records[i].reset(now);
            // Vary the hit counts and ages for realistic eviction
            records[i].hit_count.store(rand::thread_rng().gen_range(1..100), Ordering::Relaxed);
            records[i].last_hit.store(now.saturating_sub(rand::thread_rng().gen_range(0..3600)), Ordering::Relaxed);
        }

        self.len.store(self.capacity, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_insert_and_find() {
        let db = EvictingBucket::new(100, 32);
        let hash: [u64; 4] = [1, 2, 3, 4];

        // First insert
        let (count, is_new) = db.find_or_insert_combined(&hash, 1000);
        assert_eq!(count, 1);
        assert!(is_new);

        // Second access - should find existing
        let (count, is_new) = db.find_or_insert_combined(&hash, 1001);
        assert_eq!(count, 2);
        assert!(!is_new);
    }

    #[test]
    fn test_eviction_when_full() {
        let db = EvictingBucket::new(10, 32);
        db.fill_random(1000);

        assert!(db.is_full());

        // Insert new entry - should evict something
        let new_hash: [u64; 4] = [0xDEADBEEF, 0xCAFEBABE, 0x12345678, 0x87654321];
        let (count, is_new) = db.find_or_insert_combined(&new_hash, 2000);

        assert_eq!(count, 1);
        assert!(is_new);
        assert_eq!(db.len(), 10); // Still at capacity
    }
}
