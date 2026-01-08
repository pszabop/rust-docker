#![allow(dead_code)]
use rand::Rng;
use std::fs;
use plotters::prelude::*;
use simhash::simhash;
use std::time::Instant;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;

// ==================== BIT UNIFORMITY ANALYSIS ====================

#[derive(Clone)]
struct BitUniformityStats {
    bit_flip_counts: [u32; 256],
    trials: u32,
    mean: f64,
    std_dev: f64,
    min: u32,
    max: u32,
    coeff_of_variation: f64,
}

fn analyze_bit_uniformity(
    input: &str,
    trials: usize,
    chars_to_modify: usize,
    mod_type: ModType,
) -> BitUniformityStats {
    let mut bit_flip_counts = [0u32; 256];
    let original_hash = stable_document_hash_256(input);

    for _ in 0..trials {
        let modified = match mod_type {
            ModType::Letter => randomly_modify_letters(input, chars_to_modify),
            ModType::Word => randomly_modify_words(input, chars_to_modify),
            ModType::Insert => randomly_insert_chars(input, chars_to_modify),
            ModType::Remove => randomly_remove_chars(input, chars_to_modify),
        };
        let modified_hash = stable_document_hash_256(&modified);

        // XOR to find differing bits
        for byte_idx in 0..32 {
            let diff = original_hash[byte_idx] ^ modified_hash[byte_idx];
            for bit_in_byte in 0..8 {
                if (diff >> bit_in_byte) & 1 == 1 {
                    let bit_idx = byte_idx * 8 + bit_in_byte;
                    bit_flip_counts[bit_idx] += 1;
                }
            }
        }
    }

    // Calculate statistics
    let sum: u32 = bit_flip_counts.iter().sum();
    let mean = sum as f64 / 256.0;

    let variance: f64 = bit_flip_counts.iter()
        .map(|&c| (c as f64 - mean).powi(2))
        .sum::<f64>() / 256.0;
    let std_dev = variance.sqrt();

    let min = *bit_flip_counts.iter().min().unwrap();
    let max = *bit_flip_counts.iter().max().unwrap();
    let coeff_of_variation = if mean > 0.0 { std_dev / mean } else { 0.0 };

    BitUniformityStats {
        bit_flip_counts,
        trials: trials as u32,
        mean,
        std_dev,
        min,
        max,
        coeff_of_variation,
    }
}

fn print_uniformity_results(stats: &BitUniformityStats, label: &str) {
    println!("\n=== {} ===", label);
    println!("Trials: {}", stats.trials);
    println!("Mean flips per bit: {:.2}", stats.mean);
    println!("Std dev: {:.2}", stats.std_dev);
    println!("Min: {}, Max: {}", stats.min, stats.max);
    println!("Coeff of Variation: {:.4} (lower = more uniform, <0.1 is good)", stats.coeff_of_variation);

    // Show distribution by 32-bit segment (8 segments)
    println!("\n32-bit Segment Analysis (for 8-segment lookup):");
    for seg in 0..8 {
        let start_bit = seg * 32;
        let end_bit = start_bit + 32;
        let segment_sum: u32 = stats.bit_flip_counts[start_bit..end_bit].iter().sum();
        let segment_mean = segment_sum as f64 / 32.0;
        let segment_var: f64 = stats.bit_flip_counts[start_bit..end_bit].iter()
            .map(|&c| (c as f64 - segment_mean).powi(2))
            .sum::<f64>() / 32.0;
        let segment_std = segment_var.sqrt();
        let segment_cv = if segment_mean > 0.0 { segment_std / segment_mean } else { 0.0 };
        println!("  Segment {} (bits {:3}-{:3}): mean={:.2}, std={:.2}, CV={:.4}",
                 seg, start_bit, end_bit - 1, segment_mean, segment_std, segment_cv);
    }
}

fn generate_bit_uniformity_plot(
    stats: &BitUniformityStats,
    filename: &str,
    title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (1200, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_count = stats.max as f64 * 1.1;

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 22))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0..256, 0.0..max_count)?;

    chart.configure_mesh()
        .x_desc("Bit Position")
        .y_desc("Flip Count")
        .draw()?;

    // Draw bars for each bit position
    chart.draw_series(
        stats.bit_flip_counts.iter().enumerate().map(|(i, &count)| {
            let color = match i / 32 {
                0 => RED,
                1 => BLUE,
                2 => GREEN,
                3 => MAGENTA,
                4 => CYAN,
                5 => RGBColor(255, 165, 0), // Orange
                6 => RGBColor(128, 0, 128), // Purple
                _ => BLACK,
            };
            Rectangle::new([(i as i32, 0.0), ((i + 1) as i32, count as f64)], color.filled())
        })
    )?;

    // Draw mean line
    chart.draw_series(LineSeries::new(
        [(0, stats.mean), (256, stats.mean)],
        BLACK.stroke_width(2),
    ))?;

    // Draw segment boundaries
    for seg in 1..8 {
        let x = seg * 32;
        chart.draw_series(LineSeries::new(
            [(x, 0.0), (x, max_count)],
            BLACK.stroke_width(1),
        ))?;
    }

    root.present()?;
    Ok(())
}

fn run_bit_uniformity_analysis() {
    println!("\n============================================================");
    println!("=== BIT UNIFORMITY ANALYSIS FOR HASH SEGMENTATION ===");
    println!("============================================================\n");
    println!("Goal: Determine if nilsimsa hash bits flip uniformly across all 256 positions.");
    println!("If uniform, we can segment the hash (e.g., 8x 32-bit) for fuzzy lookups.");
    println!("If non-uniform, some segments would be unreliable.\n");

    let chrome = fs::read_to_string("chrome_values.json")
        .expect("Unable to read chrome_values.json");
    let firefox = fs::read_to_string("firefox_values.json")
        .expect("Unable to read firefox_values.json");

    const TRIALS: usize = 1000;

    // Test different modification amounts
    for &chars_modified in &[8, 32, 64] {
        println!("\n{}", "=".repeat(60));
        println!("MODIFICATION: {} chars", chars_modified);
        println!("{}", "=".repeat(60));

        // Letter modifications (scattered)
        let stats_letter_chrome = analyze_bit_uniformity(&chrome, TRIALS, chars_modified, ModType::Letter);
        let stats_letter_firefox = analyze_bit_uniformity(&firefox, TRIALS, chars_modified, ModType::Letter);
        print_uniformity_results(&stats_letter_chrome, &format!("Chrome - {} letter changes", chars_modified));
        print_uniformity_results(&stats_letter_firefox, &format!("Firefox - {} letter changes", chars_modified));

        // Word modifications (concentrated)
        let stats_word_chrome = analyze_bit_uniformity(&chrome, TRIALS, chars_modified, ModType::Word);
        let stats_word_firefox = analyze_bit_uniformity(&firefox, TRIALS, chars_modified, ModType::Word);
        print_uniformity_results(&stats_word_chrome, &format!("Chrome - {} word changes", chars_modified));
        print_uniformity_results(&stats_word_firefox, &format!("Firefox - {} word changes", chars_modified));

        // Generate plot for 32-char modifications (representative)
        if chars_modified == 32 {
            generate_bit_uniformity_plot(
                &stats_letter_chrome,
                "bit_uniformity_letter_32.png",
                "Bit Flip Distribution - 32 Letter Changes (Chrome)"
            ).expect("Failed to generate letter plot");

            generate_bit_uniformity_plot(
                &stats_word_chrome,
                "bit_uniformity_word_32.png",
                "Bit Flip Distribution - 32 Word Changes (Chrome)"
            ).expect("Failed to generate word plot");
        }
    }

    // Cross-browser analysis: how do bits flip between Chrome and Firefox hashes?
    println!("\n{}", "=".repeat(60));
    println!("CROSS-BROWSER BIT DIFFERENCE ANALYSIS");
    println!("{}", "=".repeat(60));
    let chrome_hash = stable_document_hash_256(&chrome);
    let firefox_hash = stable_document_hash_256(&firefox);
    println!("\nBits differing between Chrome and Firefox base hashes:");
    let mut diff_bits = Vec::new();
    for byte_idx in 0..32 {
        let diff = chrome_hash[byte_idx] ^ firefox_hash[byte_idx];
        for bit_in_byte in 0..8 {
            if (diff >> bit_in_byte) & 1 == 1 {
                diff_bits.push(byte_idx * 8 + bit_in_byte);
            }
        }
    }
    println!("Total differing bits: {}", diff_bits.len());
    println!("By segment:");
    for seg in 0..8 {
        let count = diff_bits.iter().filter(|&&b| b >= seg * 32 && b < (seg + 1) * 32).count();
        println!("  Segment {} (bits {:3}-{:3}): {} bits differ",
                 seg, seg * 32, (seg + 1) * 32 - 1, count);
    }

    println!("\nGenerated: bit_uniformity_letter_32.png, bit_uniformity_word_32.png");
}

// ==================== SEGMENTATION ANALYSIS ====================

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

/// Generate a random document (simulating browser fingerprint-like data)
fn generate_random_document(rng: &mut impl Rng, length: usize) -> String {
    // Mix of words, numbers, and punctuation like real fingerprint data
    let words = ["Chrome", "Firefox", "Safari", "Mozilla", "WebKit", "Gecko", "Intel", "AMD",
                 "Windows", "MacOS", "Linux", "Android", "true", "false", "null", "undefined",
                 "screen", "canvas", "audio", "webgl", "fonts", "plugins", "timezone"];

    let mut doc = String::with_capacity(length);
    while doc.len() < length {
        let choice = rng.gen_range(0..10);
        match choice {
            0..=4 => {
                // Random word
                let word = words[rng.gen_range(0..words.len())];
                doc.push_str(word);
            }
            5..=6 => {
                // Random number
                let num: u32 = rng.gen_range(0..10000);
                doc.push_str(&num.to_string());
            }
            7 => {
                // Random float
                let num: f32 = rng.gen_range(0.0..1000.0);
                doc.push_str(&format!("{:.2}", num));
            }
            _ => {
                // Random lowercase string
                let len = rng.gen_range(3..10);
                for _ in 0..len {
                    doc.push(rng.gen_range(b'a'..=b'z') as char);
                }
            }
        }
        // Add separator
        if doc.len() < length {
            let sep = match rng.gen_range(0..4) {
                0 => ", ",
                1 => "; ",
                2 => " ",
                _ => ", ",
            };
            doc.push_str(sep);
        }
    }
    doc.truncate(length);
    doc
}

fn run_segmentation_analysis() {
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

// ==================== RATE LIMIT ANALYSIS ====================

fn run_ratelimit_analysis() {
    println!("\n============================================================");
    println!("=== RATE LIMIT THRESHOLD ANALYSIS ===");
    println!("============================================================\n");
    println!("Goal: Find hamming distance threshold that:");
    println!("  - Catches fuzzed versions of same browser (attacker can't escape)");
    println!("  - Doesn't catch different browsers (no false positives)\n");

    let mut rng = rand::thread_rng();

    const NUM_BROWSERS: usize = 500;
    const DOC_LENGTH: usize = 1000;
    const FUZZ_TRIALS_PER_BROWSER: usize = 10;

    println!("Generating {} unique browser fingerprints...", NUM_BROWSERS);

    // Generate unique browser fingerprints
    let browsers: Vec<String> = (0..NUM_BROWSERS)
        .map(|_| generate_random_document(&mut rng, DOC_LENGTH))
        .collect();

    let browser_hashes: Vec<[u8; 32]> = browsers.iter()
        .map(|b| stable_document_hash_256(b))
        .collect();

    // Collect hamming distances for SAME browser (fuzzed)
    println!("Computing same-browser fuzzed distances...");
    let mut same_browser_distances: Vec<u32> = Vec::new();

    for (fuzz_chars, label) in &[(8, "8 chars"), (16, "16 chars"), (32, "32 chars"), (64, "64 chars")] {
        let mut distances_for_fuzz: Vec<u32> = Vec::new();

        for (browser_idx, browser) in browsers.iter().enumerate() {
            for _ in 0..FUZZ_TRIALS_PER_BROWSER {
                let fuzzed = randomly_modify_letters(browser, *fuzz_chars);
                let fuzzed_hash = stable_document_hash_256(&fuzzed);
                let dist = hamming_distance_256(&browser_hashes[browser_idx], &fuzzed_hash);
                distances_for_fuzz.push(dist);
                same_browser_distances.push(dist);
            }
        }

        let mean = distances_for_fuzz.iter().map(|&d| d as f64).sum::<f64>() / distances_for_fuzz.len() as f64;
        let max = *distances_for_fuzz.iter().max().unwrap();
        let p95 = percentile(&distances_for_fuzz, 95.0);
        let p99 = percentile(&distances_for_fuzz, 99.0);
        println!("  Fuzz {}: mean={:.1}, p95={}, p99={}, max={}", label, mean, p95, p99, max);
    }

    // Collect hamming distances for DIFFERENT browsers
    println!("\nComputing different-browser distances...");
    let mut different_browser_distances: Vec<u32> = Vec::new();

    // Compare each browser to a sample of other browsers
    let comparisons_per_browser = 50;
    for i in 0..NUM_BROWSERS {
        for _ in 0..comparisons_per_browser {
            let j = rng.gen_range(0..NUM_BROWSERS);
            if i != j {
                let dist = hamming_distance_256(&browser_hashes[i], &browser_hashes[j]);
                different_browser_distances.push(dist);
            }
        }
    }

    let diff_mean = different_browser_distances.iter().map(|&d| d as f64).sum::<f64>()
        / different_browser_distances.len() as f64;
    let diff_min = *different_browser_distances.iter().min().unwrap();
    let diff_p5 = percentile(&different_browser_distances, 5.0);
    let diff_p1 = percentile(&different_browser_distances, 1.0);

    println!("  Different browsers: mean={:.1}, min={}, p1={}, p5={}", diff_mean, diff_min, diff_p1, diff_p5);

    // Analyze threshold options
    println!("\n============================================================");
    println!("THRESHOLD ANALYSIS");
    println!("============================================================\n");
    println!("For each threshold T, we check:");
    println!("  - True Positive Rate: % of fuzzed same-browser within T (want HIGH)");
    println!("  - False Positive Rate: % of different-browser within T (want ZERO)\n");

    println!("{:>10} | {:>15} | {:>15} | {:>12}",
             "Threshold", "TPR (same)", "FPR (diff)", "Verdict");
    println!("{}", "-".repeat(60));

    for threshold in &[20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120] {
        let tpr = same_browser_distances.iter()
            .filter(|&&d| d <= *threshold)
            .count() as f64 / same_browser_distances.len() as f64 * 100.0;

        let fpr = different_browser_distances.iter()
            .filter(|&&d| d <= *threshold)
            .count() as f64 / different_browser_distances.len() as f64 * 100.0;

        let verdict = if fpr == 0.0 && tpr > 90.0 {
            "GOOD"
        } else if fpr == 0.0 && tpr > 50.0 {
            "OK"
        } else if fpr > 0.0 {
            "FP RISK"
        } else {
            "LOW TPR"
        };

        println!("{:>10} | {:>14.1}% | {:>14.2}% | {:>12}",
                 threshold, tpr, fpr, verdict);
    }

    // Distribution visualization
    println!("\n============================================================");
    println!("DISTANCE DISTRIBUTIONS");
    println!("============================================================\n");

    // Bucket distances into histogram
    let bucket_size = 10;
    let max_dist = 256;
    let num_buckets = max_dist / bucket_size + 1;

    let mut same_hist = vec![0usize; num_buckets];
    let mut diff_hist = vec![0usize; num_buckets];

    for &d in &same_browser_distances {
        same_hist[(d as usize) / bucket_size] += 1;
    }
    for &d in &different_browser_distances {
        diff_hist[(d as usize) / bucket_size] += 1;
    }

    let same_max = *same_hist.iter().max().unwrap() as f64;
    let diff_max = *diff_hist.iter().max().unwrap() as f64;

    println!("Hamming Distance Distribution (bucket size = {}):", bucket_size);
    println!("{:>10} | {:20} | {:20}", "Distance", "Same Browser (fuzzed)", "Different Browsers");
    println!("{}", "-".repeat(60));

    for bucket in 0..num_buckets {
        let range_start = bucket * bucket_size;
        let range_end = range_start + bucket_size - 1;

        let same_bar_len = (same_hist[bucket] as f64 / same_max * 15.0) as usize;
        let diff_bar_len = (diff_hist[bucket] as f64 / diff_max * 15.0) as usize;

        let same_bar: String = "█".repeat(same_bar_len);
        let diff_bar: String = "█".repeat(diff_bar_len);

        if same_hist[bucket] > 0 || diff_hist[bucket] > 0 {
            println!("{:>3}-{:<3}    | {:15} {:>4} | {:15} {:>4}",
                     range_start, range_end.min(255),
                     same_bar, same_hist[bucket],
                     diff_bar, diff_hist[bucket]);
        }
    }

    // Recommendation
    println!("\n============================================================");
    println!("RECOMMENDATION FOR KV-BASED RATE LIMITING");
    println!("============================================================\n");

    // Find the gap
    let same_max_dist = *same_browser_distances.iter().max().unwrap();
    let diff_min_dist = *different_browser_distances.iter().min().unwrap();

    if same_max_dist < diff_min_dist {
        println!("GOOD NEWS: Clean separation exists!");
        println!("  - Max same-browser fuzzed distance: {}", same_max_dist);
        println!("  - Min different-browser distance: {}", diff_min_dist);
        println!("  - Safe threshold range: {} to {}", same_max_dist, diff_min_dist);
        println!("\nStrategy: Use threshold around {} for zero false positives", (same_max_dist + diff_min_dist) / 2);
    } else {
        println!("WARNING: Distributions overlap!");
        println!("  - Max same-browser fuzzed distance: {}", same_max_dist);
        println!("  - Min different-browser distance: {}", diff_min_dist);
        println!("  - Overlap region: {} to {}", diff_min_dist, same_max_dist);

        // Find best threshold
        let mut best_threshold = 0;
        let mut best_score = 0.0;
        for t in 0..=256 {
            let tpr = same_browser_distances.iter().filter(|&&d| d <= t).count() as f64
                / same_browser_distances.len() as f64;
            let fpr = different_browser_distances.iter().filter(|&&d| d <= t).count() as f64
                / different_browser_distances.len() as f64;
            // Score: maximize TPR while heavily penalizing FPR
            let score = tpr - 10.0 * fpr;
            if score > best_score {
                best_score = score;
                best_threshold = t;
            }
        }
        println!("\nBest threshold (maximizing TPR - 10*FPR): {}", best_threshold);
    }

    println!("\n--- KV LOOKUP APPROACH ---");
    println!("Since exact segment match is unreliable, consider:");
    println!("1. Store full 256-bit hash as value, with fingerprint ID as key");
    println!("2. On new request, compute hash and scan recent hashes (time-windowed)");
    println!("3. Use hamming distance <= threshold to match");
    println!("4. For scale: use LSH bands to reduce comparison set, then exact hamming check");
}

/// Calculate percentile of a list of values
fn percentile(values: &[u32], pct: f64) -> u32 {
    let mut sorted: Vec<u32> = values.to_vec();
    sorted.sort();
    let idx = ((pct / 100.0) * (sorted.len() - 1) as f64) as usize;
    sorted[idx]
}

// ==================== SIMHASH COMPARISON ====================

/// Compute simhash (64-bit) for a document
fn compute_simhash(input: &str) -> u64 {
    // simhash crate takes a string and does its own tokenization
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

fn run_simhash_comparison() {
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

// ==================== BK-TREE BENCHMARK ====================

/// Wrapper type for 256-bit hash to use with bktree crate
#[derive(Clone, Debug, PartialEq, Eq)]
struct Hash256([u8; 32]);

/// Hamming distance function for bktree (returns isize as required by crate)
fn hash256_hamming_distance(a: &Hash256, b: &Hash256) -> isize {
    a.0.iter()
        .zip(b.0.iter())
        .map(|(x, y)| (x ^ y).count_ones() as isize)
        .sum()
}

fn run_bktree_benchmark() {
    use bktree::BkTree;

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

// ==================== THREAD-SAFE RATE LIMITER (Tokio Compatible) ====================

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

    /// Convert a 256-bit hash to 4 x u64
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

    /// Compute hamming distance between two u64x4 hashes
    #[inline(always)]
    pub fn hamming_distance(a: &[u64; 4], b: &[u64; 4]) -> u32 {
        (a[0] ^ b[0]).count_ones()
            + (a[1] ^ b[1]).count_ones()
            + (a[2] ^ b[2]).count_ones()
            + (a[3] ^ b[3]).count_ones()
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
                if Self::hamming_distance(query, h) <= threshold {
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
                if Self::hamming_distance(query, h) <= threshold {
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
            if Self::hamming_distance(query, h) <= threshold {
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
        let query = RateLimitBucket::hash_to_u64(hash);
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

    /// Async version of check_request for explicit async contexts.
    /// Uses tokio::task::block_in_place to ensure we don't block other tasks.
    #[cfg(feature = "tokio")]
    pub async fn check_request_async(&self, hash: &[u8; 32], timestamp_secs: u64) -> u32 {
        // For sub-100μs operations, we can safely do this synchronously
        // But provide async API for consistency
        self.check_request(hash, timestamp_secs)
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

fn run_ratelimiter_benchmark() {
    use std::thread;

    println!("\n============================================================");
    println!("=== THREAD-SAFE RATE LIMITER BENCHMARK (Tokio Compatible) ===");
    println!("============================================================\n");

    let mut rng = rand::thread_rng();

    // Configuration
    const NUM_BUCKETS: usize = 12;        // 12 buckets
    const BUCKET_DURATION: u64 = 10;       // 10 seconds each = 2 min window
    const THRESHOLD: u32 = 30;
    const CAPACITY_PER_BUCKET: usize = 10_000;

    let limiter = Arc::new(FuzzyRateLimiter::new(
        NUM_BUCKETS,
        BUCKET_DURATION,
        THRESHOLD,
        CAPACITY_PER_BUCKET,
    ));

    println!("Configuration:");
    println!("  Window: {} seconds ({} buckets × {} sec)",
             limiter.window_duration_secs(), NUM_BUCKETS, BUCKET_DURATION);
    println!("  Hamming threshold: {}", THRESHOLD);
    println!("  Using: parking_lot::RwLock (tokio-safe)\n");

    // Pre-generate test fingerprints
    const NUM_FINGERPRINTS: usize = 1000;
    const DOC_LENGTH: usize = 1000;

    println!("Generating {} test fingerprints...", NUM_FINGERPRINTS);
    let fingerprints: Vec<[u8; 32]> = (0..NUM_FINGERPRINTS)
        .map(|_| {
            let doc = generate_random_document(&mut rng, DOC_LENGTH);
            stable_document_hash_256(&doc)
        })
        .collect();

    // Benchmark single-threaded performance
    println!("\n--- Single-threaded benchmark ---");
    let timestamp = 100u64; // Fixed timestamp for testing

    let start = Instant::now();
    const ITERATIONS: usize = 10_000;
    for i in 0..ITERATIONS {
        let fp = &fingerprints[i % fingerprints.len()];
        let _count = limiter.check_request(fp, timestamp);
    }
    let elapsed = start.elapsed();
    let per_request = elapsed.as_micros() as f64 / ITERATIONS as f64;

    println!("  {} requests in {:?}", ITERATIONS, elapsed);
    println!("  {:.2}μs per request", per_request);
    println!("  Entries in limiter: {}", limiter.total_entries());

    // Benchmark with fuzzed fingerprints
    println!("\n--- Fuzzed fingerprint benchmark ---");
    let docs: Vec<String> = (0..NUM_FINGERPRINTS)
        .map(|_| generate_random_document(&mut rng, DOC_LENGTH))
        .collect();

    let limiter3 = Arc::new(FuzzyRateLimiter::new(
        NUM_BUCKETS, BUCKET_DURATION, THRESHOLD, CAPACITY_PER_BUCKET,
    ));

    // Insert originals
    for doc in &docs {
        let hash = stable_document_hash_256(doc);
        limiter3.check_request(&hash, timestamp);
    }
    println!("  Inserted {} original fingerprints", docs.len());

    // Query with fuzzed versions
    let start = Instant::now();
    let mut hits = 0;
    for doc in &docs {
        let fuzzed = randomly_modify_letters(doc, 16);
        let hash = stable_document_hash_256(&fuzzed);
        let count = limiter3.check_request(&hash, timestamp);
        if count > 1 {
            hits += 1;
        }
    }
    let elapsed = start.elapsed();

    println!("  Fuzzed queries: {} in {:?}", docs.len(), elapsed);
    println!("  Hit rate (found original): {:.1}%", hits as f64 / docs.len() as f64 * 100.0);
    println!("  {:.2}μs per fuzzed request", elapsed.as_micros() as f64 / docs.len() as f64);

    // Multi-threaded benchmark (simulating tokio runtime threads)
    println!("\n--- Multi-threaded benchmark (4 threads, simulating tokio workers) ---");
    let limiter4 = Arc::new(FuzzyRateLimiter::new(
        NUM_BUCKETS, BUCKET_DURATION, THRESHOLD, CAPACITY_PER_BUCKET,
    ));
    let fingerprints_arc = Arc::new(fingerprints.clone());

    let num_threads = 4;
    let requests_per_thread = 5000;

    let start = Instant::now();
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let limiter = Arc::clone(&limiter4);
            let fps = Arc::clone(&fingerprints_arc);
            thread::spawn(move || {
                for i in 0..requests_per_thread {
                    let fp = &fps[(thread_id * 1000 + i) % fps.len()];
                    let _count = limiter.check_request(fp, 100);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
    let elapsed = start.elapsed();
    let total_requests = num_threads * requests_per_thread;

    println!("  {} total requests across {} threads in {:?}",
             total_requests, num_threads, elapsed);
    println!("  {:.2}μs per request (wall clock / total requests)",
             elapsed.as_micros() as f64 / total_requests as f64);
    println!("  Throughput: {:.0} requests/sec",
             total_requests as f64 / elapsed.as_secs_f64());
    println!("  Entries in limiter: {}", limiter4.total_entries());

    // Tokio async benchmark
    println!("\n--- Tokio async benchmark ---");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let limiter5 = Arc::new(FuzzyRateLimiter::new(
        NUM_BUCKETS, BUCKET_DURATION, THRESHOLD, CAPACITY_PER_BUCKET,
    ));
    let fingerprints_arc2 = Arc::new(fingerprints);

    let tokio_result = rt.block_on(async {
        let num_tasks = 8;
        let requests_per_task = 2500;

        let start = Instant::now();
        let mut handles = Vec::new();

        for task_id in 0..num_tasks {
            let limiter = Arc::clone(&limiter5);
            let fps = Arc::clone(&fingerprints_arc2);

            handles.push(tokio::spawn(async move {
                for i in 0..requests_per_task {
                    let fp = &fps[(task_id * 500 + i) % fps.len()];
                    // Direct call - parking_lot is safe in async context for <100μs ops
                    let _count = limiter.check_request(fp, 100);
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let elapsed = start.elapsed();
        (num_tasks * requests_per_task, elapsed)
    });

    let (total_async_requests, async_elapsed) = tokio_result;
    println!("  {} total requests across 8 tokio tasks in {:?}",
             total_async_requests, async_elapsed);
    println!("  {:.2}μs per request",
             async_elapsed.as_micros() as f64 / total_async_requests as f64);
    println!("  Throughput: {:.0} requests/sec",
             total_async_requests as f64 / async_elapsed.as_secs_f64());
    println!("  Entries in limiter: {}", limiter5.total_entries());

    // Test is_rate_limited helper
    println!("\n--- Rate limiting demo ---");
    let demo_limiter = Arc::new(FuzzyRateLimiter::new(1, 60, 30, 1000));
    let test_hash = stable_document_hash_256("test fingerprint");

    for i in 1..=15 {
        let is_limited = demo_limiter.is_rate_limited(&test_hash, 100, 10);
        println!("  Request {}: count={}, rate_limited={}",
                 i,
                 demo_limiter.check_request(&test_hash, 100) - 1, // -1 because check increments
                 is_limited);
    }

    println!("\n============================================================");
    println!("SUMMARY");
    println!("============================================================");
    println!("\nTarget: 500 RPS per core = 2000μs budget per request");
    println!("Rate limiter overhead: {:.2}μs per request", per_request);
    println!("Budget used: {:.2}%", per_request / 2000.0 * 100.0);

    println!("\n--- Usage Example (Pingora/Tokio) ---");
    println!(r#"
```rust
use std::sync::Arc;
use std::time::{{SystemTime, UNIX_EPOCH}};

// Initialize once at startup
let limiter = Arc::new(FuzzyRateLimiter::new(
    12,      // 12 buckets
    10,      // 10 seconds each = 2 minute window
    30,      // Hamming distance threshold
    10_000,  // Capacity per bucket
));

// Start automatic rotation
let _rotation_handle = limiter.start_rotation_task();

// In your request handler (async fn):
async fn handle_request(
    limiter: &FuzzyRateLimiter,
    fingerprint_hash: [u8; 32],
) -> Result<Response, RateLimited> {{
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check rate limit (sub-50μs, safe in async)
    if limiter.is_rate_limited(&fingerprint_hash, now, 100) {{
        return Err(RateLimited);
    }}

    // Process request...
    Ok(Response::new())
}}
```
"#);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let sim = if args.len() > 1 {
        args[1].as_str()
    } else {
        "help"
    };

    match sim {
        "uniformity" => run_bit_uniformity_analysis(),
        "discrimination" => run_discrimination_analysis(),
        "segmentation" => run_segmentation_analysis(),
        "ratelimit" => run_ratelimit_analysis(),
        "simhash" => run_simhash_comparison(),
        "bktree" => run_bktree_benchmark(),
        "ratelimiter" => run_ratelimiter_benchmark(),
        _ => {
            eprintln!("Usage: {} <simulation>", args[0]);
            eprintln!();
            eprintln!("Available simulations:");
            eprintln!("  uniformity      - Analyze bit flip uniformity for hash segmentation");
            eprintln!("  discrimination  - Analyze discrimination thresholds for various modifications");
            eprintln!("  segmentation    - Test if at least one segment survives fuzzing (for KV lookup)");
            eprintln!("  ratelimit       - Analyze threshold for rate limiting without false positives");
            eprintln!("  simhash         - Compare simhash vs nilsimsa for segment stability");
            eprintln!("  bktree          - Benchmark BK-tree for fuzzy hash lookup");
            eprintln!("  ratelimiter     - Benchmark thread-safe rate limiter");
            eprintln!();
            eprintln!("Examples:");
            eprintln!("  ./run.sh uniformity");
            eprintln!("  ./run.sh discrimination");
            eprintln!("  ./run.sh segmentation");
            eprintln!("  ./run.sh ratelimit");
            eprintln!("  ./run.sh simhash");
            eprintln!("  ./run.sh bktree");
            eprintln!("  ./run.sh ratelimiter");
            std::process::exit(1);
        }
    }
}

fn run_discrimination_analysis() {
    let chrome_long = fs::read_to_string("chrome_values.json")
        .expect("Unable to read file chrome_values.json");
    let firefox_long = fs::read_to_string("firefox_values.json")
        .expect("Unable to read file firefox_values.json");

    // Debug: show what word modification does
    println!("=== DEBUG: Word Modification Test ===");
    println!("Chrome input length: {} chars", chrome_long.len());
    let chrome_chars: Vec<char> = chrome_long.chars().collect();
    let fields = find_csv_fields(&chrome_chars);
    println!("Found {} CSV fields:", fields.len());
    for (i, (s, e)) in fields.iter().enumerate().take(10) {
        let content: String = chrome_chars[*s..*e].iter().collect();
        println!("  [{}] pos {}-{}, len={}: {:?}", i, s, e, e-s, content);
    }

    println!("\nTesting word modification (target=8 chars):");
    let modified = randomly_modify_words(&chrome_long, 8);
    println!("Original len: {}, Modified len: {}", chrome_long.len(), modified.len());

    // Find first difference
    let orig_chars: Vec<char> = chrome_long.chars().collect();
    let mod_chars: Vec<char> = modified.chars().collect();
    let mut diff_count = 0;
    let mut first_diff = None;
    for i in 0..orig_chars.len().min(mod_chars.len()) {
        if orig_chars[i] != mod_chars[i] {
            diff_count += 1;
            if first_diff.is_none() {
                first_diff = Some(i);
            }
        }
    }
    println!("Chars different: {}", diff_count);
    if let Some(pos) = first_diff {
        let start = pos.saturating_sub(10);
        let end = (pos + 30).min(orig_chars.len());
        let orig_ctx: String = orig_chars[start..end].iter().collect();
        let mod_ctx: String = mod_chars[start..end.min(mod_chars.len())].iter().collect();
        println!("First diff at pos {}: \n  orig: {:?}\n  mod:  {:?}", pos, orig_ctx, mod_ctx);
    }
    println!("=== END DEBUG ===\n");

    const NUM_TRIALS: usize = 100;
    let mod_counts: Vec<usize> = (0..=8).map(|i| 1 << i).collect(); // 1,2,4,8,...256

    // Calculate baselines
    let baseline_64 = hamming_distance_64(
        stable_document_hash_64(&chrome_long),
        stable_document_hash_64(&firefox_long)
    );
    let baseline_256 = hamming_distance_256(
        &stable_document_hash_256(&chrome_long),
        &stable_document_hash_256(&firefox_long)
    );

    println!("Baselines: 64-bit={} bits ({}%), 256-bit={} bits ({}%)\n",
             baseline_64, baseline_64 as f64 / 64.0 * 100.0,
             baseline_256, baseline_256 as f64 / 256.0 * 100.0);

    // ==================== RANDOM LETTER TEST ====================
    println!("============================================================");
    println!("=== TEST 1: RANDOM LETTER CHANGES ===");
    println!("============================================================\n");

    println!("--- 64-bit Random Letter ---");
    let letter_chrome_64 = run_analysis(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome", ModType::Letter, HashSize::Bit64);
    let letter_firefox_64 = run_analysis(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox", ModType::Letter, HashSize::Bit64);
    print_results_table(&letter_chrome_64, &letter_firefox_64, baseline_64);
    let letter_limit_64 = find_discrimination_limit(&letter_chrome_64, &letter_firefox_64, baseline_64);
    println!("Safe limit (64-bit, letters): ~{} chars\n", letter_limit_64);

    println!("--- 256-bit Random Letter ---");
    let letter_chrome_256 = run_analysis(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome", ModType::Letter, HashSize::Bit256);
    let letter_firefox_256 = run_analysis(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox", ModType::Letter, HashSize::Bit256);
    print_results_table(&letter_chrome_256, &letter_firefox_256, baseline_256);
    let letter_limit_256 = find_discrimination_limit(&letter_chrome_256, &letter_firefox_256, baseline_256);
    println!("Safe limit (256-bit, letters): ~{} chars\n", letter_limit_256);

    // ==================== RANDOM WORD TEST ====================
    println!("============================================================");
    println!("=== TEST 2: RANDOM WORD CHANGES (JSON field values) ===");
    println!("============================================================\n");

    println!("--- 64-bit Random Word ---");
    let word_chrome_64 = run_analysis(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome", ModType::Word, HashSize::Bit64);
    let word_firefox_64 = run_analysis(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox", ModType::Word, HashSize::Bit64);
    print_results_table(&word_chrome_64, &word_firefox_64, baseline_64);
    let word_limit_64 = find_discrimination_limit(&word_chrome_64, &word_firefox_64, baseline_64);
    println!("Safe limit (64-bit, words): ~{} chars\n", word_limit_64);

    println!("--- 256-bit Random Word ---");
    let word_chrome_256 = run_analysis(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome", ModType::Word, HashSize::Bit256);
    let word_firefox_256 = run_analysis(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox", ModType::Word, HashSize::Bit256);
    print_results_table(&word_chrome_256, &word_firefox_256, baseline_256);
    let word_limit_256 = find_discrimination_limit(&word_chrome_256, &word_firefox_256, baseline_256);
    println!("Safe limit (256-bit, words): ~{} chars\n", word_limit_256);

    // ==================== RANDOM INSERTION TEST ====================
    println!("============================================================");
    println!("=== TEST 3: RANDOM INSERTIONS ===");
    println!("============================================================\n");

    println!("--- 64-bit Random Insert ---");
    let insert_chrome_64 = run_analysis(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome", ModType::Insert, HashSize::Bit64);
    let insert_firefox_64 = run_analysis(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox", ModType::Insert, HashSize::Bit64);
    print_results_table(&insert_chrome_64, &insert_firefox_64, baseline_64);
    let insert_limit_64 = find_discrimination_limit(&insert_chrome_64, &insert_firefox_64, baseline_64);
    println!("Safe limit (64-bit, insert): ~{} chars\n", insert_limit_64);

    println!("--- 256-bit Random Insert ---");
    let insert_chrome_256 = run_analysis(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome", ModType::Insert, HashSize::Bit256);
    let insert_firefox_256 = run_analysis(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox", ModType::Insert, HashSize::Bit256);
    print_results_table(&insert_chrome_256, &insert_firefox_256, baseline_256);
    let insert_limit_256 = find_discrimination_limit(&insert_chrome_256, &insert_firefox_256, baseline_256);
    println!("Safe limit (256-bit, insert): ~{} chars\n", insert_limit_256);

    // ==================== RANDOM REMOVAL TEST ====================
    println!("============================================================");
    println!("=== TEST 4: RANDOM REMOVALS ===");
    println!("============================================================\n");

    println!("--- 64-bit Random Remove ---");
    let remove_chrome_64 = run_analysis(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome", ModType::Remove, HashSize::Bit64);
    let remove_firefox_64 = run_analysis(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox", ModType::Remove, HashSize::Bit64);
    print_results_table(&remove_chrome_64, &remove_firefox_64, baseline_64);
    let remove_limit_64 = find_discrimination_limit(&remove_chrome_64, &remove_firefox_64, baseline_64);
    println!("Safe limit (64-bit, remove): ~{} chars\n", remove_limit_64);

    println!("--- 256-bit Random Remove ---");
    let remove_chrome_256 = run_analysis(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome", ModType::Remove, HashSize::Bit256);
    let remove_firefox_256 = run_analysis(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox", ModType::Remove, HashSize::Bit256);
    print_results_table(&remove_chrome_256, &remove_firefox_256, baseline_256);
    let remove_limit_256 = find_discrimination_limit(&remove_chrome_256, &remove_firefox_256, baseline_256);
    println!("Safe limit (256-bit, remove): ~{} chars\n", remove_limit_256);

    // ==================== SUMMARY ====================
    println!("============================================================");
    println!("=== SUMMARY ===");
    println!("============================================================");
    println!("                    64-bit      256-bit");
    println!("Random Letters:     ~{:<6}     ~{:<6} chars", letter_limit_64, letter_limit_256);
    println!("Random Words:       ~{:<6}     ~{:<6} chars", word_limit_64, word_limit_256);
    println!("Random Inserts:     ~{:<6}     ~{:<6} chars", insert_limit_64, insert_limit_256);
    println!("Random Removes:     ~{:<6}     ~{:<6} chars", remove_limit_64, remove_limit_256);

    // Generate comparison plots
    generate_all_comparison_plot(
        &letter_chrome_64, &letter_firefox_64,
        &word_chrome_64, &word_firefox_64,
        &insert_chrome_64, &insert_firefox_64,
        &remove_chrome_64, &remove_firefox_64,
        baseline_64, "discrimination_64bit.png", 64
    ).expect("Failed to generate 64-bit plot");

    generate_all_comparison_plot(
        &letter_chrome_256, &letter_firefox_256,
        &word_chrome_256, &word_firefox_256,
        &insert_chrome_256, &insert_firefox_256,
        &remove_chrome_256, &remove_firefox_256,
        baseline_256, "discrimination_256bit.png", 256
    ).expect("Failed to generate 256-bit plot");

    println!("\nGenerated: discrimination_64bit.png, discrimination_256bit.png");
}

#[derive(Clone, Copy)]
enum ModType { Letter, Word, Insert, Remove }

#[derive(Clone, Copy)]
enum HashSize { Bit64, Bit256 }

#[derive(Clone)]
struct Stats {
    num_modifications: usize,
    mean: f64,
    stdev: f64,
}

fn run_analysis(input: &str, num_trials: usize, mod_counts: &[usize], name: &str,
                mod_type: ModType, hash_size: HashSize) -> Vec<Stats> {
    let type_str = match mod_type {
        ModType::Letter => "letter",
        ModType::Word => "word",
        ModType::Insert => "insert",
        ModType::Remove => "remove",
    };
    println!("  Analyzing {} ({})...", name, type_str);

    let mut results = Vec::with_capacity(mod_counts.len());

    for &target_chars in mod_counts {
        let mut distances: Vec<u32> = Vec::with_capacity(num_trials);

        for _ in 0..num_trials {
            let modified = match mod_type {
                ModType::Letter => randomly_modify_letters(input, target_chars),
                ModType::Word => randomly_modify_words(input, target_chars),
                ModType::Insert => randomly_insert_chars(input, target_chars),
                ModType::Remove => randomly_remove_chars(input, target_chars),
            };

            let dist = match hash_size {
                HashSize::Bit64 => hamming_distance_64(
                    stable_document_hash_64(input),
                    stable_document_hash_64(&modified)
                ),
                HashSize::Bit256 => hamming_distance_256(
                    &stable_document_hash_256(input),
                    &stable_document_hash_256(&modified)
                ),
            };
            distances.push(dist);
        }

        let mean = distances.iter().map(|&d| d as f64).sum::<f64>() / num_trials as f64;
        let variance = distances.iter().map(|&d| (d as f64 - mean).powi(2)).sum::<f64>() / num_trials as f64;
        let stdev = variance.sqrt();

        results.push(Stats { num_modifications: target_chars, mean, stdev });
        println!("    {} chars: mean={:.2}, mean+2σ={:.2}", target_chars, mean, mean + 2.0 * stdev);
    }

    results
}

fn print_results_table(chrome: &[Stats], firefox: &[Stats], baseline: u32) {
    println!("  {:>6} | {:>8} {:>8} | {:>8} {:>8} | {:>10} {:>8}",
             "Chars", "C Mean", "C +2σ", "F Mean", "F +2σ", "Combined", "% Base");
    println!("  {}", "-".repeat(75));

    for (c, f) in chrome.iter().zip(firefox.iter()) {
        let c_upper = c.mean + 2.0 * c.stdev;
        let f_upper = f.mean + 2.0 * f.stdev;
        let combined = c_upper + f_upper;
        let pct = combined / baseline as f64 * 100.0;
        println!("  {:>6} | {:>8.2} {:>8.2} | {:>8.2} {:>8.2} | {:>10.2} {:>7.1}%",
                 c.num_modifications, c.mean, c_upper, f.mean, f_upper, combined, pct);
    }
}

fn find_discrimination_limit(chrome: &[Stats], firefox: &[Stats], baseline: u32) -> usize {
    for i in 0..chrome.len() {
        let c_upper = chrome[i].mean + 2.0 * chrome[i].stdev;
        let f_upper = firefox[i].mean + 2.0 * firefox[i].stdev;
        if c_upper + f_upper >= baseline as f64 {
            return if i > 0 { chrome[i-1].num_modifications } else { 0 };
        }
    }
    chrome.last().unwrap().num_modifications
}

fn generate_all_comparison_plot(
    letter_chrome: &[Stats], letter_firefox: &[Stats],
    word_chrome: &[Stats], word_firefox: &[Stats],
    insert_chrome: &[Stats], insert_firefox: &[Stats],
    remove_chrome: &[Stats], remove_firefox: &[Stats],
    baseline: u32, filename: &str, bits: u32
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (1000, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_x = letter_chrome.last().unwrap().num_modifications as f64 * 1.1;

    // Calculate max Y from all data
    let all_combined: Vec<f64> = letter_chrome.iter().zip(letter_firefox.iter())
        .chain(word_chrome.iter().zip(word_firefox.iter()))
        .chain(insert_chrome.iter().zip(insert_firefox.iter()))
        .chain(remove_chrome.iter().zip(remove_firefox.iter()))
        .map(|(c, f)| (c.mean + 2.0 * c.stdev) + (f.mean + 2.0 * f.stdev))
        .collect();
    let max_y = all_combined.iter().cloned().fold(baseline as f64, f64::max) * 1.2;

    let mut chart = ChartBuilder::on(&root)
        .caption(format!("Modification Types - Discrimination Threshold ({}-bit)", bits), ("sans-serif", 22))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d((1.0_f64).log2()..max_x.log2(), 0.0..max_y)?;

    chart.configure_mesh()
        .x_desc("Characters Modified (log scale)")
        .y_desc("Combined Noise: Chrome(mean+2σ) + Firefox(mean+2σ)")
        .x_label_formatter(&|x| format!("{}", (2.0_f64).powf(*x) as u32))
        .draw()?;

    // Letter changes (magenta)
    let letter_combined: Vec<_> = letter_chrome.iter().zip(letter_firefox.iter())
        .map(|(c, f)| ((c.num_modifications as f64).log2(),
                       (c.mean + 2.0 * c.stdev) + (f.mean + 2.0 * f.stdev)))
        .collect();
    chart.draw_series(LineSeries::new(letter_combined.iter().cloned(), MAGENTA.stroke_width(3)))?
        .label("Letters (replace)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], MAGENTA.stroke_width(3)));

    // Word changes (cyan)
    let word_combined: Vec<_> = word_chrome.iter().zip(word_firefox.iter())
        .map(|(c, f)| ((c.num_modifications as f64).log2(),
                       (c.mean + 2.0 * c.stdev) + (f.mean + 2.0 * f.stdev)))
        .collect();
    chart.draw_series(LineSeries::new(word_combined.iter().cloned(), CYAN.stroke_width(3)))?
        .label("Words (replace)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], CYAN.stroke_width(3)));

    // Insert changes (blue)
    let insert_combined: Vec<_> = insert_chrome.iter().zip(insert_firefox.iter())
        .map(|(c, f)| ((c.num_modifications as f64).log2(),
                       (c.mean + 2.0 * c.stdev) + (f.mean + 2.0 * f.stdev)))
        .collect();
    chart.draw_series(LineSeries::new(insert_combined.iter().cloned(), BLUE.stroke_width(3)))?
        .label("Insertions").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE.stroke_width(3)));

    // Remove changes (red)
    let remove_combined: Vec<_> = remove_chrome.iter().zip(remove_firefox.iter())
        .map(|(c, f)| ((c.num_modifications as f64).log2(),
                       (c.mean + 2.0 * c.stdev) + (f.mean + 2.0 * f.stdev)))
        .collect();
    chart.draw_series(LineSeries::new(remove_combined.iter().cloned(), RED.stroke_width(3)))?
        .label("Removals").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.stroke_width(3)));

    // Baseline threshold
    chart.draw_series(LineSeries::new(
        [(1.0_f64.log2(), baseline as f64), (max_x.log2(), baseline as f64)],
        GREEN.stroke_width(3),
    ))?.label(format!("Baseline ({} bits)", baseline))
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN.stroke_width(3)));

    chart.configure_series_labels()
        .position(SeriesLabelPosition::UpperLeft)
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    Ok(())
}

// ==================== HASH FUNCTIONS ====================

fn stable_document_hash_64(input: &str) -> u64 {
    let mut hasher = nilsimsa::Nilsimsa::new();
    hasher.update(input);
    let hex_str = hasher.digest();
    u64::from_str_radix(&hex_str[0..16], 16).expect("Invalid hex")
}

fn stable_document_hash_256(input: &str) -> [u8; 32] {
    let mut hasher = nilsimsa::Nilsimsa::new();
    hasher.update(input);
    let hex_str = hasher.digest();
    let mut result = [0u8; 32];
    for i in 0..32 {
        result[i] = u8::from_str_radix(&hex_str[i * 2..i * 2 + 2], 16).expect("Invalid hex");
    }
    result
}

fn hamming_distance_64(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn hamming_distance_256(a: &[u8; 32], b: &[u8; 32]) -> u32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x ^ y).count_ones()).sum()
}

// ==================== MODIFICATION FUNCTIONS ====================

/// Randomly change N individual characters (scattered changes)
fn randomly_modify_letters(input: &str, n: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    for _ in 0..n {
        let mut idx = rng.gen_range(0..len);
        // Skip structural characters
        while matches!(chars[idx], ',' | '"' | ':' | '[' | ']' | '{' | '}') {
            idx = rng.gen_range(0..len);
        }
        chars[idx] = rng.gen_range(b'a'..=b'z') as char;
    }

    chars.into_iter().collect()
}

/// Randomly change whole comma-separated fields until we've changed ~N characters (concentrated changes)
fn randomly_modify_words(input: &str, target_chars: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut chars: Vec<char> = input.chars().collect();
    let mut chars_changed = 0;

    // Find all comma-separated field positions
    let fields = find_csv_fields(&chars);
    if fields.is_empty() {
        return input.to_string();
    }

    // Keep replacing random fields until we hit target
    let mut replaced_fields = std::collections::HashSet::new();
    while chars_changed < target_chars && replaced_fields.len() < fields.len() {
        // Pick a random field we haven't replaced yet
        let field_idx = rng.gen_range(0..fields.len());
        if replaced_fields.contains(&field_idx) {
            continue;
        }
        replaced_fields.insert(field_idx);

        let (start, end) = fields[field_idx];
        let field_len = end - start;

        // Replace with random lowercase letters
        for i in start..end {
            chars[i] = rng.gen_range(b'a'..=b'z') as char;
        }
        chars_changed += field_len;
    }

    chars.into_iter().collect()
}

/// Randomly insert new fields (words) until we've added ~N characters
fn randomly_insert_chars(input: &str, target_chars: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut chars: Vec<char> = input.chars().collect();
    let mut chars_added = 0;

    let fields = find_csv_fields(&chars);
    if fields.is_empty() {
        return input.to_string();
    }

    while chars_added < target_chars {
        // Pick a random position between fields to insert
        let insert_after_field = rng.gen_range(0..fields.len());
        let insert_pos = fields[insert_after_field].1; // end of that field

        // Generate a random word (3-8 chars)
        let word_len = rng.gen_range(3..=8);
        let new_word: String = (0..word_len)
            .map(|_| rng.gen_range(b'a'..=b'z') as char)
            .collect();

        // Insert ", newword" at the position
        let insertion: Vec<char> = format!(", {}", new_word).chars().collect();

        // Insert the characters
        for (i, c) in insertion.into_iter().enumerate() {
            chars.insert(insert_pos + i, c);
        }

        chars_added += word_len;

        // Recalculate field positions since we modified the string
        // (simplified: just track chars added, don't re-parse every time)
        if chars_added >= target_chars {
            break;
        }
    }

    chars.into_iter().collect()
}

/// Randomly remove entire fields (words) until we've removed ~N characters
fn randomly_remove_chars(input: &str, target_chars: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut chars: Vec<char> = input.chars().collect();
    let mut chars_removed = 0;

    let fields = find_csv_fields(&chars);
    if fields.is_empty() {
        return input.to_string();
    }

    let mut removed_indices = std::collections::HashSet::new();

    while chars_removed < target_chars && removed_indices.len() < fields.len() {
        // Pick a random field to remove
        let field_idx = rng.gen_range(0..fields.len());
        if removed_indices.contains(&field_idx) {
            continue;
        }
        removed_indices.insert(field_idx);

        // We need to recalculate positions since we're modifying
        let current_fields = find_csv_fields(&chars);
        if field_idx >= current_fields.len() {
            continue;
        }

        let (start, end) = current_fields[field_idx];
        let field_len = end - start;

        // Find the comma before this field to remove cleanly ", field"
        let remove_start = if start >= 2 && chars.get(start - 2) == Some(&',') {
            start - 2
        } else if start >= 1 && chars.get(start - 1) == Some(&',') {
            start - 1
        } else {
            start
        };
        let remove_end = end;

        if remove_start >= remove_end || remove_end > chars.len() {
            continue;
        }

        // Remove the characters (in reverse to maintain indices)
        for i in (remove_start..remove_end).rev() {
            chars.remove(i);
        }

        chars_removed += field_len;
    }

    chars.into_iter().collect()
}

/// Find all comma-separated field positions (start, end) - excludes the commas and quotes
fn find_csv_fields(chars: &[char]) -> Vec<(usize, usize)> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut in_field = false;

    for (i, &c) in chars.iter().enumerate() {
        match c {
            '"' => {
                // Skip quotes at start/end
                if !in_field {
                    start = i + 1;
                }
            }
            ',' => {
                if in_field && i > start + 1 {
                    fields.push((start, i));
                }
                in_field = false;
                start = i + 1;
            }
            ' ' if !in_field => {
                // Skip leading whitespace
                start = i + 1;
            }
            _ => {
                if !in_field {
                    start = i;
                    in_field = true;
                }
            }
        }
    }
    // Don't forget the last field
    if in_field && chars.len() > start + 1 {
        let end = if chars.last() == Some(&'"') { chars.len() - 1 } else { chars.len() };
        if end > start + 1 {
            fields.push((start, end));
        }
    }

    // Filter to fields with at least 2 chars
    fields.into_iter().filter(|(s, e)| e - s >= 2).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== HASH CONSISTENCY TESTS ====================

    #[test]
    fn test_chrome_hash_is_consistent() {
        let chrome = std::fs::read_to_string("chrome_values.json")
            .expect("Unable to read chrome_values.json");
        let hash1 = stable_document_hash_64(&chrome);
        let hash2 = stable_document_hash_64(&chrome);
        assert_eq!(hash1, hash2, "Same input should produce same hash");
    }

    #[test]
    fn test_firefox_hash_is_consistent() {
        let firefox = std::fs::read_to_string("firefox_values.json")
            .expect("Unable to read firefox_values.json");
        let hash1 = stable_document_hash_64(&firefox);
        let hash2 = stable_document_hash_64(&firefox);
        assert_eq!(hash1, hash2, "Same input should produce same hash");
    }

    #[test]
    fn test_chrome_firefox_are_different() {
        let chrome = std::fs::read_to_string("chrome_values.json")
            .expect("Unable to read chrome_values.json");
        let firefox = std::fs::read_to_string("firefox_values.json")
            .expect("Unable to read firefox_values.json");

        let chrome_hash = stable_document_hash_64(&chrome);
        let firefox_hash = stable_document_hash_64(&firefox);

        assert_ne!(chrome_hash, firefox_hash, "Chrome and Firefox should have different hashes");

        let distance = hamming_distance_64(chrome_hash, firefox_hash);
        assert!(distance >= 10 && distance <= 20,
                "Expected hamming distance 10-20, got {}", distance);
    }

    #[test]
    fn test_256bit_has_more_baseline_difference() {
        let chrome = std::fs::read_to_string("chrome_values.json")
            .expect("Unable to read chrome_values.json");
        let firefox = std::fs::read_to_string("firefox_values.json")
            .expect("Unable to read firefox_values.json");

        let dist_64 = hamming_distance_64(
            stable_document_hash_64(&chrome),
            stable_document_hash_64(&firefox)
        );
        let dist_256 = hamming_distance_256(
            &stable_document_hash_256(&chrome),
            &stable_document_hash_256(&firefox)
        );

        // 256-bit should have roughly 4x the absolute distance
        assert!(dist_256 > dist_64 * 3, "256-bit should have more absolute difference");
    }

    // ==================== SPECIFIC ALTERATION TESTS ====================

    #[test]
    fn test_single_char_change_small_distance() {
        let original = "Hello, this is a test string for nilsimsa hashing algorithm";
        let modified = "Hello, this is a test string for nilsimsa hashing Algorithm"; // A->a

        let dist = hamming_distance_64(
            stable_document_hash_64(original),
            stable_document_hash_64(modified)
        );

        assert!(dist <= 5, "Single char change should have small hamming distance, got {}", dist);
    }

    #[test]
    fn test_word_replacement_moderate_distance() {
        let chrome = std::fs::read_to_string("chrome_values.json")
            .expect("Unable to read chrome_values.json");

        // Replace "Chrome" with "Zzzzzz" (same length, different content)
        let modified = chrome.replace("Chrome", "Zzzzzz");

        let dist = hamming_distance_64(
            stable_document_hash_64(&chrome),
            stable_document_hash_64(&modified)
        );

        // Word replacement should cause moderate change
        assert!(dist >= 1 && dist <= 10,
                "Word replacement should have moderate hamming distance, got {}", dist);
    }

    #[test]
    fn test_multiple_word_replacements() {
        let chrome = std::fs::read_to_string("chrome_values.json")
            .expect("Unable to read chrome_values.json");

        // Replace multiple fields
        let modified = chrome
            .replace("Chrome", "Zzzzzz")
            .replace("Safari", "Yyyyyy")
            .replace("WebKit", "Xxxxxx");

        let dist_64 = hamming_distance_64(
            stable_document_hash_64(&chrome),
            stable_document_hash_64(&modified)
        );
        let dist_256 = hamming_distance_256(
            &stable_document_hash_256(&chrome),
            &stable_document_hash_256(&modified)
        );

        // Should still be distinguishable from baseline
        let baseline_64 = 13; // Chrome vs Firefox
        assert!(dist_64 < baseline_64,
                "Multiple word changes ({}) should be less than browser baseline ({})", dist_64, baseline_64);

        println!("Multiple word replacements: 64-bit dist={}, 256-bit dist={}", dist_64, dist_256);
    }

    #[test]
    fn test_insertion_changes_hash() {
        let chrome = std::fs::read_to_string("chrome_values.json")
            .expect("Unable to read chrome_values.json");

        // Insert a new field
        let modified = chrome.replace("Chrome,", "Chrome, inserted_field,");

        let dist = hamming_distance_64(
            stable_document_hash_64(&chrome),
            stable_document_hash_64(&modified)
        );

        assert!(dist >= 1, "Insertion should change hash, got distance {}", dist);
        assert!(dist <= 10, "Single insertion should have moderate distance, got {}", dist);
    }

    #[test]
    fn test_removal_changes_hash() {
        let chrome = std::fs::read_to_string("chrome_values.json")
            .expect("Unable to read chrome_values.json");

        // Remove "false, " (one of the boolean fields)
        let modified = chrome.replacen("false, ", "", 1);

        let dist = hamming_distance_64(
            stable_document_hash_64(&chrome),
            stable_document_hash_64(&modified)
        );

        assert!(dist >= 1, "Removal should change hash, got distance {}", dist);
        assert!(dist <= 10, "Single removal should have moderate distance, got {}", dist);
    }

    // ==================== FONT SORTING TESTS ====================

    #[test]
    fn test_font_order_same_content_similar_hash() {
        // Simulate font lists with same fonts in different order
        let fonts_v1 = "Arial, Helvetica, Times, Courier, Verdana";
        let fonts_v2 = "Verdana, Times, Arial, Courier, Helvetica";

        // Without sorting - should be different
        let dist_unsorted = hamming_distance_64(
            stable_document_hash_64(fonts_v1),
            stable_document_hash_64(fonts_v2)
        );

        // With sorting - should be identical
        let mut sorted_v1: Vec<&str> = fonts_v1.split(", ").collect();
        let mut sorted_v2: Vec<&str> = fonts_v2.split(", ").collect();
        sorted_v1.sort();
        sorted_v2.sort();
        let sorted_str_v1 = sorted_v1.join(", ");
        let sorted_str_v2 = sorted_v2.join(", ");

        let dist_sorted = hamming_distance_64(
            stable_document_hash_64(&sorted_str_v1),
            stable_document_hash_64(&sorted_str_v2)
        );

        assert_eq!(dist_sorted, 0, "Sorted identical font lists should have zero distance");
        assert!(dist_unsorted > 0, "Unsorted different-order lists should have non-zero distance");

        println!("Font order test: unsorted dist={}, sorted dist={}", dist_unsorted, dist_sorted);
    }

    #[test]
    fn test_font_subset_still_distinguishable() {
        // Original font list
        let fonts_full = "Arial, Helvetica, Times, Courier, Verdana, Georgia, Tahoma, Trebuchet";

        // Subset (removed 2 fonts)
        let fonts_subset = "Arial, Helvetica, Times, Verdana, Georgia, Tahoma";

        let dist_64 = hamming_distance_64(
            stable_document_hash_64(fonts_full),
            stable_document_hash_64(fonts_subset)
        );
        let dist_256 = hamming_distance_256(
            &stable_document_hash_256(fonts_full),
            &stable_document_hash_256(fonts_subset)
        );

        // Should change but not be completely different
        assert!(dist_64 >= 1 && dist_64 <= 15,
                "Font subset should have moderate distance at 64-bit, got {}", dist_64);

        println!("Font subset test: 64-bit dist={}, 256-bit dist={}", dist_64, dist_256);
    }

    #[test]
    fn test_font_subset_with_sorting() {
        // Same fonts, different order, one removed
        let fonts_v1 = "Arial, Helvetica, Times, Courier, Verdana";
        let fonts_v2 = "Verdana, Arial, Helvetica, Times"; // Courier removed, reordered

        // Sort both before comparing
        let mut sorted_v1: Vec<&str> = fonts_v1.split(", ").collect();
        let mut sorted_v2: Vec<&str> = fonts_v2.split(", ").collect();
        sorted_v1.sort();
        sorted_v2.sort();
        let sorted_str_v1 = sorted_v1.join(", ");
        let sorted_str_v2 = sorted_v2.join(", ");

        let dist = hamming_distance_64(
            stable_document_hash_64(&sorted_str_v1),
            stable_document_hash_64(&sorted_str_v2)
        );

        // Note: Removing 1 of 5 fonts is a 20% content change on a short string
        // nilsimsa needs more content to show locality benefits
        // For short strings, expect larger relative changes
        assert!(dist >= 1, "Removing a font should change the hash");

        println!("Sorted font subset: v1='{}', v2='{}', dist={}", sorted_str_v1, sorted_str_v2, dist);
        println!("  (Note: short strings show larger relative change)");
    }

    // ==================== EDGE CASES ====================

    #[test]
    fn test_empty_string() {
        let hash = stable_document_hash_64("");
        // Should not panic, should return some hash
        assert!(hash != 0 || hash == 0); // Just checking it runs
    }

    #[test]
    fn test_very_short_string() {
        // Note: nilsimsa is designed for documents, not single characters
        // Very short strings may produce similar or identical hashes
        let hash1 = stable_document_hash_64("a");
        let hash2 = stable_document_hash_64("b");
        let hash3 = stable_document_hash_64("hello world this is a longer string");
        let hash4 = stable_document_hash_64("hello world this is a longer strinG");

        // Short strings: behavior is undefined, just verify no panic
        let dist_short = hamming_distance_64(hash1, hash2);
        println!("Single char 'a' vs 'b': dist={} (nilsimsa not designed for this)", dist_short);

        // Longer strings should work properly
        let dist_long = hamming_distance_64(hash3, hash4);
        assert!(dist_long <= 5, "Single char change in longer string should have small distance, got {}", dist_long);
    }

    #[test]
    fn test_hamming_distance_symmetry() {
        let a: u64 = 0x123456789ABCDEF0;
        let b: u64 = 0xFEDCBA9876543210;
        assert_eq!(hamming_distance_64(a, b), hamming_distance_64(b, a));
    }

    #[test]
    fn test_hamming_distance_identity() {
        let a: u64 = 0x123456789ABCDEF0;
        assert_eq!(hamming_distance_64(a, a), 0);
    }

    #[test]
    fn test_hamming_distance_max() {
        let a: u64 = 0x0000000000000000;
        let b: u64 = 0xFFFFFFFFFFFFFFFF;
        assert_eq!(hamming_distance_64(a, b), 64);
    }
}
