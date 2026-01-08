//! Nilsimsa hash functions and utilities
//!
//! Provides 64-bit and 256-bit locality-sensitive hashing using nilsimsa,
//! plus hamming distance calculations.

/// Compute 64-bit nilsimsa hash (truncated from 256-bit)
pub fn stable_document_hash_64(input: &str) -> u64 {
    let mut hasher = nilsimsa::Nilsimsa::new();
    hasher.update(input);
    let hex_str = hasher.digest();
    u64::from_str_radix(&hex_str[0..16], 16).expect("Invalid hex")
}

/// Compute full 256-bit nilsimsa hash
pub fn stable_document_hash_256(input: &str) -> [u8; 32] {
    let mut hasher = nilsimsa::Nilsimsa::new();
    hasher.update(input);
    let hex_str = hasher.digest();
    let mut result = [0u8; 32];
    for i in 0..32 {
        result[i] = u8::from_str_radix(&hex_str[i * 2..i * 2 + 2], 16).expect("Invalid hex");
    }
    result
}

/// Hamming distance between two 64-bit values
#[inline]
pub fn hamming_distance_64(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Hamming distance between two 256-bit hashes
#[inline]
pub fn hamming_distance_256(a: &[u8; 32], b: &[u8; 32]) -> u32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x ^ y).count_ones()).sum()
}

/// Wrapper type for 256-bit hash to use with bktree crate
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hash256(pub [u8; 32]);

/// Hamming distance function for bktree (returns isize as required by crate)
pub fn hash256_hamming_distance(a: &Hash256, b: &Hash256) -> isize {
    a.0.iter()
        .zip(b.0.iter())
        .map(|(x, y)| (x ^ y).count_ones() as isize)
        .sum()
}

/// Convert a 256-bit hash to 4 x u64 for SIMD operations
#[inline]
pub fn hash_to_u64(hash: &[u8; 32]) -> [u64; 4] {
    let mut arr = [0u64; 4];
    for i in 0..4 {
        arr[i] = u64::from_le_bytes([
            hash[i*8], hash[i*8+1], hash[i*8+2], hash[i*8+3],
            hash[i*8+4], hash[i*8+5], hash[i*8+6], hash[i*8+7],
        ]);
    }
    arr
}

/// SIMD-optimized hamming distance for u64x4 representation
#[inline(always)]
pub fn hamming_distance_u64x4(a: &[u64; 4], b: &[u64; 4]) -> u32 {
    (a[0] ^ b[0]).count_ones()
        + (a[1] ^ b[1]).count_ones()
        + (a[2] ^ b[2]).count_ones()
        + (a[3] ^ b[3]).count_ones()
}
