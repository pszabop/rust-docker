//! Nilsimsa Hash Correctness Testing Library
//!
//! Provides tools for analyzing nilsimsa locality-sensitive hash behavior,
//! specifically for browser fingerprint discrimination and fuzzy rate limiting.

pub mod hash;
pub mod modification;
pub mod rate_limiter;
pub mod benchmarks;

// Re-export commonly used items
pub use hash::{stable_document_hash_64, stable_document_hash_256, hamming_distance_64, hamming_distance_256};
pub use modification::{ModType, randomly_modify_letters, randomly_modify_words};
pub use rate_limiter::FuzzyRateLimiter;
