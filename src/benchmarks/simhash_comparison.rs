//! Simhash vs Nilsimsa segment stability comparison

use simhash::simhash;

use crate::hash::{stable_document_hash_256, hamming_distance_256};
use crate::modification::{randomly_modify_letters, generate_random_document};

/// Compute simhash (64-bit) for a document
fn compute_simhash(input: &str) -> u64 {
    simhash(input)
}

/// Extract 16-bit segments from 64-bit simhash (4 segments)
fn extract_simhash_segments_4(hash: u64) -> [u16; 4] {
    [
        (hash >> 48) as u16,
        (hash >> 32) as u16,
        (hash >> 16) as u16,
        hash as u16,
    ]
}

/// Extract 8-bit segments from 64-bit simhash (8 segments)
fn extract_simhash_segments_8(hash: u64) -> [u8; 8] {
    [
        (hash >> 56) as u8,
        (hash >> 48) as u8,
        (hash >> 40) as u8,
        (hash >> 32) as u8,
        (hash >> 24) as u8,
        (hash >> 16) as u8,
        (hash >> 8) as u8,
        hash as u8,
    ]
}

fn hamming_distance_64_bits(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Extract 24-bit segments from a 256-bit hash
fn extract_segments(hash: &[u8; 32], num_segments: usize) -> Vec<u32> {
    let mut segments = Vec::with_capacity(num_segments);
    let starts: Vec<usize> = vec![0, 32, 64, 96, 128, 160, 192, 224];

    for &bit_start in starts.iter().take(num_segments) {
        let byte_start = bit_start / 8;
        let seg: u32 = ((hash[byte_start] as u32) << 16)
            | ((hash[byte_start + 1] as u32) << 8)
            | (hash[byte_start + 2] as u32);
        segments.push(seg);
    }

    segments
}

pub fn run() {
    println!("\n============================================================");
    println!("=== SIMHASH vs NILSIMSA SEGMENT STABILITY COMPARISON ===");
    println!("============================================================\n");
    println!("Testing whether simhash (64-bit) has better segment stability than nilsimsa.\n");

    let mut rng = rand::thread_rng();

    const NUM_DOCUMENTS: usize = 1000;
    const DOC_LENGTH: usize = 1000;

    let mod_amounts = vec![4, 8, 16, 32, 64, 128];

    println!("Generating {} random documents...\n", NUM_DOCUMENTS);

    let documents: Vec<String> = (0..NUM_DOCUMENTS)
        .map(|_| generate_random_document(&mut rng, DOC_LENGTH))
        .collect();

    // Compute both hash types for all documents
    let nilsimsa_hashes: Vec<[u8; 32]> = documents.iter()
        .map(|d| stable_document_hash_256(d))
        .collect();

    let simhash_hashes: Vec<u64> = documents.iter()
        .map(|d| compute_simhash(d))
        .collect();

    // Test segment stability for both
    println!("{}", "=".repeat(80));
    println!("SEGMENT STABILITY: At least one segment survives fuzzing?");
    println!("{}", "=".repeat(80));

    println!("\n--- NILSIMSA (256-bit, 8 segments of 24 bits) ---\n");
    println!("{:>8} | {:>15} | {:>12} | {:>15}", "ModChars", "AtLeast1Match", "AvgMatches", "NoneMatch");
    println!("{}", "-".repeat(60));

    for &mod_chars in &mod_amounts {
        let mut at_least_one = 0;
        let mut total_matches = 0;
        let mut none_match = 0;

        for (idx, doc) in documents.iter().enumerate() {
            let modified = randomly_modify_letters(doc, mod_chars);
            let mod_hash = stable_document_hash_256(&modified);
            let orig_segs = extract_segments(&nilsimsa_hashes[idx], 8);
            let mod_segs = extract_segments(&mod_hash, 8);

            let matches: usize = orig_segs.iter().zip(mod_segs.iter())
                .filter(|(a, b)| a == b).count();

            total_matches += matches;
            if matches >= 1 { at_least_one += 1; }
            if matches == 0 { none_match += 1; }
        }

        println!("{:>8} | {:>14.1}% | {:>12.2} | {:>14.1}%",
                 mod_chars,
                 at_least_one as f64 / NUM_DOCUMENTS as f64 * 100.0,
                 total_matches as f64 / NUM_DOCUMENTS as f64,
                 none_match as f64 / NUM_DOCUMENTS as f64 * 100.0);
    }

    println!("\n--- SIMHASH (64-bit, 4 segments of 16 bits) ---\n");
    println!("{:>8} | {:>15} | {:>12} | {:>15}", "ModChars", "AtLeast1Match", "AvgMatches", "NoneMatch");
    println!("{}", "-".repeat(60));

    for &mod_chars in &mod_amounts {
        let mut at_least_one = 0;
        let mut total_matches = 0;
        let mut none_match = 0;

        for (idx, doc) in documents.iter().enumerate() {
            let modified = randomly_modify_letters(doc, mod_chars);
            let mod_hash = compute_simhash(&modified);
            let orig_segs = extract_simhash_segments_4(simhash_hashes[idx]);
            let mod_segs = extract_simhash_segments_4(mod_hash);

            let matches: usize = orig_segs.iter().zip(mod_segs.iter())
                .filter(|(a, b)| a == b).count();

            total_matches += matches;
            if matches >= 1 { at_least_one += 1; }
            if matches == 0 { none_match += 1; }
        }

        println!("{:>8} | {:>14.1}% | {:>12.2} | {:>14.1}%",
                 mod_chars,
                 at_least_one as f64 / NUM_DOCUMENTS as f64 * 100.0,
                 total_matches as f64 / NUM_DOCUMENTS as f64,
                 none_match as f64 / NUM_DOCUMENTS as f64 * 100.0);
    }

    println!("\n--- SIMHASH (64-bit, 8 segments of 8 bits) ---\n");
    println!("{:>8} | {:>15} | {:>12} | {:>15}", "ModChars", "AtLeast1Match", "AvgMatches", "NoneMatch");
    println!("{}", "-".repeat(60));

    for &mod_chars in &mod_amounts {
        let mut at_least_one = 0;
        let mut total_matches = 0;
        let mut none_match = 0;

        for (idx, doc) in documents.iter().enumerate() {
            let modified = randomly_modify_letters(doc, mod_chars);
            let mod_hash = compute_simhash(&modified);
            let orig_segs = extract_simhash_segments_8(simhash_hashes[idx]);
            let mod_segs = extract_simhash_segments_8(mod_hash);

            let matches: usize = orig_segs.iter().zip(mod_segs.iter())
                .filter(|(a, b)| a == b).count();

            total_matches += matches;
            if matches >= 1 { at_least_one += 1; }
            if matches == 0 { none_match += 1; }
        }

        println!("{:>8} | {:>14.1}% | {:>12.2} | {:>14.1}%",
                 mod_chars,
                 at_least_one as f64 / NUM_DOCUMENTS as f64 * 100.0,
                 total_matches as f64 / NUM_DOCUMENTS as f64,
                 none_match as f64 / NUM_DOCUMENTS as f64 * 100.0);
    }

    // Hamming distance comparison
    println!("\n{}", "=".repeat(80));
    println!("HAMMING DISTANCE COMPARISON: Same browser fuzzed");
    println!("{}", "=".repeat(80));

    println!("\n{:>8} | {:>20} | {:>20}", "ModChars", "Nilsimsa (of 256)", "Simhash (of 64)");
    println!("{}", "-".repeat(55));

    for &mod_chars in &mod_amounts {
        let mut nilsimsa_dists: Vec<u32> = Vec::new();
        let mut simhash_dists: Vec<u32> = Vec::new();

        for (idx, doc) in documents.iter().enumerate() {
            let modified = randomly_modify_letters(doc, mod_chars);

            let nil_mod = stable_document_hash_256(&modified);
            let sim_mod = compute_simhash(&modified);

            nilsimsa_dists.push(hamming_distance_256(&nilsimsa_hashes[idx], &nil_mod));
            simhash_dists.push(hamming_distance_64_bits(simhash_hashes[idx], sim_mod));
        }

        let nil_mean = nilsimsa_dists.iter().map(|&d| d as f64).sum::<f64>() / NUM_DOCUMENTS as f64;
        let sim_mean = simhash_dists.iter().map(|&d| d as f64).sum::<f64>() / NUM_DOCUMENTS as f64;

        let nil_max = *nilsimsa_dists.iter().max().unwrap();
        let sim_max = *simhash_dists.iter().max().unwrap();

        println!("{:>8} | mean={:>5.1}, max={:>3} | mean={:>5.1}, max={:>3}",
                 mod_chars, nil_mean, nil_max, sim_mean, sim_max);
    }

    // Different browser comparison
    println!("\n{}", "=".repeat(80));
    println!("DIFFERENT BROWSER DISTANCES");
    println!("{}", "=".repeat(80));

    let mut nil_diff_dists: Vec<u32> = Vec::new();
    let mut sim_diff_dists: Vec<u32> = Vec::new();

    for i in 0..NUM_DOCUMENTS.min(200) {
        for j in (i+1)..NUM_DOCUMENTS.min(200) {
            nil_diff_dists.push(hamming_distance_256(&nilsimsa_hashes[i], &nilsimsa_hashes[j]));
            sim_diff_dists.push(hamming_distance_64_bits(simhash_hashes[i], simhash_hashes[j]));
        }
    }

    let nil_mean = nil_diff_dists.iter().map(|&d| d as f64).sum::<f64>() / nil_diff_dists.len() as f64;
    let nil_min = *nil_diff_dists.iter().min().unwrap();
    let sim_mean = sim_diff_dists.iter().map(|&d| d as f64).sum::<f64>() / sim_diff_dists.len() as f64;
    let sim_min = *sim_diff_dists.iter().min().unwrap();

    println!("\nNilsimsa (256-bit): mean={:.1}, min={}", nil_mean, nil_min);
    println!("Simhash (64-bit):   mean={:.1}, min={}", sim_mean, sim_min);

    println!("\n{}", "=".repeat(80));
    println!("VERDICT");
    println!("{}", "=".repeat(80));
    println!("\nCompare 'AtLeast1Match' percentages above.");
    println!("If simhash is significantly better, it might be worth using for LSH lookup.");
    println!("If similar or worse, the problem is fundamental to locality-sensitive hashing.");
}
