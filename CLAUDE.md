# Nilsimsa Hash Correctness Testing

Rust project for analyzing nilsimsa locality-sensitive hash behavior, specifically for browser fingerprint
discrimination and fuzzy rate limiting.

## Quick Reference

```bash
./run.sh                  # Show available simulations
./run.sh discrimination   # Run discrimination threshold analysis
./run.sh uniformity       # Run bit uniformity analysis (for hash segmentation)
./run.sh ratelimiter      # Benchmark thread-safe rate limiter
./run.sh test             # Run unit tests (15 tests)
```

All execution happens in Docker (see docker-compose.yml). Output files (PNGs) appear in project root.

## What This Project Does

Tests whether nilsimsa hashes can distinguish browser fingerprints (Chrome vs Firefox) despite:
- Random character changes (scattered modifications)
- Random word/field changes (concentrated modifications)
- Insertions and removals
- Font list order randomization (privacy countermeasure)

Also provides a production-ready **FuzzyRateLimiter** for rate-limiting based on fingerprint similarity.

## Key Concepts

**Nilsimsa**: Locality-sensitive hash producing 256 bits. Similar documents produce similar hashes.
We test both 64-bit (truncated) and full 256-bit versions.

**Hamming Distance**: Count of differing bits between two hashes. Lower = more similar.

**Discrimination Threshold**: When combined noise (Chrome mean+2σ + Firefox mean+2σ) exceeds baseline
browser difference, you can no longer reliably distinguish them.

**Locality Benefit**: Concentrated changes (replacing whole words/fields) cause less hash disruption
than the same number of scattered single-character changes. This is nilsimsa's key property.

## Current Findings

```
                    64-bit      256-bit
Random Letters:     ~32         ~64     chars  (scattered - worst)
Random Words:       ~64         ~128    chars  (concentrated - best)
Random Inserts:     ~64         ~128    chars
Random Removes:     ~64         ~256    chars
```

Baseline Chrome vs Firefox: 13 bits (64-bit), 63 bits (256-bit)

### Rate Limiter Performance

SIMD brute-force beats BK-tree for fuzzy hash lookup:
- BK-tree: ~9000μs per query (45x too slow)
- SIMD brute-force: ~44μs per query (meets 200μs target)

FuzzyRateLimiter with tokio:
- 386K req/sec throughput
- 2.59μs per request
- 100% hit rate on fuzzed fingerprints
- 0% false positives on different browsers

### Bit Uniformity (for segmentation)

Nilsimsa bit flips are **NOT uniform** across the 256 bits. This affects hash segmentation strategies.

```
Coefficient of Variation: 1.5 - 3.5 (want <0.1 for uniform)
Some bits never flip, others flip 30-50% of trials
```

By 32-bit segment (8 segments total):
- Segments 1,2 (bits 32-95): Low sensitivity, rarely flip
- Segment 5 (bits 160-191): High sensitivity, flips ~8x more than seg 2

**Implication**: Simple segment-based fuzzy lookup won't work well. Use brute-force SIMD instead.

## File Structure

```
src/
├── main.rs              # CLI dispatcher and unit tests
├── lib.rs               # Library exports
├── hash.rs              # Nilsimsa hash functions, hamming distance
├── modification.rs      # Document modification functions
├── rate_limiter.rs      # FuzzyRateLimiter (tokio-compatible)
└── benchmarks/
    ├── mod.rs
    ├── common.rs              # Shared types (Stats, HashSize)
    ├── bit_uniformity.rs      # Bit flip uniformity analysis
    ├── segmentation.rs        # Hash segmentation for KV lookup
    ├── ratelimit_threshold.rs # Threshold analysis
    ├── simhash_comparison.rs  # Simhash vs nilsimsa comparison
    ├── bktree.rs              # BK-tree benchmark
    ├── ratelimiter.rs         # Rate limiter benchmark
    └── discrimination.rs      # Browser discrimination analysis
```

Supporting files:
- `chrome_values.json` / `firefox_values.json` - Sample browser fingerprints (CSV format, not JSON)
- `run.sh` - Docker wrapper for cargo run/test
- `discrimination_*.png` - Generated discrimination threshold plots
- `bit_uniformity_*.png` - Generated bit flip distribution plots

## Testing Notes

The fingerprint files are actually CSV strings wrapped in quotes, not JSON objects:
```
"Chrome, 128, 128.0.0, Mac OS X, ..."
```

The `find_csv_fields()` function in `modification.rs` parses these comma-separated values.

Unit tests use specific (not random) alterations for deterministic results. Tests cover:
- Hash consistency
- Browser discrimination
- Font list sorting (neutralizes order randomization)
- Edge cases

## Using the Rate Limiter

```rust
use std::sync::Arc;
use hash_correctness::FuzzyRateLimiter;

// Initialize once at startup
let limiter = Arc::new(FuzzyRateLimiter::new(
    12,      // 12 buckets
    10,      // 10 seconds each = 2 minute window
    30,      // Hamming distance threshold
    10_000,  // Capacity per bucket
));

// Start automatic rotation
let _rotation_handle = limiter.start_rotation_task();

// In request handler
if limiter.is_rate_limited(&fingerprint_hash, timestamp_secs, 100) {
    return Err(RateLimited);
}
```

## Browser Fingerprinting Context

Browsers try to obfuscate fingerprints by randomizing:
- Canvas rendering (changes hash fields)
- Font enumeration order (sorting neutralizes this)
- WebGL renderer strings
- Audio context values

Recommendation: Sort font lists before hashing, use 256-bit, consider removing known-randomized
fields if you only need browser/OS identification rather than unique user tracking.
