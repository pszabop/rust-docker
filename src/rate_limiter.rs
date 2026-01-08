//! Thread-safe fuzzy rate limiter using SIMD hamming distance
//!
//! Designed for tokio async contexts. Uses parking_lot::RwLock which is safe
//! for async code when critical sections are <100μs.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::hash::{hash_to_u64, hamming_distance_u64x4};

/// A single time bucket containing fingerprint hashes and their request counts.
/// Uses parking_lot::RwLock for better performance than std::sync::RwLock.
pub struct RateLimitBucket {
    /// Fingerprint hashes stored as 4 x u64 for SIMD-friendly access
    hashes: RwLock<Vec<[u64; 4]>>,
    /// Request counts (parallel array to hashes)
    counts: RwLock<Vec<AtomicU32>>,
    /// Number of entries (for lock-free read of size)
    len: AtomicUsize,
}

impl RateLimitBucket {
    pub fn new(capacity: usize) -> Self {
        Self {
            hashes: RwLock::new(Vec::with_capacity(capacity)),
            counts: RwLock::new(Vec::with_capacity(capacity)),
            len: AtomicUsize::new(0),
        }
    }

    /// Check request and increment count. Returns (count, is_new).
    /// This is the hot path - optimized for the common case (found).
    ///
    /// Note: Uses parking_lot which doesn't block the tokio runtime for these
    /// short critical sections (<10μs). Safe to call from async context.
    pub fn check_and_increment(&self, query: &[u64; 4], threshold: u32) -> (u32, bool) {
        // Fast path: read lock for search
        {
            let hashes = self.hashes.read();
            let counts = self.counts.read();

            for (i, h) in hashes.iter().enumerate() {
                if hamming_distance_u64x4(query, h) <= threshold {
                    // Found - increment atomically (no write lock needed!)
                    let new_count = counts[i].fetch_add(1, Ordering::Relaxed) + 1;
                    return (new_count, false);
                }
            }
        }
        // Drop read lock before acquiring write lock

        // Slow path: need to insert
        {
            let mut hashes = self.hashes.write();
            let mut counts = self.counts.write();

            // Double-check: another thread might have inserted while we waited
            for (i, h) in hashes.iter().enumerate() {
                if hamming_distance_u64x4(query, h) <= threshold {
                    let new_count = counts[i].fetch_add(1, Ordering::Relaxed) + 1;
                    return (new_count, false);
                }
            }

            // Actually insert
            hashes.push(*query);
            counts.push(AtomicU32::new(1));
            self.len.fetch_add(1, Ordering::Relaxed);
            (1, true)
        }
    }

    /// Get current count for a fingerprint (read-only)
    pub fn get_count(&self, query: &[u64; 4], threshold: u32) -> Option<u32> {
        let hashes = self.hashes.read();
        let counts = self.counts.read();

        for (i, h) in hashes.iter().enumerate() {
            if hamming_distance_u64x4(query, h) <= threshold {
                return Some(counts[i].load(Ordering::Relaxed));
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
        let mut counts = self.counts.write();
        hashes.clear();
        counts.clear();
        self.len.store(0, Ordering::Relaxed);
    }
}

/// Time-bucketed rate limiter with configurable window.
///
/// Designed for use with tokio - all operations are fast enough (<50μs)
/// that they don't need spawn_blocking.
///
/// # Example with Tokio
/// ```ignore
/// use std::sync::Arc;
/// use std::time::{SystemTime, UNIX_EPOCH};
///
/// let limiter = Arc::new(FuzzyRateLimiter::new(12, 10, 30, 10_000));
///
/// // In an async handler:
/// async fn handle_request(limiter: Arc<FuzzyRateLimiter>, fingerprint: [u8; 32]) -> bool {
///     let now = SystemTime::now()
///         .duration_since(UNIX_EPOCH)
///         .unwrap()
///         .as_secs();
///
///     let count = limiter.check_request(&fingerprint, now);
///     count <= 100 // Allow up to 100 requests per window
/// }
/// ```
pub struct FuzzyRateLimiter {
    buckets: Vec<RateLimitBucket>,
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
            .map(|_| RateLimitBucket::new(capacity_per_bucket))
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
        let (_, _) = self.buckets[current_bucket].check_and_increment(&query, self.threshold);

        // Sum counts across all buckets
        let mut total = 0u32;
        for bucket in &self.buckets {
            if let Some(count) = bucket.get_count(&query, self.threshold) {
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
