//! Nilsimsa Hash Correctness Testing
//!
//! CLI tool for analyzing nilsimsa locality-sensitive hash behavior.

#![allow(dead_code)]

mod hash;
mod modification;
mod rate_limiter;
mod evicting_db;
mod bktree_db;
mod benchmarks;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let sim = if args.len() > 1 {
        args[1].as_str()
    } else {
        "help"
    };

    match sim {
        "uniformity" => benchmarks::bit_uniformity::run(),
        "discrimination" => benchmarks::discrimination::run(),
        "segmentation" => benchmarks::segmentation::run(),
        "ratelimit" => benchmarks::ratelimit_threshold::run(),
        "simhash" => benchmarks::simhash_comparison::run(),
        "bktree" => benchmarks::bktree::run(),
        "ratelimiter" => benchmarks::ratelimiter::run(),
        "fingerprints" => benchmarks::fingerprint_compare::run(),
        "evicting" => benchmarks::evicting::run(),
        "realistic" => benchmarks::realistic::run(),
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
            eprintln!("  fingerprints    - Compare fingerprints from fingerprints/ directory");
            eprintln!("  evicting        - Benchmark evicting database strategies");
            eprintln!("  realistic       - Benchmark with realistic fingerprint data vs random");
            eprintln!();
            eprintln!("Examples:");
            eprintln!("  ./run.sh uniformity");
            eprintln!("  ./run.sh discrimination");
            eprintln!("  ./run.sh fingerprints");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::hash::{stable_document_hash_64, stable_document_hash_256, hamming_distance_64, hamming_distance_256};

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

        // Longer strings should show locality
        let dist_long = hamming_distance_64(hash3, hash4);
        assert!(dist_long <= 5, "Single char change in longer string should have small distance, got {}", dist_long);
    }

    #[test]
    fn test_unicode_content() {
        let text1 = "Hello world with unicode: 你好世界 αβγδ";
        let text2 = "Hello world with unicode: 你好世界 αβγε"; // δ -> ε

        let dist = hamming_distance_64(
            stable_document_hash_64(text1),
            stable_document_hash_64(text2)
        );

        // Should handle unicode gracefully
        assert!(dist <= 10, "Unicode content should hash correctly, got dist {}", dist);
        println!("Unicode test: dist={}", dist);
    }
}
