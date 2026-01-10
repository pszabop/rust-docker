//! Benchmark comparing eviction strategies for the fuzzy hash database
//!
//! Tests combined single-pass vs separate passes for search + victim finding

use std::hint::black_box;
use std::time::Instant;

use crate::evicting_db::EvictingBucket;
use crate::bktree_db::BKTreeDB;
use crate::hash::hamming_distance_u64x4;

/// Generate a random 256-bit hash
fn random_hash() -> [u64; 4] {
    [
        rand::random::<u64>(),
        rand::random::<u64>(),
        rand::random::<u64>(),
        rand::random::<u64>(),
    ]
}

/// Baseline: raw scan with no locking, no eviction logic
/// This is the theoretical maximum speed
fn benchmark_raw_scan(entries: &[[u64; 4]], queries: &[[u64; 4]], threshold: u32) -> (std::time::Duration, u32) {
    let mut found = 0u32;

    let start = Instant::now();
    for query in queries {
        for entry in entries {
            if hamming_distance_u64x4(black_box(query), black_box(entry)) <= threshold {
                found += 1;
                break;
            }
        }
    }
    (start.elapsed(), found)
}

/// Baseline: raw scan that always completes (no early exit)
fn benchmark_raw_scan_full(entries: &[[u64; 4]], queries: &[[u64; 4]]) -> (std::time::Duration, u32) {
    let mut total_min = 0u32;

    let start = Instant::now();
    for query in queries {
        let mut min_dist = u32::MAX;
        for entry in entries {
            let d = hamming_distance_u64x4(black_box(query), black_box(entry));
            if d < min_dist {
                min_dist = d;
            }
        }
        total_min += min_dist;
    }
    (start.elapsed(), total_min)
}

pub fn run() {
    println!("\n============================================================");
    println!("=== EVICTING DATABASE BENCHMARK ===");
    println!("============================================================\n");

    // Configuration
    const CAPACITY: usize = 10_000;
    const THRESHOLD: u32 = 32;
    const ITERATIONS: usize = 10_000;

    println!("Configuration:");
    println!("  Capacity: {} entries", CAPACITY);
    println!("  Hamming threshold: {}", THRESHOLD);
    println!("  Entry size: {} bytes (hash) + {} bytes (record) = {} bytes",
             32, 8, 40);
    println!("  Total memory: {} KB", CAPACITY * 40 / 1024);
    println!("  Victim window: 1/8 = {} entries\n", CAPACITY / 8);

    // ========================================================================
    // Test 0: BASELINE - Raw scan speed (no locking, no eviction)
    // ========================================================================
    println!("--- Test 0: BASELINE - Raw Scan (no locks, no eviction) ---\n");

    let baseline_entries: Vec<[u64; 4]> = (0..CAPACITY)
        .map(|i| {
            let x = (i as u64).wrapping_mul(0x517cc1b727220a95);
            [x, x.rotate_left(17), x.rotate_left(34), x.rotate_left(51)]
        })
        .collect();

    // Queries that won't match (forces full scan)
    let miss_queries: Vec<[u64; 4]> = (0..ITERATIONS)
        .map(|i| {
            let q = (i as u64).wrapping_mul(0x9e3779b97f4a7c15);
            [q, q.rotate_left(13), q.rotate_left(26), q.rotate_left(39)]
        })
        .collect();

    // Queries that will match (early exit)
    let hit_queries: Vec<[u64; 4]> = (0..ITERATIONS)
        .map(|i| baseline_entries[i % CAPACITY])
        .collect();

    let (raw_miss_time, checksum) = benchmark_raw_scan_full(&baseline_entries, &miss_queries);
    println!("  Full scan (miss): {:?} ({:.2}μs/op, {:.1}ns/entry) [checksum: {}]",
             raw_miss_time,
             raw_miss_time.as_micros() as f64 / ITERATIONS as f64,
             raw_miss_time.as_nanos() as f64 / ITERATIONS as f64 / CAPACITY as f64,
             checksum);

    let (raw_hit_time, found) = benchmark_raw_scan(&baseline_entries, &hit_queries, THRESHOLD);
    println!("  Early exit (hit): {:?} ({:.2}μs/op) [found: {}]",
             raw_hit_time,
             raw_hit_time.as_micros() as f64 / ITERATIONS as f64,
             found);

    let baseline_miss_us = raw_miss_time.as_micros() as f64 / ITERATIONS as f64;
    let baseline_ns_per_entry = raw_miss_time.as_nanos() as f64 / ITERATIONS as f64 / CAPACITY as f64;

    println!("\n  >>> BASELINE: {:.2}μs per full scan, {:.1}ns per entry <<<\n",
             baseline_miss_us, baseline_ns_per_entry);

    // ========================================================================
    // Test 1: 100% Hit Rate (existing entries)
    // ========================================================================
    println!("--- Test 1: 100% Hit Rate (searching for existing entries) ---\n");

    let db_combined = EvictingBucket::new(CAPACITY, THRESHOLD);
    let db_separate = EvictingBucket::new(CAPACITY, THRESHOLD);

    // Fill databases with same data
    db_combined.fill_random(1000);
    db_separate.fill_random(1000);

    // Get some hashes that exist in the database
    let existing_hashes: Vec<[u64; 4]> = (0..ITERATIONS)
        .map(|_| random_hash())
        .collect();

    // Fill both DBs with these hashes so we can find them
    for (i, hash) in existing_hashes.iter().enumerate().take(CAPACITY) {
        // Directly set hashes (hacky but works for benchmark)
        let _ = db_combined.find_or_insert_combined(hash, 1000);
        let _ = db_separate.find_or_insert_separate(hash, 1000);
    }

    // Benchmark combined approach - hits
    let start = Instant::now();
    for hash in existing_hashes.iter().take(ITERATIONS) {
        let _ = db_combined.find_or_insert_combined(hash, 2000);
    }
    let combined_hit_time = start.elapsed();

    // Benchmark separate approach - hits
    let start = Instant::now();
    for hash in existing_hashes.iter().take(ITERATIONS) {
        let _ = db_separate.find_or_insert_separate(hash, 2000);
    }
    let separate_hit_time = start.elapsed();

    println!("  Combined pass:  {:?} ({:.2}μs/op)",
             combined_hit_time,
             combined_hit_time.as_micros() as f64 / ITERATIONS as f64);
    println!("  Separate passes: {:?} ({:.2}μs/op)",
             separate_hit_time,
             separate_hit_time.as_micros() as f64 / ITERATIONS as f64);
    println!("  Ratio: {:.2}x",
             combined_hit_time.as_nanos() as f64 / separate_hit_time.as_nanos() as f64);

    // ========================================================================
    // Test 2: 100% Miss Rate (random hash attack simulation)
    // ========================================================================
    println!("\n--- Test 2: 100% Miss Rate (random hash attack) ---\n");

    let db_combined = EvictingBucket::new(CAPACITY, THRESHOLD);
    let db_separate = EvictingBucket::new(CAPACITY, THRESHOLD);

    // Fill databases
    db_combined.fill_random(1000);
    db_separate.fill_random(1000);

    // Generate completely random hashes (will never match)
    let attack_hashes: Vec<[u64; 4]> = (0..ITERATIONS)
        .map(|_| random_hash())
        .collect();

    // Benchmark combined approach - misses (evictions)
    let start = Instant::now();
    for hash in &attack_hashes {
        let _ = db_combined.find_or_insert_combined(hash, 2000);
    }
    let combined_miss_time = start.elapsed();

    // Benchmark separate approach - misses (evictions)
    let start = Instant::now();
    for hash in &attack_hashes {
        let _ = db_separate.find_or_insert_separate(hash, 2000);
    }
    let separate_miss_time = start.elapsed();

    println!("  Combined pass:  {:?} ({:.2}μs/op)",
             combined_miss_time,
             combined_miss_time.as_micros() as f64 / ITERATIONS as f64);
    println!("  Separate passes: {:?} ({:.2}μs/op)",
             separate_miss_time,
             separate_miss_time.as_micros() as f64 / ITERATIONS as f64);
    println!("  Ratio: {:.2}x",
             combined_miss_time.as_nanos() as f64 / separate_miss_time.as_nanos() as f64);

    // ========================================================================
    // Test 3: Mixed workload (50% hit, 50% miss)
    // ========================================================================
    println!("\n--- Test 3: 50/50 Mixed Workload ---\n");

    let db_combined = EvictingBucket::new(CAPACITY, THRESHOLD);
    let db_separate = EvictingBucket::new(CAPACITY, THRESHOLD);

    // Insert some known hashes
    let known_hashes: Vec<[u64; 4]> = (0..CAPACITY / 2)
        .map(|_| random_hash())
        .collect();

    for hash in &known_hashes {
        let _ = db_combined.find_or_insert_combined(hash, 1000);
        let _ = db_separate.find_or_insert_separate(hash, 1000);
    }

    // Fill rest with random
    db_combined.fill_random(1000);
    db_separate.fill_random(1000);

    // Re-insert known hashes so they're findable
    for hash in &known_hashes {
        let _ = db_combined.find_or_insert_combined(hash, 1000);
        let _ = db_separate.find_or_insert_separate(hash, 1000);
    }

    // Mixed workload: alternate between known and random
    let mut mixed_hashes = Vec::with_capacity(ITERATIONS);
    for i in 0..ITERATIONS {
        if i % 2 == 0 && i / 2 < known_hashes.len() {
            mixed_hashes.push(known_hashes[i / 2 % known_hashes.len()]);
        } else {
            mixed_hashes.push(random_hash());
        }
    }

    // Benchmark combined
    let start = Instant::now();
    for hash in &mixed_hashes {
        let _ = db_combined.find_or_insert_combined(hash, 2000);
    }
    let combined_mixed_time = start.elapsed();

    // Benchmark separate
    let start = Instant::now();
    for hash in &mixed_hashes {
        let _ = db_separate.find_or_insert_separate(hash, 2000);
    }
    let separate_mixed_time = start.elapsed();

    println!("  Combined pass:  {:?} ({:.2}μs/op)",
             combined_mixed_time,
             combined_mixed_time.as_micros() as f64 / ITERATIONS as f64);
    println!("  Separate passes: {:?} ({:.2}μs/op)",
             separate_mixed_time,
             separate_mixed_time.as_micros() as f64 / ITERATIONS as f64);
    println!("  Ratio: {:.2}x",
             combined_mixed_time.as_nanos() as f64 / separate_mixed_time.as_nanos() as f64);

    // ========================================================================
    // Test 4: Scaling with capacity
    // ========================================================================
    println!("\n--- Test 4: Scaling with Capacity (100% miss rate) ---\n");
    println!("  {:>10} {:>15} {:>15} {:>10}", "Capacity", "Combined", "Separate", "Ratio");

    for &cap in &[1_000, 5_000, 10_000, 25_000, 50_000] {
        let db_combined = EvictingBucket::new(cap, THRESHOLD);
        let db_separate = EvictingBucket::new(cap, THRESHOLD);

        db_combined.fill_random(1000);
        db_separate.fill_random(1000);

        let iters = 1000;
        let hashes: Vec<[u64; 4]> = (0..iters).map(|_| random_hash()).collect();

        let start = Instant::now();
        for hash in &hashes {
            let _ = db_combined.find_or_insert_combined(hash, 2000);
        }
        let combined_time = start.elapsed();

        let start = Instant::now();
        for hash in &hashes {
            let _ = db_separate.find_or_insert_separate(hash, 2000);
        }
        let separate_time = start.elapsed();

        let combined_us = combined_time.as_micros() as f64 / iters as f64;
        let separate_us = separate_time.as_micros() as f64 / iters as f64;
        let ratio = combined_us / separate_us;

        println!("  {:>10} {:>12.2}μs {:>12.2}μs {:>10.2}x",
                 cap, combined_us, separate_us, ratio);
    }

    // ========================================================================
    // Summary
    // ========================================================================
    println!("\n============================================================");
    println!("SUMMARY");
    println!("============================================================");
    println!("\nHit path (existing entry found):");
    println!("  - Combined: Does victim tracking even on hits (1/8 window)");
    println!("  - Separate: Clean search, no victim tracking overhead");
    println!("\nMiss path (eviction needed):");
    println!("  - Combined: One pass through data");
    println!("  - Separate: Two passes (search + victim scan)");
    println!("\nRecommendation: Use whichever is faster for your expected hit rate.");
    println!("  High hit rate (>80%): Separate passes likely better");
    println!("  Low hit rate / under attack: Combined likely better");

    // ========================================================================
    // Test 5: BK-Tree vs Linear Scan
    // ========================================================================
    println!("\n============================================================");
    println!("=== BK-TREE vs LINEAR SCAN COMPARISON ===");
    println!("============================================================\n");

    for &capacity in &[5_000, 10_000, 25_000, 50_000, 100_000] {
        println!("--- Capacity: {} entries ---\n", capacity);

        // Linear scan (evicting bucket)
        let linear_db = EvictingBucket::new(capacity, THRESHOLD);
        linear_db.fill_random(1000);

        // BK-Tree (1000 entries per leaf for good cache behavior)
        let bktree_db = BKTreeDB::new(THRESHOLD, 1000, capacity);
        bktree_db.fill_random(capacity, 1000);

        // Generate test queries (all misses for worst case)
        let test_queries: Vec<[u64; 4]> = (0..1000)
            .map(|_| random_hash())
            .collect();

        // Benchmark linear
        let start = Instant::now();
        for query in &test_queries {
            let _ = linear_db.find_or_insert_combined(query, 2000);
        }
        let linear_time = start.elapsed();

        // Benchmark BK-tree
        let start = Instant::now();
        for query in &test_queries {
            let _ = bktree_db.find_or_insert(query, 2000);
        }
        let bktree_time = start.elapsed();

        let linear_us = linear_time.as_micros() as f64 / 1000.0;
        let bktree_us = bktree_time.as_micros() as f64 / 1000.0;
        let speedup = linear_us / bktree_us;

        println!("  Linear scan:  {:.2}μs/op (per core)", linear_us);
        println!("  BK-Tree:      {:.2}μs/op (per core), {} nodes", bktree_us, bktree_db.node_count());
        println!("  Speedup:      {:.2}x", speedup);

        if speedup > 1.0 {
            println!("  >>> BK-Tree is {:.1}x FASTER <<<", speedup);
        } else {
            println!("  >>> Linear scan is {:.1}x faster <<<", 1.0 / speedup);
        }
        println!();
    }

    println!("============================================================");
    println!("CONCLUSION");
    println!("============================================================");
    println!("\nAt 200μs budget per core:");
    println!("  - Linear scan: ~25K entries max");
    println!("  - BK-Tree: Potentially much higher if speedup is significant");
    println!("\nBK-Tree works best when:");
    println!("  - Large dataset (>25K entries)");
    println!("  - Threshold is small relative to hash size (32/256 = 12.5%)");
    println!("  - Data is well-distributed (not clustered)");
}
