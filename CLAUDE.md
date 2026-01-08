# Nilsimsa Hash Correctness Testing

Rust project for analyzing nilsimsa locality-sensitive hash behavior, specifically for browser fingerprint discrimination.

## Quick Reference

```bash
./run.sh        # Run main statistical analysis
./run.sh test   # Run unit tests (17 tests)
```

All execution happens in Docker (see docker-compose.yml). Output files (PNGs) appear in project root.

## What This Project Does

Tests whether nilsimsa hashes can distinguish browser fingerprints (Chrome vs Firefox) despite:
- Random character changes (scattered modifications)
- Random word/field changes (concentrated modifications)
- Insertions and removals
- Font list order randomization (privacy countermeasure)

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

## File Structure

- `src/main.rs` - Statistical analysis + unit tests
- `chrome_values.json` / `firefox_values.json` - Sample browser fingerprints (CSV format, not JSON)
- `run.sh` - Docker wrapper for cargo run/test
- `discrimination_*.png` - Generated comparison plots

## Testing Notes

The fingerprint files are actually CSV strings wrapped in quotes, not JSON objects:
```
"Chrome, 128, 128.0.0, Mac OS X, ..."
```

The `find_csv_fields()` function parses these comma-separated values.

Unit tests use specific (not random) alterations for deterministic results. Tests cover:
- Hash consistency
- Browser discrimination
- Font list sorting (neutralizes order randomization)
- Edge cases

## Browser Fingerprinting Context

Browsers try to obfuscate fingerprints by randomizing:
- Canvas rendering (changes hash fields)
- Font enumeration order (sorting neutralizes this)
- WebGL renderer strings
- Audio context values

Recommendation: Sort font lists before hashing, use 256-bit, consider removing known-randomized
fields if you only need browser/OS identification rather than unique user tracking.
