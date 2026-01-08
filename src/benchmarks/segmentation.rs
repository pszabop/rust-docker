//! Segmentation analysis for key-value lookup feasibility

use crate::hash::stable_document_hash_256;
use crate::modification::{randomly_modify_letters, randomly_modify_words, generate_random_document};

/// Extract 24-bit segments from a 256-bit hash
/// For 4 segments: bits 0-23, 64-87, 128-151, 192-215
/// For 8 segments: bits 0-23, 32-55, 64-87, 96-119, 128-151, 160-183, 192-215, 224-247
fn extract_segments(hash: &[u8; 32], num_segments: usize) -> Vec<u32> {
    let mut segments = Vec::with_capacity(num_segments);

    // Bit positions for segment starts (24 bits each)
    let starts: Vec<usize> = if num_segments == 4 {
        vec![0, 64, 128, 192]  // Spread across the hash
    } else {
        vec![0, 32, 64, 96, 128, 160, 192, 224]  // Every 32 bits
    };

    for &bit_start in starts.iter().take(num_segments) {
        let byte_start = bit_start / 8;
        // Extract 3 bytes (24 bits) starting at byte_start
        let seg: u32 = ((hash[byte_start] as u32) << 16)
            | ((hash[byte_start + 1] as u32) << 8)
            | (hash[byte_start + 2] as u32);
        segments.push(seg);
    }

    segments
}

pub fn run() {
    println!("\n============================================================");
    println!("=== SEGMENTATION ANALYSIS FOR KEY-VALUE LOOKUP ===");
    println!("============================================================\n");
    println!("Goal: Test if at least one 24-bit segment survives document fuzzing.");
    println!("If yes, we can use that segment as a Redis key to find the full hash.\n");

    let mut rng = rand::thread_rng();

    const NUM_DOCUMENTS: usize = 1000;
    const DOC_LENGTH: usize = 1000;  // Similar to fingerprint length

    // Test different modification amounts
    let mod_amounts = vec![4, 8, 16, 32, 64, 128];

    println!("Generating {} random documents of ~{} chars each...\n", NUM_DOCUMENTS, DOC_LENGTH);

    // Generate all random documents upfront
    let documents: Vec<String> = (0..NUM_DOCUMENTS)
        .map(|_| generate_random_document(&mut rng, DOC_LENGTH))
        .collect();

    // Hash all original documents
    let original_hashes: Vec<[u8; 32]> = documents.iter()
        .map(|doc| stable_document_hash_256(doc))
        .collect();

    for &num_segments in &[4, 8] {
        println!("{}", "=".repeat(70));
        println!("{} SEGMENTS (24 bits each = {} bits used of 256)",
                 num_segments, num_segments * 24);
        println!("{}", "=".repeat(70));

        // Extract original segments
        let original_segments: Vec<Vec<u32>> = original_hashes.iter()
            .map(|h| extract_segments(h, num_segments))
            .collect();

        println!("\n{:>8} | {:>12} | {:>12} | {:>15} | {:>15}",
                 "ModChars", "AtLeast1Match", "AvgMatches", "AllMatch", "NoneMatch");
        println!("{}", "-".repeat(70));

        for &mod_chars in &mod_amounts {
            let mut at_least_one_match = 0;
            let mut total_matches = 0;
            let mut all_match = 0;
            let mut none_match = 0;

            for (doc_idx, doc) in documents.iter().enumerate() {
                // Randomly modify the document
                let modified = randomly_modify_letters(doc, mod_chars);
                let modified_hash = stable_document_hash_256(&modified);
                let modified_segments = extract_segments(&modified_hash, num_segments);

                // Count matching segments
                let matches: usize = original_segments[doc_idx].iter()
                    .zip(modified_segments.iter())
                    .filter(|(orig, modif)| orig == modif)
                    .count();

                total_matches += matches;
                if matches >= 1 {
                    at_least_one_match += 1;
                }
                if matches == num_segments {
                    all_match += 1;
                }
                if matches == 0 {
                    none_match += 1;
                }
            }

            let pct_at_least_one = at_least_one_match as f64 / NUM_DOCUMENTS as f64 * 100.0;
            let avg_matches = total_matches as f64 / NUM_DOCUMENTS as f64;
            let pct_all = all_match as f64 / NUM_DOCUMENTS as f64 * 100.0;
            let pct_none = none_match as f64 / NUM_DOCUMENTS as f64 * 100.0;

            println!("{:>8} | {:>11.1}% | {:>12.2} | {:>14.1}% | {:>14.1}%",
                     mod_chars, pct_at_least_one, avg_matches, pct_all, pct_none);
        }
        println!();
    }

    // Also test with word-level modifications (more realistic)
    println!("\n{}", "=".repeat(70));
    println!("WORD-LEVEL MODIFICATIONS (more realistic for fingerprint changes)");
    println!("{}", "=".repeat(70));

    for &num_segments in &[4, 8] {
        println!("\n--- {} Segments ---", num_segments);

        let original_segments: Vec<Vec<u32>> = original_hashes.iter()
            .map(|h| extract_segments(h, num_segments))
            .collect();

        println!("\n{:>8} | {:>12} | {:>12} | {:>15} | {:>15}",
                 "ModChars", "AtLeast1Match", "AvgMatches", "AllMatch", "NoneMatch");
        println!("{}", "-".repeat(70));

        for &mod_chars in &mod_amounts {
            let mut at_least_one_match = 0;
            let mut total_matches = 0;
            let mut all_match = 0;
            let mut none_match = 0;

            for (doc_idx, doc) in documents.iter().enumerate() {
                let modified = randomly_modify_words(doc, mod_chars);
                let modified_hash = stable_document_hash_256(&modified);
                let modified_segments = extract_segments(&modified_hash, num_segments);

                let matches: usize = original_segments[doc_idx].iter()
                    .zip(modified_segments.iter())
                    .filter(|(orig, modif)| orig == modif)
                    .count();

                total_matches += matches;
                if matches >= 1 {
                    at_least_one_match += 1;
                }
                if matches == num_segments {
                    all_match += 1;
                }
                if matches == 0 {
                    none_match += 1;
                }
            }

            let pct_at_least_one = at_least_one_match as f64 / NUM_DOCUMENTS as f64 * 100.0;
            let avg_matches = total_matches as f64 / NUM_DOCUMENTS as f64;
            let pct_all = all_match as f64 / NUM_DOCUMENTS as f64 * 100.0;
            let pct_none = none_match as f64 / NUM_DOCUMENTS as f64 * 100.0;

            println!("{:>8} | {:>11.1}% | {:>12.2} | {:>14.1}% | {:>14.1}%",
                     mod_chars, pct_at_least_one, avg_matches, pct_all, pct_none);
        }
    }

    println!("\n============================================================");
    println!("INTERPRETATION:");
    println!("============================================================");
    println!("- 'AtLeast1Match': % of docs where at least one segment survived (can do KV lookup)");
    println!("- 'AvgMatches': Average number of segments that match (out of N)");
    println!("- 'AllMatch': % where all segments match (no change detected)");
    println!("- 'NoneMatch': % where NO segments match (lookup would FAIL)");
    println!("\nFor KV lookup to work reliably, 'AtLeast1Match' should be ~100%");
}
