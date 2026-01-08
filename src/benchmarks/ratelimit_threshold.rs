//! Rate limit threshold analysis - finding optimal hamming distance threshold

use rand::Rng;

use crate::hash::{stable_document_hash_256, hamming_distance_256};
use crate::modification::{randomly_modify_letters, generate_random_document};
use super::common::percentile;

pub fn run() {
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
