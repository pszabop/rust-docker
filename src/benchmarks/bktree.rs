//! BK-tree benchmark for fuzzy hash lookup

use std::time::Instant;
use bktree::BkTree;

use crate::hash::{stable_document_hash_256, Hash256, hash256_hamming_distance};
use crate::modification::{randomly_modify_letters, generate_random_document};

pub fn run() {
    println!("\n============================================================");
    println!("=== BK-TREE BENCHMARK FOR FUZZY HASH LOOKUP ===");
    println!("============================================================\n");

    let mut rng = rand::thread_rng();

    // Test different thresholds at fixed tree size
    const TREE_SIZE: usize = 10_000;
    const DOC_LENGTH: usize = 1000;
    const QUERY_COUNT: usize = 1000;
    let thresholds: Vec<isize> = vec![20, 30, 40];

    // Generate documents once
    println!("Generating {} random fingerprints...", TREE_SIZE);
    let documents: Vec<String> = (0..TREE_SIZE)
        .map(|_| generate_random_document(&mut rng, DOC_LENGTH))
        .collect();

    let hashes: Vec<Hash256> = documents.iter()
        .map(|d| Hash256(stable_document_hash_256(d)))
        .collect();

    // Build BK-tree once
    println!("Building BK-tree...");
    let build_start = Instant::now();
    let mut tree: BkTree<Hash256> = BkTree::new(hash256_hamming_distance);
    for hash in &hashes {
        tree.insert(hash.clone());
    }
    let build_time = build_start.elapsed();
    println!("Build time: {:?} ({:.2}μs per insert)\n", build_time,
             build_time.as_micros() as f64 / TREE_SIZE as f64);

    for &threshold in &thresholds {
        println!("{}", "=".repeat(70));
        println!("THRESHOLD: {} bits (tree size: {})", threshold, TREE_SIZE);
        println!("{}", "=".repeat(70));

        // Benchmark exact lookups (query hash that exists in tree)
        println!("\n--- Exact Lookups (hash exists in tree) ---");
        let mut exact_times = Vec::with_capacity(QUERY_COUNT);
        let mut exact_found_counts = Vec::with_capacity(QUERY_COUNT);

        for i in 0..QUERY_COUNT {
            let query = hashes[i % hashes.len()].clone();
            let start = Instant::now();
            let results = tree.find(query, threshold);
            exact_times.push(start.elapsed().as_micros() as u64);
            exact_found_counts.push(results.len());
        }

        let avg_exact_time = exact_times.iter().sum::<u64>() as f64 / QUERY_COUNT as f64;
        let max_exact_time = *exact_times.iter().max().unwrap();
        let avg_found = exact_found_counts.iter().sum::<usize>() as f64 / QUERY_COUNT as f64;
        println!("  Avg query time: {:.1}μs, Max: {}μs", avg_exact_time, max_exact_time);
        println!("  Avg matches found: {:.1}", avg_found);

        // Benchmark fuzzy lookups (query slightly modified hashes)
        println!("\n--- Fuzzy Lookups (16 chars fuzzed) ---");
        let mut fuzzy_times = Vec::with_capacity(QUERY_COUNT);
        let mut fuzzy_found_counts = Vec::with_capacity(QUERY_COUNT);
        let mut fuzzy_hit_rate = 0;

        for i in 0..QUERY_COUNT {
            let original_doc = &documents[i % documents.len()];
            let fuzzed_doc = randomly_modify_letters(original_doc, 16);
            let fuzzed_hash = Hash256(stable_document_hash_256(&fuzzed_doc));

            let start = Instant::now();
            let results = tree.find(fuzzed_hash, threshold);
            fuzzy_times.push(start.elapsed().as_micros() as u64);
            fuzzy_found_counts.push(results.len());

            if !results.is_empty() {
                fuzzy_hit_rate += 1;
            }
        }

        let avg_fuzzy_time = fuzzy_times.iter().sum::<u64>() as f64 / QUERY_COUNT as f64;
        let max_fuzzy_time = *fuzzy_times.iter().max().unwrap();
        let avg_fuzzy_found = fuzzy_found_counts.iter().sum::<usize>() as f64 / QUERY_COUNT as f64;
        println!("  Avg query time: {:.1}μs, Max: {}μs", avg_fuzzy_time, max_fuzzy_time);
        println!("  Avg matches found: {:.1}", avg_fuzzy_found);
        println!("  Hit rate (found original): {:.1}%", fuzzy_hit_rate as f64 / QUERY_COUNT as f64 * 100.0);

        // Benchmark miss lookups (query hash that doesn't exist)
        println!("\n--- Miss Lookups (new hash, no match expected) ---");
        let mut miss_times = Vec::with_capacity(QUERY_COUNT);
        let mut miss_found_counts = Vec::with_capacity(QUERY_COUNT);

        for _ in 0..QUERY_COUNT {
            let new_doc = generate_random_document(&mut rng, DOC_LENGTH);
            let new_hash = Hash256(stable_document_hash_256(&new_doc));

            let start = Instant::now();
            let results = tree.find(new_hash, threshold);
            miss_times.push(start.elapsed().as_micros() as u64);
            miss_found_counts.push(results.len());
        }

        let avg_miss_time = miss_times.iter().sum::<u64>() as f64 / QUERY_COUNT as f64;
        let max_miss_time = *miss_times.iter().max().unwrap();
        let false_positive_rate = miss_found_counts.iter().filter(|&&c| c > 0).count();
        println!("  Avg query time: {:.1}μs, Max: {}μs", avg_miss_time, max_miss_time);
        println!("  False positives: {} / {} ({:.2}%)",
                 false_positive_rate, QUERY_COUNT,
                 false_positive_rate as f64 / QUERY_COUNT as f64 * 100.0);

        // Compare to brute force
        println!("\n--- Brute Force Comparison ---");
        let mut brute_times = Vec::with_capacity(100);

        for i in 0..100 {
            let query = &hashes[i % hashes.len()];
            let start = Instant::now();
            let _matches: Vec<_> = hashes.iter()
                .filter(|h| hash256_hamming_distance(query, *h) <= threshold)
                .collect();
            brute_times.push(start.elapsed().as_micros() as u64);
        }

        let avg_brute_time = brute_times.iter().sum::<u64>() as f64 / 100.0;
        println!("  Avg brute force time: {:.1}μs", avg_brute_time);
        println!("  BK-tree speedup: {:.1}x", avg_brute_time / avg_exact_time);

        println!();
    }

    // SIMD-optimized brute force
    println!("{}", "=".repeat(70));
    println!("SIMD BRUTE FORCE (u64-based, cache-friendly)");
    println!("{}", "=".repeat(70));

    // Convert hashes to u64 arrays for faster operations
    let hashes_u64: Vec<[u64; 4]> = hashes.iter()
        .map(|h| {
            let mut arr = [0u64; 4];
            for i in 0..4 {
                arr[i] = u64::from_le_bytes([
                    h.0[i*8], h.0[i*8+1], h.0[i*8+2], h.0[i*8+3],
                    h.0[i*8+4], h.0[i*8+5], h.0[i*8+6], h.0[i*8+7],
                ]);
            }
            arr
        })
        .collect();

    let threshold_u32 = 30u32; // Use threshold 30 (the sweet spot)

    println!("\nThreshold: {} bits", threshold_u32);

    // Benchmark SIMD brute force
    let mut simd_times = Vec::with_capacity(QUERY_COUNT);
    let mut simd_found = Vec::with_capacity(QUERY_COUNT);

    for i in 0..QUERY_COUNT {
        let query_hash = &hashes[i % hashes.len()];
        let query_u64: [u64; 4] = {
            let mut arr = [0u64; 4];
            for j in 0..4 {
                arr[j] = u64::from_le_bytes([
                    query_hash.0[j*8], query_hash.0[j*8+1], query_hash.0[j*8+2], query_hash.0[j*8+3],
                    query_hash.0[j*8+4], query_hash.0[j*8+5], query_hash.0[j*8+6], query_hash.0[j*8+7],
                ]);
            }
            arr
        };

        let start = Instant::now();
        let mut found = 0usize;
        for h in &hashes_u64 {
            let dist = (query_u64[0] ^ h[0]).count_ones()
                     + (query_u64[1] ^ h[1]).count_ones()
                     + (query_u64[2] ^ h[2]).count_ones()
                     + (query_u64[3] ^ h[3]).count_ones();
            if dist <= threshold_u32 {
                found += 1;
            }
        }
        simd_times.push(start.elapsed().as_micros() as u64);
        simd_found.push(found);
    }

    let avg_simd_time = simd_times.iter().sum::<u64>() as f64 / QUERY_COUNT as f64;
    let max_simd_time = *simd_times.iter().max().unwrap();
    let avg_simd_found = simd_found.iter().sum::<usize>() as f64 / QUERY_COUNT as f64;
    println!("  Avg query time: {:.1}μs, Max: {}μs", avg_simd_time, max_simd_time);
    println!("  Avg matches found: {:.1}", avg_simd_found);

    // Fuzzy lookup with SIMD
    println!("\n--- SIMD Fuzzy Lookups (16 chars fuzzed) ---");
    let mut simd_fuzzy_times = Vec::with_capacity(QUERY_COUNT);
    let mut simd_fuzzy_hit = 0;

    for i in 0..QUERY_COUNT {
        let original_doc = &documents[i % documents.len()];
        let fuzzed_doc = randomly_modify_letters(original_doc, 16);
        let fuzzed_hash = stable_document_hash_256(&fuzzed_doc);
        let query_u64: [u64; 4] = {
            let mut arr = [0u64; 4];
            for j in 0..4 {
                arr[j] = u64::from_le_bytes([
                    fuzzed_hash[j*8], fuzzed_hash[j*8+1], fuzzed_hash[j*8+2], fuzzed_hash[j*8+3],
                    fuzzed_hash[j*8+4], fuzzed_hash[j*8+5], fuzzed_hash[j*8+6], fuzzed_hash[j*8+7],
                ]);
            }
            arr
        };

        let start = Instant::now();
        let mut found = false;
        for h in &hashes_u64 {
            let dist = (query_u64[0] ^ h[0]).count_ones()
                     + (query_u64[1] ^ h[1]).count_ones()
                     + (query_u64[2] ^ h[2]).count_ones()
                     + (query_u64[3] ^ h[3]).count_ones();
            if dist <= threshold_u32 {
                found = true;
                break; // Early exit on first match
            }
        }
        simd_fuzzy_times.push(start.elapsed().as_micros() as u64);
        if found { simd_fuzzy_hit += 1; }
    }

    let avg_simd_fuzzy = simd_fuzzy_times.iter().sum::<u64>() as f64 / QUERY_COUNT as f64;
    println!("  Avg query time: {:.1}μs (with early exit)", avg_simd_fuzzy);
    println!("  Hit rate: {:.1}%", simd_fuzzy_hit as f64 / QUERY_COUNT as f64 * 100.0);

    println!("\n{}", "=".repeat(70));
    println!("SUMMARY");
    println!("{}", "=".repeat(70));
    println!("\nTarget: 200μs per lookup for rate limiting");
    println!("BK-tree: ~9000μs (45x too slow)");
    println!("SIMD brute force: {:.1}μs", avg_simd_time);
    if avg_simd_time <= 200.0 {
        println!("  ✓ SIMD brute force MEETS the target!");
    } else {
        println!("  ✗ Still {:.1}x too slow", avg_simd_time / 200.0);
    }
}
