# Nilsimsa Hash Correctness Testing

Rust project for analyzing nilsimsa locality-sensitive hash behavior, specifically for browser fingerprint
discrimination and fuzzy rate limiting.

## Quick Reference

```bash
./run.sh                  # Show available simulations
./run.sh realistic        # Real fingerprint analysis + threshold selection (primary)
./run.sh fingerprints     # Compare WAF hash vs data-only hash format
./run.sh discrimination   # Run discrimination threshold analysis
./run.sh uniformity       # Run bit uniformity analysis (for hash segmentation)
./run.sh ratelimiter      # Benchmark thread-safe rate limiter
./run.sh test             # Run unit tests
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

### Two-Threshold Strategy (Production Recommendation)

Use two thresholds for different enforcement scenarios:

| Threshold | Mode | Use Case | False Positive | Same-User Match |
|-----------|------|----------|----------------|-----------------|
| 24 | STRICT | Rate limiting | 0% | 67% |
| 40 | LOOSE | Cookie binding | 0% | 100% |

- **STRICT (24)**: For rate limiting. Zero false positives, catches 2/3 of same-user sessions.
- **LOOSE (40)**: For cookie binding enforcement. Catches incognito mode / browser variations.
- **Attack window**: Attacker must randomize 24-40 bits to evade both thresholds.

Results with 64-char canvas hashes:
- Same GPU avg distance: **19.8 bits** (well under threshold 24)
- Different GPU avg distance: **92.3 bits** (impossible to accidentally match)
- Same GPU match at ≤32: **91.7%**

### Canonical String Format

**DATA-ONLY format** (no field names, no separators) improves differentiation by ~12 bits average:
```rust
// GOOD: Data-only format
fn to_canonical_string(info: &BrowserInfo) -> String {
    let mut s = String::with_capacity(1024);
    s.push_str(&info.user_agent.to_lowercase());
    s.push_str(&info.platform.to_lowercase());
    s.push_str(&format!("{}{}{}", width, height, depth));  // no separators
    s.push_str(&info.canvas_hash);
    s.push_str(&info.webgl_renderer.to_lowercase());
    // ... high-entropy fields only, no field names
    s
}

// BAD: Field-separated format (wastes hash entropy on boilerplate)
"ua:mozilla/5.0...|screen:1920x1080x24|platform:macintel|..."
```

### Canvas Hash Weighting (64-char hashes)

Canvas hashes must be **64 hex characters** (not 8) to have proper weight in nilsimsa:

```javascript
// OLD: 8 chars - negligible weight vs 400-char font list
canvasHash: "-1a2b3c4d"

// NEW: 64 chars - proper weight, 8x more trigrams
canvasHash: "1a2b3c4d5e6f78901a2b3c4d5e6f78901a2b3c4d5e6f78901a2b3c4d5e6f7890"
```

**Why canvas fingerprinting works** (produces unique hash per GPU):
- GPU floating-point precision varies (NVIDIA vs AMD vs Intel vs Apple)
- Anti-aliasing algorithms are GPU-specific
- Driver version affects rendering paths
- Font rasterization uses GPU-accelerated paths

Same GPU = identical pixels = identical hash. Different GPU = slightly different pixels = different hash.

**Brave browser fuzzing**: Brave adds deterministic noise per session. Same session = consistent hash.
New incognito window = different hash (~36 bits distance). Use `isBrave: true` field to detect.

### Field Entropy Analysis

High-entropy fields (include these):
- `canvasHash` (64 chars), `webglCanvasHash` (64 chars) - GPU-dependent rendering
- `webglRenderer`, `webglVendor` - GPU identification strings
- `installedFonts` - varies by installed applications
- `userAgent`, `screenWidth/Height/Depth`

Low/zero-entropy fields (skip these):
- `audioFingerprint` (constant "function:44100:1")
- `maxTouchPoints` (0 for all desktop)
- `timezoneOffset`, `language`, `productSub`

### Discrimination Thresholds (Synthetic Data)

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
├── bktree_db.rs         # BK-tree implementation (slower than SIMD)
├── evicting_db.rs       # Evicting bucket with SIMD hamming distance
└── benchmarks/
    ├── mod.rs
    ├── common.rs              # Shared types (Stats, HashSize)
    ├── bit_uniformity.rs      # Bit flip uniformity analysis
    ├── segmentation.rs        # Hash segmentation for KV lookup
    ├── ratelimit_threshold.rs # Threshold analysis
    ├── simhash_comparison.rs  # Simhash vs nilsimsa comparison
    ├── bktree.rs              # BK-tree benchmark
    ├── ratelimiter.rs         # Rate limiter benchmark
    ├── discrimination.rs      # Browser discrimination analysis
    ├── realistic.rs           # Real fingerprint analysis, threshold selection
    ├── fingerprint_compare.rs # Compare WAF hash vs data-only hash
    └── evicting.rs            # Evicting bucket benchmark

fingerprints/                  # Real browser fingerprint samples
├── same/                      # Same browser (Brave), different sessions
└── different/                 # Different browsers (Chrome, Firefox, Safari)
```

Supporting files:
- `chrome_values.json` / `firefox_values.json` - Sample browser fingerprints (CSV format, not JSON)
- `run.sh` - Docker wrapper for cargo run/test
- `discrimination_*.png` - Generated discrimination threshold plots
- `bit_uniformity_*.png` - Generated bit flip distribution plots

## Testing Notes

### Real Fingerprint Format (fingerprints/ directory)

JSON files with `browser_info` object and `document_hash256` (WAF-computed hash):
```json
{
  "browser_info": {
    "userAgent": "Mozilla/5.0 ...",
    "canvasHash": "1a2b3c4d5e6f78901a2b3c4d5e6f78901a2b3c4d5e6f78901a2b3c4d5e6f7890",
    "webglCanvasHash": "f1e2d3c4b5a69870f1e2d3c4b5a69870f1e2d3c4b5a69870f1e2d3c4b5a69870",
    "webglRenderer": "Intel(R) HD Graphics 400",
    "installedFonts": "ARIAL,ARIAL BLACK,...",
    ...
  },
  "document_hash256": "bc8d01b01e5eb920..."
}
```

Note: `canvasHash` and `webglCanvasHash` are now 64 hex chars (8 segment hashes) for proper
nilsimsa weighting. Old format used 8-char truncated hashes which had negligible influence.

### Legacy CSV Format (chrome_values.json, firefox_values.json)

CSV strings wrapped in quotes (older format):
```
"Chrome, 128, 128.0.0, Mac OS X, ..."
```

The `find_csv_fields()` function in `modification.rs` parses these.

### Unit Tests

Use specific (not random) alterations for deterministic results. Tests cover:
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
    24,      // Hamming distance threshold (STRICT - 0% false positive)
    10_000,  // Capacity per bucket
));

// Start automatic rotation
let _rotation_handle = limiter.start_rotation_task();

// In request handler
if limiter.is_rate_limited(&fingerprint_hash, timestamp_secs, 100) {
    return Err(RateLimited);
}
```

For cookie binding enforcement, use threshold 40 (LOOSE - catches incognito/variations).

## Browser Fingerprinting Context

Browsers try to obfuscate fingerprints by randomizing:
- Canvas rendering (changes hash fields)
- Font enumeration order (sorting neutralizes this)
- WebGL renderer strings
- Audio context values

### Recommendations

1. **Use 64-char canvas hashes** - 8-char hashes have negligible weight. Use 8-segment hashing for 64 chars.
2. **Use data-only canonical format** - No field names, no separators. Improves differentiation by ~12 bits.
3. **Sort font lists before hashing** - Neutralizes enumeration order randomization.
4. **Use 256-bit hashes** - Better discrimination than 64-bit truncated.
5. **Skip low-entropy fields** - audioFingerprint, maxTouchPoints, timezoneOffset add noise, not signal.
6. **Detect Brave browser** - Brave spoofs Chrome UA but has `isBrave: true` in fingerprint. Fuzzes canvas per-session.
7. **Two thresholds** - Use 24 for rate limiting (strict), 40 for cookie binding (loose).
