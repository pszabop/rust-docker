//! Thread-safe fuzzy hash database and rate limiter using SIMD hamming distance
//!
//! Designed for tokio async contexts. Uses parking_lot::RwLock which is safe
//! for async code when critical sections are <100μs.
//!
//! The core data structure is `FuzzyHashBucket<T>` which stores records keyed by
//! fuzzy hash. Records are accessed via callback with `&T`, so use interior
//! mutability (AtomicU32, Mutex, etc.) for mutable fields.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::hash::{hash_to_u64, hamming_distance_u64x4};

/// A single bucket containing fingerprint hashes and their associated records.
/// Uses parking_lot::RwLock for better performance than std::sync::RwLock.
///
/// Records are accessed via `&T` (shared reference), so use interior mutability
/// (AtomicU32, Mutex, RwLock, Cell, RefCell) for fields that need mutation.
/// This allows atomic operations under a read lock for maximum performance.
pub struct FuzzyHashBucket<T> {
    /// Fingerprint hashes stored as 4 x u64 for SIMD-friendly access
    hashes: RwLock<Vec<[u64; 4]>>,
    /// Records (parallel array to hashes)
    records: RwLock<Vec<T>>,
    /// Number of entries (for lock-free read of size)
    len: AtomicUsize,
}

impl<T: Default> FuzzyHashBucket<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            hashes: RwLock::new(Vec::with_capacity(capacity)),
            records: RwLock::new(Vec::with_capacity(capacity)),
            len: AtomicUsize::new(0),
        }
    }

    /// Find an existing entry and call f with shared reference to record.
    /// Returns None if not found.
    ///
    /// For mutation, T should use interior mutability (Atomic*, Mutex, etc.).
    /// This allows atomic operations under a read lock.
    pub fn find<F, R>(&self, query: &[u64; 4], threshold: u32, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        let hashes = self.hashes.read();
        let records = self.records.read();

        for (i, h) in hashes.iter().enumerate() {
            if hamming_distance_u64x4(query, h) <= threshold {
                return Some(f(&records[i]));
            }
        }
        None
    }

    /// Find existing entry or insert default, then call f with shared reference.
    /// Returns (result of f, is_new).
    ///
    /// This is the hot path - optimized for the common case (found).
    /// Note: Uses parking_lot which doesn't block the tokio runtime for these
    /// short critical sections (<10μs). Safe to call from async context.
    pub fn find_or_insert<F, R>(&self, query: &[u64; 4], threshold: u32, f: F) -> (R, bool)
    where
        F: FnOnce(&T) -> R,
    {
        // Fast path: read lock for search
        {
            let hashes = self.hashes.read();
            let records = self.records.read();

            for (i, h) in hashes.iter().enumerate() {
                if hamming_distance_u64x4(query, h) <= threshold {
                    // Found - call f with shared reference (allows atomic ops)
                    return (f(&records[i]), false);
                }
            }
        }
        // Drop read locks before acquiring write locks

        // Slow path: need to insert
        {
            let mut hashes = self.hashes.write();
            let mut records = self.records.write();

            // Double-check: another thread might have inserted while we waited
            for (i, h) in hashes.iter().enumerate() {
                if hamming_distance_u64x4(query, h) <= threshold {
                    return (f(&records[i]), false);
                }
            }

            // Actually insert
            hashes.push(*query);
            records.push(T::default());
            let idx = records.len() - 1;
            self.len.fetch_add(1, Ordering::Relaxed);
            (f(&records[idx]), true)
        }
    }

    /// Update an existing entry with exclusive access. Returns None if not found.
    /// Use this when you need &mut T (e.g., for non-atomic updates).
    pub fn update<F, R>(&self, query: &[u64; 4], threshold: u32, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let hashes = self.hashes.read();
        let mut records = self.records.write();

        for (i, h) in hashes.iter().enumerate() {
            if hamming_distance_u64x4(query, h) <= threshold {
                return Some(f(&mut records[i]));
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        let mut hashes = self.hashes.write();
        let mut records = self.records.write();
        hashes.clear();
        records.clear();
        self.len.store(0, Ordering::Relaxed);
    }
}

/// Simple fuzzy hash database without time bucketing.
/// Good for session stores, reputation tracking, etc.
pub struct FuzzyHashDB<T> {
    bucket: FuzzyHashBucket<T>,
    threshold: u32,
}

impl<T: Default> FuzzyHashDB<T> {
    pub fn new(threshold: u32, capacity: usize) -> Self {
        Self {
            bucket: FuzzyHashBucket::new(capacity),
            threshold,
        }
    }

    /// Find an existing entry by hash.
    pub fn find<F, R>(&self, hash: &[u8; 32], f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        let query = hash_to_u64(hash);
        self.bucket.find(&query, self.threshold, f)
    }

    /// Find existing entry or insert default.
    pub fn find_or_insert<F, R>(&self, hash: &[u8; 32], f: F) -> (R, bool)
    where
        F: FnOnce(&T) -> R,
    {
        let query = hash_to_u64(hash);
        self.bucket.find_or_insert(&query, self.threshold, f)
    }

    /// Update an existing entry with exclusive access.
    pub fn update<F, R>(&self, hash: &[u8; 32], f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let query = hash_to_u64(hash);
        self.bucket.update(&query, self.threshold, f)
    }

    pub fn len(&self) -> usize {
        self.bucket.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bucket.is_empty()
    }

    pub fn clear(&self) {
        self.bucket.clear();
    }

    pub fn threshold(&self) -> u32 {
        self.threshold
    }
}

// ============================================================================
// Rate Limiter - specialized use of FuzzyHashDB with time bucketing
// ============================================================================

/// Record for rate limiting - uses AtomicU32 for lock-free increment
#[derive(Default)]
pub struct RateLimitRecord {
    pub count: AtomicU32,
}

impl RateLimitRecord {
    pub fn increment(&self) -> u32 {
        self.count.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn get(&self) -> u32 {
        self.count.load(Ordering::Relaxed)
    }
}

/// Time-bucketed rate limiter with configurable window.
///
/// Designed for use with tokio - all operations are fast enough (<50μs)
/// that they don't need spawn_blocking.
pub struct FuzzyRateLimiter {
    buckets: Vec<FuzzyHashBucket<RateLimitRecord>>,
    bucket_duration_secs: u64,
    threshold: u32,
    num_buckets: usize,
}

impl FuzzyRateLimiter {
    /// Create a new rate limiter.
    ///
    /// # Arguments
    /// * `num_buckets` - Number of time buckets (e.g., 12)
    /// * `bucket_duration_secs` - Duration of each bucket in seconds (e.g., 10)
    /// * `threshold` - Hamming distance threshold for fuzzy matching (e.g., 30)
    /// * `capacity_per_bucket` - Expected max entries per bucket for pre-allocation
    ///
    /// Total window = num_buckets * bucket_duration_secs
    pub fn new(num_buckets: usize, bucket_duration_secs: u64, threshold: u32, capacity_per_bucket: usize) -> Self {
        let buckets = (0..num_buckets)
            .map(|_| FuzzyHashBucket::new(capacity_per_bucket))
            .collect();
        Self {
            buckets,
            bucket_duration_secs,
            threshold,
            num_buckets,
        }
    }

    /// Get the bucket index for a given timestamp
    #[inline]
    pub fn bucket_for_time(&self, timestamp_secs: u64) -> usize {
        ((timestamp_secs / self.bucket_duration_secs) as usize) % self.num_buckets
    }

    /// Check request and return total count across all buckets.
    ///
    /// This is safe to call from async context - operations are <50μs.
    pub fn check_request(&self, hash: &[u8; 32], timestamp_secs: u64) -> u32 {
        let query = hash_to_u64(hash);
        let current_bucket = self.bucket_for_time(timestamp_secs);

        // Increment in current bucket
        self.buckets[current_bucket].find_or_insert(&query, self.threshold, |record| {
            record.increment();
        });

        // Sum counts across all buckets
        let mut total = 0u32;
        for bucket in &self.buckets {
            if let Some(count) = bucket.find(&query, self.threshold, |r| r.get()) {
                total += count;
            }
        }
        total
    }

    /// Check if a fingerprint is rate limited (exceeds threshold count)
    pub fn is_rate_limited(&self, hash: &[u8; 32], timestamp_secs: u64, max_requests: u32) -> bool {
        self.check_request(hash, timestamp_secs) > max_requests
    }

    /// Rotate buckets - clear the oldest bucket.
    /// Call this periodically (e.g., every bucket_duration_secs).
    pub fn rotate(&self, timestamp_secs: u64) {
        let next_bucket = self.bucket_for_time(timestamp_secs + self.bucket_duration_secs);
        self.buckets[next_bucket].clear();
    }

    /// Start a background task that rotates buckets automatically.
    /// Returns a handle that can be used to stop the rotation.
    pub fn start_rotation_task(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let limiter = Arc::clone(self);
        let duration = self.bucket_duration_secs;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(duration)
            );

            loop {
                interval.tick().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                limiter.rotate(now);
            }
        })
    }

    pub fn total_entries(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    pub fn window_duration_secs(&self) -> u64 {
        self.num_buckets as u64 * self.bucket_duration_secs
    }
}

// ============================================================================
// Backwards compatibility - re-export old names
// ============================================================================

/// Backwards compatible alias
pub type RateLimitBucket = FuzzyHashBucket<RateLimitRecord>;
