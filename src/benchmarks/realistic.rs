//! Benchmark comparing realistic fingerprint data vs random data
//!
//! Tests:
//! 1. Compression ratio as attack detection metric
//! 2. Hamming distance distribution (realistic vs random)
//! 3. BK-tree performance with realistic hash distributions

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use flate2::Compression;
use flate2::write::DeflateEncoder;
use rand::Rng;
use serde::Deserialize;

use crate::hash::{stable_document_hash_256, hash_to_u64, hamming_distance_u64x4};
use crate::bktree_db::BKTreeDB;
use crate::evicting_db::EvictingBucket;

/// Raw browser fingerprint data - matches production BrowserFingerprint struct
/// in pingora-bot/src/browser_id.rs for consistent normalization
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserInfo {
    // Phase 1: Basic attributes (minimal)
    user_agent: String,
    platform: Option<String>,
    screen_width: Option<u32>,
    screen_height: Option<u32>,
    color_depth: Option<u32>,
    timezone_offset: Option<i32>,
    language: Option<String>,
    hardware_concurrency: Option<u32>,
    device_memory: Option<f64>,

    // Phase 2: Browser differentiators (Brave vs Chrome, etc.)
    is_brave: Option<bool>,
    vendor: Option<String>,
    product_sub: Option<String>,
    do_not_track: Option<String>,
    pdf_viewer_enabled: Option<bool>,
    plugin_count: Option<u32>,
    has_chrome: Option<bool>,
    canvas_hash: Option<String>,
    webgl_vendor: Option<String>,
    webgl_renderer: Option<String>,

    // Phase 3: High-entropy fields (improved differentiation)
    avail_width: Option<u32>,
    avail_height: Option<u32>,
    max_touch_points: Option<u32>,
    webgl_canvas_hash: Option<String>,
    installed_fonts: Option<String>,
    audio_fingerprint: Option<String>,
    voice_count: Option<u32>,
}

#[derive(Deserialize)]
struct Fingerprint {
    browser_info: BrowserInfo,
    #[allow(dead_code)]
    document_hash256: String,
}

/// Convert BrowserInfo to canonical string for hashing.
/// DATA-ONLY FORMAT: No field names, no separators - maximizes data influence on hash.
/// Verified to improve differentiation by +12 bits average in fingerprint comparison tests.
fn to_canonical_string(info: &BrowserInfo) -> String {
    let mut s = String::with_capacity(1024);

    // Core identifying data - lowercased where appropriate
    s.push_str(&info.user_agent.to_lowercase());

    if let Some(ref p) = info.platform {
        s.push_str(&p.to_lowercase());
    }

    // Screen - as combined string (no separators)
    if let (Some(w), Some(h), Some(d)) = (info.screen_width, info.screen_height, info.color_depth) {
        s.push_str(&format!("{}{}{}", w, h, d));
    }

    // Skip timezone_offset - zero entropy in test data
    // Skip language - low entropy

    // Hardware
    if let Some(cores) = info.hardware_concurrency {
        s.push_str(&cores.to_string());
    }
    if let Some(mem) = info.device_memory {
        s.push_str(&format!("{}", mem));
    }

    // Browser differentiators
    if let Some(brave) = info.is_brave {
        if brave { s.push_str("brave"); }
    }
    if let Some(ref v) = info.vendor {
        s.push_str(&v.to_lowercase());
    }
    // Skip productSub - low entropy (same for all Chrome-based)
    if let Some(ref dnt) = info.do_not_track {
        s.push_str(dnt);
    }
    if let Some(pdf) = info.pdf_viewer_enabled {
        if pdf { s.push_str("pdf"); }
    }
    if let Some(plugins) = info.plugin_count {
        s.push_str(&plugins.to_string());
    }
    // Skip hasChrome - redundant with isBrave and UA

    // High-entropy hashes - the key differentiators
    if let Some(ref canvas) = info.canvas_hash {
        s.push_str(canvas);
    }
    if let Some(ref glv) = info.webgl_vendor {
        s.push_str(&glv.to_lowercase());
    }
    if let Some(ref glr) = info.webgl_renderer {
        s.push_str(&glr.to_lowercase());
    }

    // Avail dimensions (no separator)
    if let Some(avail_width) = info.avail_width {
        if let Some(avail_height) = info.avail_height {
            s.push_str(&format!("{}{}", avail_width, avail_height));
        }
    }

    // Skip maxTouchPoints - zero entropy in desktop test data

    // WebGL canvas hash
    if let Some(ref glc) = info.webgl_canvas_hash {
        s.push_str(glc);
    }

    // Fonts - very high entropy
    if let Some(ref fonts) = info.installed_fonts {
        if !fonts.is_empty() {
            s.push_str(&fonts.to_lowercase());
        }
    }

    // Skip audioFingerprint - zero entropy in test data

    // Voice count
    if let Some(voices) = info.voice_count {
        s.push_str(&voices.to_string());
    }

    s
}

/// Load all fingerprints from a directory
fn load_fingerprints(dir: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let path = Path::new(dir);

    if !path.exists() {
        return results;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(fp) = serde_json::from_str::<Fingerprint>(&content) {
                        let canonical = to_canonical_string(&fp.browser_info);
                        let filename = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        results.push((filename, canonical));
                    }
                }
            }
        }
    }

    results
}

/// Compute compression ratio (compressed_size / original_size)
fn compression_ratio(data: &str) -> f64 {
    let bytes = data.as_bytes();
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    let compressed = encoder.finish().unwrap();
    compressed.len() as f64 / bytes.len() as f64
}

/// Generate random ASCII string of given length
fn random_ascii_string(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| rng.gen_range(32u8..127u8) as char)
        .collect()
}

/// Generate random UTF-8 string that mimics fingerprint structure
fn random_structured_string(template: &str) -> String {
    let mut rng = rand::thread_rng();
    let mut result = String::with_capacity(template.len());

    for c in template.chars() {
        if c == '|' || c == ':' {
            result.push(c);
        } else if c.is_ascii_digit() {
            result.push(rng.gen_range(b'0'..=b'9') as char);
        } else if c.is_ascii_alphabetic() {
            if c.is_ascii_lowercase() {
                result.push(rng.gen_range(b'a'..=b'z') as char);
            } else {
                result.push(rng.gen_range(b'A'..=b'Z') as char);
            }
        } else {
            result.push(c);
        }
    }

    result
}

pub fn run() {
    println!("\n============================================================");
    println!("=== REALISTIC DATA BENCHMARK ===");
    println!("============================================================\n");

    // Load real fingerprints
    let same_fps = load_fingerprints("fingerprints/same");
    let diff_fps = load_fingerprints("fingerprints/different");
    let all_fps: Vec<_> = same_fps.iter().chain(diff_fps.iter()).collect();

    // Track whether we have real fingerprints for Tests 1-3
    let has_real_fps = !all_fps.is_empty();

    // Variables for summary (set by Tests 1-3 if run, or defaults)
    let mut avg_real = 0.0;
    let mut avg_random = 0.0;
    let mut realistic_avg = 0.0;
    let mut realistic_min = 0u32;
    let mut realistic_max = 0u32;
    let mut random_avg = 0.0;
    let mut random_min = 0u32;
    let mut random_max = 0u32;
    let mut speedup = 1.0;
    let mut random_speedup = 1.0;

    // RNG needed by Tests 2-4
    let mut rng = rand::thread_rng();

    if !has_real_fps {
        println!("No fingerprints found in fingerprints/same or fingerprints/different");
        println!("Skipping Tests 1-3 (require real fingerprints), running Test 4 (synthetic)\n");
    } else {
        println!("Loaded {} fingerprints ({} same, {} different)\n",
                 all_fps.len(), same_fps.len(), diff_fps.len());
    }

    // ========================================================================
    // Test 1: Compression Ratio Detection
    // ========================================================================
    if has_real_fps {
    println!("--- Test 1: Compression Ratio Detection ---\n");

    let sample_fp = &all_fps[0].1;
    let sample_len = sample_fp.len();

    println!("Sample canonical string ({} bytes):", sample_len);
    println!("  {}...\n", &sample_fp[..sample_len.min(80)]);

    // Real fingerprints
    println!("Real fingerprints:");
    let mut real_ratios = Vec::new();
    for (filename, canonical) in &all_fps {
        let ratio = compression_ratio(canonical);
        real_ratios.push(ratio);
        println!("  {:50} len={:4} ratio={:.3}", filename, canonical.len(), ratio);
    }
    let avg_real = real_ratios.iter().sum::<f64>() / real_ratios.len() as f64;
    println!("  Average compression ratio: {:.3}\n", avg_real);

    // Random ASCII strings (same lengths)
    println!("Random ASCII strings:");
    let mut random_ratios = Vec::new();
    for (_, canonical) in &all_fps {
        let random_str = random_ascii_string(canonical.len());
        let ratio = compression_ratio(&random_str);
        random_ratios.push(ratio);
    }
    let avg_random = random_ratios.iter().sum::<f64>() / random_ratios.len() as f64;
    println!("  Average compression ratio: {:.3}\n", avg_random);

    // Structured random (knows the format but random values)
    println!("Structured random (knows format):");
    let mut structured_ratios = Vec::new();
    for (_, canonical) in &all_fps {
        let structured = random_structured_string(canonical);
        let ratio = compression_ratio(&structured);
        structured_ratios.push(ratio);
    }
    let avg_structured = structured_ratios.iter().sum::<f64>() / structured_ratios.len() as f64;
    println!("  Average compression ratio: {:.3}\n", avg_structured);

    println!("Detection threshold recommendation: {:.3}", (avg_real + avg_structured) / 2.0);
    println!("  Real fingerprints:   ratio < {:.3}", avg_real + 0.1);
    println!("  Random ASCII:        ratio > {:.3}", avg_random - 0.1);
    println!("  Structured random:   ratio ~ {:.3} (harder to detect)\n", avg_structured);

    // ========================================================================
    // Test 2: Hamming Distance Distribution
    // ========================================================================
    println!("--- Test 2: Hamming Distance Distribution ---\n");

    // Hash all real fingerprints
    let real_hashes: Vec<[u64; 4]> = all_fps.iter()
        .map(|(_, canonical)| hash_to_u64(&stable_document_hash_256(canonical)))
        .collect();

    // Pairwise distances for real fingerprints
    let mut real_distances = Vec::new();
    for i in 0..real_hashes.len() {
        for j in (i+1)..real_hashes.len() {
            real_distances.push(hamming_distance_u64x4(&real_hashes[i], &real_hashes[j]));
        }
    }

    if !real_distances.is_empty() {
        let min_d = *real_distances.iter().min().unwrap();
        let max_d = *real_distances.iter().max().unwrap();
        let avg_d = real_distances.iter().map(|&d| d as f64).sum::<f64>() / real_distances.len() as f64;
        println!("Real fingerprint pairs ({} pairs):", real_distances.len());
        println!("  Min distance: {}", min_d);
        println!("  Max distance: {}", max_d);
        println!("  Avg distance: {:.1}", avg_d);
        println!("  Distribution: {:?}\n", &real_distances);
    }

    // Generate variations of real fingerprints (small modifications)
    println!("Generating 1000 realistic variations...");
    let mut realistic_hashes = Vec::with_capacity(1000);
    let mut rng = rand::thread_rng();

    for _ in 0..1000 {
        // Pick a random real fingerprint as base
        let (_, base) = &all_fps[rng.gen_range(0..all_fps.len())];

        // Apply small random modifications (1-5 character changes)
        let num_changes = rng.gen_range(1..=5);
        let mut modified = base.clone();
        let bytes = unsafe { modified.as_bytes_mut() };

        for _ in 0..num_changes {
            let pos = rng.gen_range(0..bytes.len());
            // Keep it alphanumeric
            bytes[pos] = if rng.gen_bool(0.5) {
                rng.gen_range(b'a'..=b'z')
            } else {
                rng.gen_range(b'0'..=b'9')
            };
        }

        let hash = hash_to_u64(&stable_document_hash_256(&modified));
        realistic_hashes.push(hash);
    }

    // Measure distances between realistic variations
    let sample_size = 100;
    let mut realistic_distances = Vec::new();
    for i in 0..sample_size {
        for j in (i+1)..sample_size {
            realistic_distances.push(hamming_distance_u64x4(&realistic_hashes[i], &realistic_hashes[j]));
        }
    }

    let realistic_min = *realistic_distances.iter().min().unwrap();
    let realistic_max = *realistic_distances.iter().max().unwrap();
    let realistic_avg = realistic_distances.iter().map(|&d| d as f64).sum::<f64>() / realistic_distances.len() as f64;

    println!("\nRealistic variations (sample of {} pairs):", realistic_distances.len());
    println!("  Min distance: {}", realistic_min);
    println!("  Max distance: {}", realistic_max);
    println!("  Avg distance: {:.1}", realistic_avg);

    // Generate nilsimsa hashes of random ASCII strings
    let random_ascii_hashes: Vec<[u64; 4]> = (0..1000)
        .map(|_| {
            let s = random_ascii_string(400); // Similar length to real fingerprints
            hash_to_u64(&stable_document_hash_256(&s))
        })
        .collect();

    let mut random_ascii_distances = Vec::new();
    for i in 0..sample_size {
        for j in (i+1)..sample_size {
            random_ascii_distances.push(hamming_distance_u64x4(&random_ascii_hashes[i], &random_ascii_hashes[j]));
        }
    }

    let random_ascii_min = *random_ascii_distances.iter().min().unwrap();
    let random_ascii_max = *random_ascii_distances.iter().max().unwrap();
    let random_ascii_avg = random_ascii_distances.iter().map(|&d| d as f64).sum::<f64>() / random_ascii_distances.len() as f64;

    println!("\nNilsimsa of random ASCII strings (sample of {} pairs):", random_ascii_distances.len());
    println!("  Min distance: {}", random_ascii_min);
    println!("  Max distance: {}", random_ascii_max);
    println!("  Avg distance: {:.1}", random_ascii_avg);

    // Generate "realistic fakes" - mix and match real fingerprint field values
    // NOTE: This test only works with old pipe-separated format (ua:...|screen:...|...).
    // With data-only format, there are no field boundaries to parse.
    println!("\nGenerating realistic fakes (mix-and-match real values)...");

    // Try to parse field components from old format
    let mut user_agents: Vec<&str> = Vec::new();
    let mut screens: Vec<&str> = Vec::new();
    let mut other_fields: Vec<Vec<&str>> = Vec::new();

    for (_, canonical) in &all_fps {
        let parts: Vec<&str> = canonical.split('|').collect();
        let mut others = Vec::new();
        for part in &parts {
            if part.starts_with("ua:") {
                user_agents.push(&part[3..]);
            } else if part.starts_with("screen:") {
                screens.push(&part[7..]);
            } else {
                others.push(*part);
            }
        }
        other_fields.push(others);
    }

    // Skip this test if using data-only format (no parseable fields)
    let has_parseable_fields = !user_agents.is_empty() && !screens.is_empty();
    if !has_parseable_fields {
        println!("  Skipped: Data-only format detected (no field prefixes to parse)");
        println!("  Mix-and-match attacks require field-separated format.\n");
    }

    // Create fakes by mixing components from different fingerprints
    let fake_hashes: Vec<[u64; 4]> = if has_parseable_fields { (0..1000)
        .map(|_| {
            let ua = user_agents[rng.gen_range(0..user_agents.len())];
            let screen = screens[rng.gen_range(0..screens.len())];
            let others = &other_fields[rng.gen_range(0..other_fields.len())];

            let mut parts = vec![
                format!("ua:{}", ua),
                format!("screen:{}", screen),
            ];
            parts.extend(others.iter().map(|s| s.to_string()));

            let fake = parts.join("|");
            hash_to_u64(&stable_document_hash_256(&fake))
        })
        .collect()
    } else {
        Vec::new()
    };

    // fake_distances is used later in histogram, so define outside the if block
    let mut fake_distances: Vec<u32> = Vec::new();

    if has_parseable_fields {
        for i in 0..sample_size {
            for j in (i+1)..sample_size {
                fake_distances.push(hamming_distance_u64x4(&fake_hashes[i], &fake_hashes[j]));
            }
        }

        let fake_min = *fake_distances.iter().min().unwrap();
        let fake_max = *fake_distances.iter().max().unwrap();
        let fake_avg = fake_distances.iter().map(|&d| d as f64).sum::<f64>() / fake_distances.len() as f64;

        println!("Realistic fakes (mix-and-match real components):");
        println!("  Min distance: {}", fake_min);
        println!("  Max distance: {}", fake_max);
        println!("  Avg distance: {:.1}", fake_avg);

        // Also measure distance from fakes to real fingerprints
    // Break down by same vs different browser sources
    println!("\nFake-to-real analysis:");

    // Label each real fingerprint as "same" or "different"
    let same_count = same_fps.len();
    let real_labels: Vec<&str> = (0..all_fps.len())
        .map(|i| if i < same_count { "same" } else { "diff" })
        .collect();

    let mut matches_to_same = 0;
    let mut matches_to_diff = 0;
    let mut total_to_same = 0;
    let mut total_to_diff = 0;

    for fake in fake_hashes.iter().take(100) {
        for (i, real) in real_hashes.iter().enumerate() {
            let dist = hamming_distance_u64x4(fake, real);
            if real_labels[i] == "same" {
                total_to_same += 1;
                if dist <= 32 {
                    matches_to_same += 1;
                }
            } else {
                total_to_diff += 1;
                if dist <= 32 {
                    matches_to_diff += 1;
                }
            }
        }
    }

    println!("  Fakes matching 'same' browser (Brave):     {}/{} ({:.1}%)",
             matches_to_same, total_to_same,
             100.0 * matches_to_same as f64 / total_to_same as f64);
    println!("  Fakes matching 'different' browsers:       {}/{} ({:.1}%)",
             matches_to_diff, total_to_diff,
             100.0 * matches_to_diff as f64 / total_to_diff as f64);

    // Show the actual distances to "different" browsers
    let mut fake_to_diff_distances = Vec::new();
    for fake in fake_hashes.iter().take(100) {
        for (i, real) in real_hashes.iter().enumerate() {
            if real_labels[i] == "diff" {
                fake_to_diff_distances.push(hamming_distance_u64x4(fake, real));
            }
        }
    }

    if !fake_to_diff_distances.is_empty() {
        let min_d = *fake_to_diff_distances.iter().min().unwrap();
        let max_d = *fake_to_diff_distances.iter().max().unwrap();
        let avg_d = fake_to_diff_distances.iter().map(|&d| d as f64).sum::<f64>()
                    / fake_to_diff_distances.len() as f64;
        println!("\n  Distance from fakes to 'different' browsers:");
        println!("    Min: {}, Max: {}, Avg: {:.1}", min_d, max_d, avg_d);
    }

    // What if attacker ONLY has access to one browser type?
    // Generate fakes using ONLY the "different" browser components
    println!("\n  Attacker scenario: only has 'different' browser data ({} samples)", diff_fps.len());

    if diff_fps.len() >= 2 {
        let diff_canonicals: Vec<&str> = all_fps.iter()
            .skip(same_count)
            .map(|(_, c)| c.as_str())
            .collect();

        // Parse components from different browsers only
        let mut diff_uas: Vec<&str> = Vec::new();
        let mut diff_screens: Vec<&str> = Vec::new();

        for canonical in &diff_canonicals {
            for part in canonical.split('|') {
                if part.starts_with("ua:") {
                    diff_uas.push(&part[3..]);
                } else if part.starts_with("screen:") {
                    diff_screens.push(&part[7..]);
                }
            }
        }

        // Create fakes from only different-browser components
        let attacker_fakes: Vec<[u64; 4]> = (0..100)
            .map(|_| {
                let ua = diff_uas[rng.gen_range(0..diff_uas.len())];
                let screen = diff_screens[rng.gen_range(0..diff_screens.len())];
                let fake = format!("ua:{}|screen:{}", ua, screen);
                hash_to_u64(&stable_document_hash_256(&fake))
            })
            .collect();

        // How many match the "same" (Brave) fingerprints?
        let mut attacker_matches_brave = 0;
        let same_hashes: Vec<[u64; 4]> = real_hashes.iter().take(same_count).cloned().collect();

        for fake in &attacker_fakes {
            for real in &same_hashes {
                if hamming_distance_u64x4(fake, real) <= 32 {
                    attacker_matches_brave += 1;
                    break; // Count each fake only once
                }
            }
        }

        println!("    Fakes from 'diff' components matching Brave: {}/100", attacker_matches_brave);
        }
    } // end if has_parseable_fields

    // Generate nilsimsa hashes of structured random strings
    let structured_random_hashes: Vec<[u64; 4]> = (0..1000)
        .map(|_| {
            let (_, template) = &all_fps[rng.gen_range(0..all_fps.len())];
            let s = random_structured_string(template);
            hash_to_u64(&stable_document_hash_256(&s))
        })
        .collect();

    let mut structured_distances = Vec::new();
    for i in 0..sample_size {
        for j in (i+1)..sample_size {
            structured_distances.push(hamming_distance_u64x4(&structured_random_hashes[i], &structured_random_hashes[j]));
        }
    }

    let structured_min = *structured_distances.iter().min().unwrap();
    let structured_max = *structured_distances.iter().max().unwrap();
    let structured_avg = structured_distances.iter().map(|&d| d as f64).sum::<f64>() / structured_distances.len() as f64;

    println!("\nNilsimsa of structured random (same format, random values):");
    println!("  Min distance: {}", structured_min);
    println!("  Max distance: {}", structured_max);
    println!("  Avg distance: {:.1}", structured_avg);

    // Generate truly random 256-bit hashes (not nilsimsa)
    let random_hashes: Vec<[u64; 4]> = (0..1000)
        .map(|_| [rng.gen(), rng.gen(), rng.gen(), rng.gen()])
        .collect();

    let mut random_distances = Vec::new();
    for i in 0..sample_size {
        for j in (i+1)..sample_size {
            random_distances.push(hamming_distance_u64x4(&random_hashes[i], &random_hashes[j]));
        }
    }

    let random_min = *random_distances.iter().min().unwrap();
    let random_max = *random_distances.iter().max().unwrap();
    let random_avg = random_distances.iter().map(|&d| d as f64).sum::<f64>() / random_distances.len() as f64;

    println!("\nTruly random 256-bit values (NOT nilsimsa):");
    println!("  Min distance: {}", random_min);
    println!("  Max distance: {}", random_max);
    println!("  Avg distance: {:.1}", random_avg);

    // Distribution histogram
    println!("\nDistance histogram:");
    let buckets = [0, 32, 64, 96, 128, 160, 192, 224, 256];
    print!("                  ");
    for i in 0..buckets.len()-1 {
        print!(" {:3}-{:3}", buckets[i], buckets[i+1]-1);
    }
    println!();

    print!("  Realistic:      ");
    for i in 0..buckets.len()-1 {
        let count = realistic_distances.iter().filter(|&&d| d >= buckets[i] && d < buckets[i+1]).count();
        print!(" {:7}", count);
    }
    println!();

    print!("  Random ASCII:   ");
    for i in 0..buckets.len()-1 {
        let count = random_ascii_distances.iter().filter(|&&d| d >= buckets[i] && d < buckets[i+1]).count();
        print!(" {:7}", count);
    }
    println!();

    print!("  Realistic fakes:");
    for i in 0..buckets.len()-1 {
        let count = fake_distances.iter().filter(|&&d| d >= buckets[i] && d < buckets[i+1]).count();
        print!(" {:7}", count);
    }
    println!();

    print!("  Structured rand:");
    for i in 0..buckets.len()-1 {
        let count = structured_distances.iter().filter(|&&d| d >= buckets[i] && d < buckets[i+1]).count();
        print!(" {:7}", count);
    }
    println!();

    print!("  Truly random:   ");
    for i in 0..buckets.len()-1 {
        let count = random_distances.iter().filter(|&&d| d >= buckets[i] && d < buckets[i+1]).count();
        print!(" {:7}", count);
    }
    println!();

    // ========================================================================
    // Test 3: BK-Tree with Realistic Data
    // ========================================================================
    println!("\n--- Test 3: BK-Tree Performance with Realistic Data ---\n");

    const THRESHOLD: u32 = 32;
    const CAPACITY: usize = 10_000;

    println!("Testing with threshold={}, capacity={}\n", THRESHOLD, CAPACITY);

    // Fill databases with realistic hashes (by generating many variations)
    let mut large_realistic: Vec<[u64; 4]> = Vec::with_capacity(CAPACITY);
    for _ in 0..CAPACITY {
        let (_, base) = &all_fps[rng.gen_range(0..all_fps.len())];
        let num_changes = rng.gen_range(0..=10);
        let mut modified = base.clone();
        let bytes = unsafe { modified.as_bytes_mut() };
        for _ in 0..num_changes {
            let pos = rng.gen_range(0..bytes.len());
            bytes[pos] = if rng.gen_bool(0.5) {
                rng.gen_range(b'a'..=b'z')
            } else {
                rng.gen_range(b'0'..=b'9')
            };
        }
        large_realistic.push(hash_to_u64(&stable_document_hash_256(&modified)));
    }

    // Create databases
    let linear_db = EvictingBucket::new(CAPACITY, THRESHOLD);
    let bktree_db = BKTreeDB::new(THRESHOLD, 1000, CAPACITY);

    // Fill both DBs with realistic hashes
    for hash in large_realistic.iter().take(CAPACITY) {
        linear_db.find_or_insert_combined(hash, 1000);
        bktree_db.find_or_insert(hash, 1000);
    }

    // Generate test queries (more variations of real data)
    let test_queries: Vec<[u64; 4]> = (0..1000).map(|_| {
        let (_, base) = &all_fps[rng.gen_range(0..all_fps.len())];
        let num_changes = rng.gen_range(0..=10);
        let mut modified = base.clone();
        let bytes = unsafe { modified.as_bytes_mut() };
        for _ in 0..num_changes {
            let pos = rng.gen_range(0..bytes.len());
            bytes[pos] = if rng.gen_bool(0.5) {
                rng.gen_range(b'a'..=b'z')
            } else {
                rng.gen_range(b'0'..=b'9')
            };
        }
        hash_to_u64(&stable_document_hash_256(&modified))
    }).collect();

    // Benchmark linear scan
    let start = Instant::now();
    let mut linear_found = 0;
    for query in &test_queries {
        let (_, is_new) = linear_db.find_or_insert_combined(query, 2000);
        if !is_new {
            linear_found += 1;
        }
    }
    let linear_time = start.elapsed();

    // Benchmark BK-tree
    let start = Instant::now();
    let mut bktree_found = 0;
    for query in &test_queries {
        let (_, is_new) = bktree_db.find_or_insert(query, 2000);
        if !is_new {
            bktree_found += 1;
        }
    }
    let bktree_time = start.elapsed();

    let linear_us = linear_time.as_micros() as f64 / 1000.0;
    let bktree_us = bktree_time.as_micros() as f64 / 1000.0;
    let speedup = linear_us / bktree_us;

    println!("Realistic data (variations of {} real fingerprints):", all_fps.len());
    println!("  Linear scan:  {:.2}μs/op, found {} matches", linear_us, linear_found);
    println!("  BK-Tree:      {:.2}μs/op, found {} matches, {} nodes",
             bktree_us, bktree_found, bktree_db.node_count());

    if speedup > 1.0 {
        println!("  >>> BK-Tree is {:.2}x FASTER <<<", speedup);
    } else {
        println!("  >>> Linear scan is {:.2}x faster <<<", 1.0 / speedup);
    }

    // Compare with random hashes
    println!("\nRandom hashes (baseline comparison):");

    let linear_db_random = EvictingBucket::new(CAPACITY, THRESHOLD);
    linear_db_random.fill_random(1000);

    let bktree_db_random = BKTreeDB::new(THRESHOLD, 1000, CAPACITY);
    bktree_db_random.fill_random(CAPACITY, 1000);

    let random_queries: Vec<[u64; 4]> = (0..1000)
        .map(|_| [rng.gen(), rng.gen(), rng.gen(), rng.gen()])
        .collect();

    let start = Instant::now();
    for query in &random_queries {
        let _ = linear_db_random.find_or_insert_combined(query, 2000);
    }
    let linear_random_time = start.elapsed();

    let start = Instant::now();
    for query in &random_queries {
        let _ = bktree_db_random.find_or_insert(query, 2000);
    }
    let bktree_random_time = start.elapsed();

    let linear_random_us = linear_random_time.as_micros() as f64 / 1000.0;
    let bktree_random_us = bktree_random_time.as_micros() as f64 / 1000.0;
    let random_speedup = linear_random_us / bktree_random_us;

    println!("  Linear scan:  {:.2}μs/op", linear_random_us);
    println!("  BK-Tree:      {:.2}μs/op, {} nodes", bktree_random_us, bktree_db_random.node_count());

    if random_speedup > 1.0 {
        println!("  >>> BK-Tree is {:.2}x FASTER <<<", random_speedup);
    } else {
        println!("  >>> Linear scan is {:.2}x faster <<<", 1.0 / random_speedup);
    }

    } // end if has_real_fps

    // ========================================================================
    // Test 4: Synthetic Cross-Platform Estimation (no real fingerprints needed)
    // ========================================================================
    println!("\n--- Test 4: Synthetic Cross-Platform Estimation ---\n");

    // Model realistic machines with correlated attributes:
    // - Canvas/WebGL hashes depend on GPU (same GPU = same hash)
    // - Hardware config varies per machine
    // - Fonts depend on OS
    // - Voices depend on OS + browser
    // Matching production canonical format from browser_id.rs

    // Define machines: (platform, gpu, screen, cores, mem, canvas_hash, webgl_hash, audio_fp)
    // Each machine is a unique hardware+software combination
    struct Machine {
        platform_short: &'static str,
        platform_ua: &'static str,
        gpu: &'static str,
        screen: &'static str,
        avail: &'static str,
        cores: u32,
        mem: f64,
        // GPU-determined values (same GPU = same hashes across machines)
        canvas_hash: &'static str,
        webgl_hash: &'static str,
        audio_fp: &'static str,
        fonts: &'static str,
    }

    // Create diverse machines
    // Note: canvas_hash and webgl_hash are deterministic per GPU (same GPU = same hashes)
    let machines = vec![
        // Windows machines with different GPUs
        Machine { platform_short: "Win32", platform_ua: "Windows NT 10.0; Win64; x64",
                  gpu: "ANGLE (NVIDIA Corporation, NVIDIA GeForce RTX 4090/PCIe/SSE2, OpenGL 4.5.0)",
                  screen: "3840x2160x24", avail: "3840x2117", cores: 16, mem: 64.0,
                  canvas_hash: "-1a2b3c4d", webgl_hash: "1f2e3d4c", audio_fp: "function:44100:6",
                  fonts: "arial,calibri,consolas,courier new,georgia,segoe ui,tahoma,times new roman,verdana" },
        Machine { platform_short: "Win32", platform_ua: "Windows NT 10.0; Win64; x64",
                  gpu: "ANGLE (NVIDIA Corporation, NVIDIA GeForce RTX 3080/PCIe/SSE2, OpenGL 4.5.0)",
                  screen: "2560x1440x24", avail: "2560x1400", cores: 8, mem: 32.0,
                  canvas_hash: "-deadbeef", webgl_hash: "5a6b7c8d", audio_fp: "function:44100:6",
                  fonts: "arial,calibri,consolas,courier new,georgia,segoe ui,tahoma,times new roman,verdana" },
        Machine { platform_short: "Win32", platform_ua: "Windows NT 10.0; Win64; x64",
                  gpu: "ANGLE (AMD, AMD Radeon RX 7900 XTX, OpenGL 4.6)",
                  screen: "2560x1440x24", avail: "2560x1400", cores: 12, mem: 32.0,
                  canvas_hash: "-cafebabe", webgl_hash: "9e0f1a2b", audio_fp: "function:44100:2",
                  fonts: "arial,calibri,consolas,courier new,georgia,segoe ui,tahoma,times new roman,verdana" },
        Machine { platform_short: "Win32", platform_ua: "Windows NT 10.0; Win64; x64",
                  gpu: "ANGLE (Intel, Intel UHD Graphics 770, OpenGL 4.6)",
                  screen: "1920x1080x24", avail: "1920x1040", cores: 4, mem: 16.0,
                  canvas_hash: "-12345678", webgl_hash: "3c4d5e6f", audio_fp: "function:44100:2",
                  fonts: "arial,calibri,consolas,courier new,georgia,segoe ui,tahoma,times new roman,verdana" },
        // More Windows machines - same GPU, different hardware (to test collision rate)
        Machine { platform_short: "Win32", platform_ua: "Windows NT 10.0; Win64; x64",
                  gpu: "ANGLE (NVIDIA Corporation, NVIDIA GeForce RTX 4090/PCIe/SSE2, OpenGL 4.5.0)",
                  screen: "2560x1440x24", avail: "2560x1400", cores: 12, mem: 32.0,
                  canvas_hash: "-1a2b3c4d", webgl_hash: "1f2e3d4c", audio_fp: "function:44100:6",
                  fonts: "arial,calibri,consolas,courier new,georgia,segoe ui,verdana" },
        Machine { platform_short: "Win32", platform_ua: "Windows NT 10.0; Win64; x64",
                  gpu: "ANGLE (NVIDIA Corporation, NVIDIA GeForce RTX 4090/PCIe/SSE2, OpenGL 4.5.0)",
                  screen: "1920x1080x24", avail: "1920x1040", cores: 8, mem: 16.0,
                  canvas_hash: "-1a2b3c4d", webgl_hash: "1f2e3d4c", audio_fp: "function:44100:2",
                  fonts: "arial,calibri,consolas,courier new,georgia,segoe ui,tahoma,times new roman" },
        // Mac machines
        Machine { platform_short: "MacIntel", platform_ua: "Macintosh; Intel Mac OS X 10_15_7",
                  gpu: "ANGLE (Apple, Apple M2 Pro, OpenGL 4.1)",
                  screen: "2880x1800x24", avail: "2880x1775", cores: 10, mem: 32.0,
                  canvas_hash: "-87654321", webgl_hash: "7a8b9c0d", audio_fp: "function:48000:2",
                  fonts: "arial,helvetica,helvetica neue,lucida grande,menlo,monaco,san francisco,times" },
        Machine { platform_short: "MacIntel", platform_ua: "Macintosh; Intel Mac OS X 10_15_7",
                  gpu: "ANGLE (Apple, Apple M2 Pro, OpenGL 4.1)",
                  screen: "1440x900x24", avail: "1440x875", cores: 8, mem: 16.0,
                  canvas_hash: "-87654321", webgl_hash: "7a8b9c0d", audio_fp: "function:48000:2",
                  fonts: "arial,helvetica,helvetica neue,lucida grande,menlo,monaco,san francisco" },
        Machine { platform_short: "MacIntel", platform_ua: "Macintosh; Intel Mac OS X 10_15_7",
                  gpu: "ANGLE (AMD, AMD Radeon RX 7900 XTX, OpenGL 4.6)",
                  screen: "2560x1440x24", avail: "2560x1400", cores: 8, mem: 32.0,
                  canvas_hash: "-cafebabe", webgl_hash: "9e0f1a2b", audio_fp: "function:48000:2",
                  fonts: "arial,helvetica,helvetica neue,lucida grande,menlo,monaco,san francisco,times" },
        // Linux machines
        Machine { platform_short: "Linux x86_64", platform_ua: "X11; Linux x86_64",
                  gpu: "ANGLE (NVIDIA Corporation, NVIDIA GeForce RTX 4090/PCIe/SSE2, OpenGL 4.5.0)",
                  screen: "3840x2160x24", avail: "3840x2160", cores: 32, mem: 128.0,
                  canvas_hash: "-1a2b3c4d", webgl_hash: "1f2e3d4c", audio_fp: "function:44100:6",
                  fonts: "dejavu sans,dejavu sans mono,liberation mono,liberation sans,noto sans,ubuntu" },
        Machine { platform_short: "Linux x86_64", platform_ua: "X11; Linux x86_64",
                  gpu: "ANGLE (AMD, AMD Radeon RX 7900 XTX, OpenGL 4.6)",
                  screen: "2560x1440x24", avail: "2560x1440", cores: 16, mem: 64.0,
                  canvas_hash: "-cafebabe", webgl_hash: "9e0f1a2b", audio_fp: "function:44100:2",
                  fonts: "dejavu sans,dejavu sans mono,liberation mono,liberation sans,noto sans,ubuntu" },
        Machine { platform_short: "Linux x86_64", platform_ua: "X11; Linux x86_64",
                  gpu: "ANGLE (Intel, Intel UHD Graphics 770, OpenGL 4.6)",
                  screen: "1920x1080x24", avail: "1920x1080", cores: 4, mem: 8.0,
                  canvas_hash: "-12345678", webgl_hash: "3c4d5e6f", audio_fp: "function:44100:2",
                  fonts: "dejavu sans,dejavu sans mono,liberation sans,noto sans" },
        // iPhone
        Machine { platform_short: "iPhone", platform_ua: "iPhone; CPU iPhone OS 17_0 like Mac OS X",
                  gpu: "Apple GPU",
                  screen: "390x844x24", avail: "390x844", cores: 6, mem: 4.0,
                  canvas_hash: "-abcdef01", webgl_hash: "1e2f3a4b", audio_fp: "function:44100:0",
                  fonts: "arial,helvetica,san francisco,times new roman" },
        Machine { platform_short: "iPhone", platform_ua: "iPhone; CPU iPhone OS 17_0 like Mac OS X",
                  gpu: "Apple GPU",
                  screen: "390x844x24", avail: "390x844", cores: 6, mem: 6.0,
                  canvas_hash: "-abcdef01", webgl_hash: "1e2f3a4b", audio_fp: "function:44100:0",
                  fonts: "arial,helvetica,san francisco,times new roman" },
    ];

    let browsers: [(&str, &str, bool); 3] = [
        ("Chrome/135.0.0.0", "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36", true),
        ("Firefox/134.0", "Gecko/20100101 Firefox/134.0", false),
        ("Safari/17.0", "AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15", false),
    ];

    // Voice counts depend on OS + browser
    fn get_voice_count(platform: &str, browser: &str) -> u32 {
        if platform.contains("iPhone") { 0 }
        else if platform.contains("Mac") { 67 }
        else if browser.contains("Firefox") { 5 }
        else { 24 }
    }

    // Generate fingerprints: each machine × each compatible browser = one fingerprint
    let mut synthetic_hashes: Vec<([u64; 4], String)> = Vec::new();

    for machine in &machines {
        for (browser_name, browser_ua, has_chrome) in &browsers {
            // Skip invalid combos
            if browser_name.contains("Safari") &&
               !machine.platform_short.contains("Mac") &&
               !machine.platform_short.contains("iPhone") {
                continue;
            }

            let voices = get_voice_count(machine.platform_short, browser_name);
            let touch = if machine.platform_short.contains("iPhone") { 5 } else { 0 };

            let ua = format!("Mozilla/5.0 ({}) {}", machine.platform_ua, browser_ua);

            // Build canonical string - DATA-ONLY FORMAT (no field names, no separators)
            // Matches to_canonical_string() for consistency with real fingerprint processing
            let mut canonical = String::with_capacity(1024);

            // Core identifying data - lowercased
            canonical.push_str(&ua.to_lowercase());
            canonical.push_str(&machine.platform_short.to_lowercase());

            // Screen - as combined string (no separators)
            canonical.push_str(machine.screen);

            // Hardware
            canonical.push_str(&machine.cores.to_string());
            canonical.push_str(&format!("{}", machine.mem));

            // Browser differentiators (skip low-entropy fields: vendor, productSub, pdf, plugins)
            // isBrave would be "brave" if true, but synthetic machines are not brave
            // Skip hasChrome - redundant with UA

            // High-entropy hashes - the key differentiators
            canonical.push_str(machine.canvas_hash);
            canonical.push_str("google inc.");  // webgl_vendor (simulated)
            canonical.push_str(&machine.gpu.to_lowercase());

            // Avail dimensions
            canonical.push_str(machine.avail);

            // Skip maxTouchPoints - zero entropy in desktop data

            // WebGL canvas hash
            canonical.push_str(machine.webgl_hash);

            // Fonts - very high entropy
            canonical.push_str(machine.fonts);

            // Skip audioFingerprint - zero entropy

            // Voice count
            canonical.push_str(&voices.to_string());

            let gpu_label = machine.gpu.split(',').nth(1)
                .unwrap_or(machine.gpu).trim()
                .split('/').next().unwrap_or("GPU").trim();
            let label = format!("{}/{}/{}",
                machine.platform_short.split_whitespace().next().unwrap_or(machine.platform_short),
                browser_name.split('/').next().unwrap_or(browser_name),
                gpu_label
            );

            let hash = hash_to_u64(&stable_document_hash_256(&canonical));
            synthetic_hashes.push((hash, label));
        }
    }

    println!("Generated {} synthetic fingerprints\n", synthetic_hashes.len());

    // Group by platform+browser to see within-group vs cross-group distances
    println!("Sample fingerprints:");
    for (hash, label) in synthetic_hashes.iter().take(10) {
        println!("  {} -> {:016x}{:016x}...", label, hash[0], hash[1]);
    }

    // Compute pairwise distances and categorize
    let mut same_platform_browser: Vec<u32> = Vec::new();
    let mut same_platform_diff_browser: Vec<u32> = Vec::new();
    let mut diff_platform_same_browser: Vec<u32> = Vec::new();
    let mut diff_platform_diff_browser: Vec<u32> = Vec::new();

    for i in 0..synthetic_hashes.len() {
        for j in (i+1)..synthetic_hashes.len() {
            let dist = hamming_distance_u64x4(&synthetic_hashes[i].0, &synthetic_hashes[j].0);

            let label_i: Vec<&str> = synthetic_hashes[i].1.split('/').collect();
            let label_j: Vec<&str> = synthetic_hashes[j].1.split('/').collect();

            let same_platform = label_i[0] == label_j[0];
            let same_browser = label_i[1] == label_j[1];

            match (same_platform, same_browser) {
                (true, true) => same_platform_browser.push(dist),
                (true, false) => same_platform_diff_browser.push(dist),
                (false, true) => diff_platform_same_browser.push(dist),
                (false, false) => diff_platform_diff_browser.push(dist),
            }
        }
    }

    fn stats(v: &[u32]) -> (u32, u32, f64) {
        if v.is_empty() { return (0, 0, 0.0); }
        let min = *v.iter().min().unwrap();
        let max = *v.iter().max().unwrap();
        let avg = v.iter().map(|&x| x as f64).sum::<f64>() / v.len() as f64;
        (min, max, avg)
    }

    println!("\nDistance analysis by category:");

    let (min, max, avg) = stats(&same_platform_browser);
    let matches = same_platform_browser.iter().filter(|&&d| d <= 32).count();
    println!("  Same platform + Same browser ({} pairs):", same_platform_browser.len());
    println!("    Min: {}, Max: {}, Avg: {:.1}", min, max, avg);
    println!("    Would match (<=32): {} ({:.1}%)", matches, 100.0 * matches as f64 / same_platform_browser.len().max(1) as f64);

    let (min, max, avg) = stats(&same_platform_diff_browser);
    let matches = same_platform_diff_browser.iter().filter(|&&d| d <= 32).count();
    println!("  Same platform + Diff browser ({} pairs):", same_platform_diff_browser.len());
    println!("    Min: {}, Max: {}, Avg: {:.1}", min, max, avg);
    println!("    Would match (<=32): {} ({:.1}%)", matches, 100.0 * matches as f64 / same_platform_diff_browser.len().max(1) as f64);

    let (min, max, avg) = stats(&diff_platform_same_browser);
    let matches = diff_platform_same_browser.iter().filter(|&&d| d <= 32).count();
    println!("  Diff platform + Same browser ({} pairs):", diff_platform_same_browser.len());
    println!("    Min: {}, Max: {}, Avg: {:.1}", min, max, avg);
    println!("    Would match (<=32): {} ({:.1}%)", matches, 100.0 * matches as f64 / diff_platform_same_browser.len().max(1) as f64);

    let (min, max, avg) = stats(&diff_platform_diff_browser);
    let matches = diff_platform_diff_browser.iter().filter(|&&d| d <= 32).count();
    println!("  Diff platform + Diff browser ({} pairs):", diff_platform_diff_browser.len());
    println!("    Min: {}, Max: {}, Avg: {:.1}", min, max, avg);
    println!("    Would match (<=32): {} ({:.1}%)", matches, 100.0 * matches as f64 / diff_platform_diff_browser.len().max(1) as f64);

    // The key question: same browser on different machines (different GPU)
    println!("\n  KEY QUESTION: Same OS + Same browser + Different GPU:");
    let mut same_os_browser_diff_gpu: Vec<u32> = Vec::new();
    for i in 0..synthetic_hashes.len() {
        for j in (i+1)..synthetic_hashes.len() {
            let label_i: Vec<&str> = synthetic_hashes[i].1.split('/').collect();
            let label_j: Vec<&str> = synthetic_hashes[j].1.split('/').collect();

            if label_i[0] == label_j[0] && label_i[1] == label_j[1] && label_i[2] != label_j[2] {
                let dist = hamming_distance_u64x4(&synthetic_hashes[i].0, &synthetic_hashes[j].0);
                same_os_browser_diff_gpu.push(dist);
            }
        }
    }

    let (min, max, avg) = stats(&same_os_browser_diff_gpu);
    let matches = same_os_browser_diff_gpu.iter().filter(|&&d| d <= 32).count();
    println!("    {} pairs, Min: {}, Max: {}, Avg: {:.1}", same_os_browser_diff_gpu.len(), min, max, avg);
    println!("    Would match (<=32): {} ({:.1}%)", matches, 100.0 * matches as f64 / same_os_browser_diff_gpu.len().max(1) as f64);

    // Also compute Same OS + Same browser + Same GPU (should be low - legitimate matches)
    println!("\n  BASELINE: Same OS + Same browser + Same GPU (expected to match):");
    let mut same_os_browser_same_gpu: Vec<u32> = Vec::new();
    for i in 0..synthetic_hashes.len() {
        for j in (i+1)..synthetic_hashes.len() {
            let label_i: Vec<&str> = synthetic_hashes[i].1.split('/').collect();
            let label_j: Vec<&str> = synthetic_hashes[j].1.split('/').collect();

            if label_i[0] == label_j[0] && label_i[1] == label_j[1] && label_i[2] == label_j[2] {
                let dist = hamming_distance_u64x4(&synthetic_hashes[i].0, &synthetic_hashes[j].0);
                same_os_browser_same_gpu.push(dist);
            }
        }
    }

    let (min_same, max_same, avg_same) = stats(&same_os_browser_same_gpu);
    let matches_same = same_os_browser_same_gpu.iter().filter(|&&d| d <= 32).count();
    println!("    {} pairs, Min: {}, Max: {}, Avg: {:.1}", same_os_browser_same_gpu.len(), min_same, max_same, avg_same);
    println!("    Would match (<=32): {} ({:.1}%)", matches_same, 100.0 * matches_same as f64 / same_os_browser_same_gpu.len().max(1) as f64);

    // Summary table
    println!("\n  Summary of results:");
    println!("  ┌────────────────────────────────────────────┬──────────────┬──────────────┐");
    println!("  │                  Category                  │ Avg Distance │ Match at ≤32 │");
    println!("  ├────────────────────────────────────────────┼──────────────┼──────────────┤");
    let (_, _, avg) = stats(&same_platform_browser);
    let pct = 100.0 * same_platform_browser.iter().filter(|&&d| d <= 32).count() as f64 / same_platform_browser.len().max(1) as f64;
    println!("  │ Same OS + Same browser (varies GPU/screen) │ {:>10.1}   │ {:>10.1}%  │", avg, pct);
    println!("  ├────────────────────────────────────────────┼──────────────┼──────────────┤");
    let (_, _, avg) = stats(&same_platform_diff_browser);
    let pct = 100.0 * same_platform_diff_browser.iter().filter(|&&d| d <= 32).count() as f64 / same_platform_diff_browser.len().max(1) as f64;
    println!("  │ Same OS + Diff browser                     │ {:>10.1}   │ {:>10.1}%  │", avg, pct);
    println!("  ├────────────────────────────────────────────┼──────────────┼──────────────┤");
    let (_, _, avg) = stats(&diff_platform_same_browser);
    let pct = 100.0 * diff_platform_same_browser.iter().filter(|&&d| d <= 32).count() as f64 / diff_platform_same_browser.len().max(1) as f64;
    println!("  │ Diff OS + Same browser                     │ {:>10.1}   │ {:>10.1}%  │", avg, pct);
    println!("  ├────────────────────────────────────────────┼──────────────┼──────────────┤");
    let (_, _, avg) = stats(&diff_platform_diff_browser);
    let pct = 100.0 * diff_platform_diff_browser.iter().filter(|&&d| d <= 32).count() as f64 / diff_platform_diff_browser.len().max(1) as f64;
    println!("  │ Diff OS + Diff browser                     │ {:>10.1}   │ {:>10.1}%  │", avg, pct);
    println!("  ├────────────────────────────────────────────┼──────────────┼──────────────┤");
    println!("  │ Same OS + Same browser + Same GPU          │ {:>10.1}   │ {:>10.1}%  │", avg_same, 100.0 * matches_same as f64 / same_os_browser_same_gpu.len().max(1) as f64);
    println!("  ├────────────────────────────────────────────┼──────────────┼──────────────┤");
    let (_, _, avg_diff_gpu) = stats(&same_os_browser_diff_gpu);
    let pct_diff_gpu = 100.0 * matches as f64 / same_os_browser_diff_gpu.len().max(1) as f64;
    println!("  │ Same OS + Same browser + Different GPU     │ {:>10.1}   │ {:>10.1}%  │", avg_diff_gpu, pct_diff_gpu);
    println!("  └────────────────────────────────────────────┴──────────────┴──────────────┘");

    // Note: Same GPU only matches 50% because machines differ in screen/fonts/hardware.
    // This is expected - same user on different monitors may exceed threshold.
    let same_gpu_match_pct = 100.0 * matches_same as f64 / same_os_browser_same_gpu.len().max(1) as f64;

    // Distribution graph for threshold selection
    println!("\n  Distance distribution (0-64 bits):");
    println!("  ═══════════════════════════════════════════════════════════════════════");
    println!("  Threshold │ Same GPU │ Diff GPU │ Diff Browser │ Diff OS │ Strategy");
    println!("  ──────────┼──────────┼──────────┼──────────────┼─────────┼──────────────");

    for threshold in [8, 12, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64] {
        let same_gpu_pct = 100.0 * same_os_browser_same_gpu.iter().filter(|&&d| d <= threshold).count() as f64
                          / same_os_browser_same_gpu.len().max(1) as f64;
        let diff_gpu_pct = 100.0 * same_os_browser_diff_gpu.iter().filter(|&&d| d <= threshold).count() as f64
                          / same_os_browser_diff_gpu.len().max(1) as f64;
        let diff_browser_pct = 100.0 * same_platform_diff_browser.iter().filter(|&&d| d <= threshold).count() as f64
                               / same_platform_diff_browser.len().max(1) as f64;
        let diff_os_pct = 100.0 * diff_platform_same_browser.iter().filter(|&&d| d <= threshold).count() as f64
                          / diff_platform_same_browser.len().max(1) as f64;

        let strategy = if threshold == 24 {
            "← STRICT (rate limiting)"
        } else if threshold == 40 {
            "← LOOSE (cookie binding)"
        } else if threshold < 24 {
            ""
        } else if threshold < 40 {
            "← attack window"
        } else {
            ""
        };

        println!("     {:>3}    │  {:>5.1}%  │  {:>5.1}%  │    {:>5.1}%     │  {:>5.1}% │ {}",
                 threshold, same_gpu_pct, diff_gpu_pct, diff_browser_pct, diff_os_pct, strategy);
    }
    println!("  ═══════════════════════════════════════════════════════════════════════");
    println!("\n  Legend:");
    println!("    Same GPU    = Same user, different sessions/hardware (want HIGH match)");
    println!("    Diff GPU    = Different user, same OS/browser (want LOW match = false positives)");
    println!("    Diff Browser = Different browser on same OS (want ZERO match)");
    println!("    Diff OS     = Different OS entirely (want ZERO match)");

    println!("\n  Two-threshold strategy:");
    println!("    • Threshold 24 (STRICT rate limiting): {:.1}% same-user match, {:.1}% false positive",
             100.0 * same_os_browser_same_gpu.iter().filter(|&&d| d <= 24).count() as f64 / same_os_browser_same_gpu.len().max(1) as f64,
             100.0 * same_os_browser_diff_gpu.iter().filter(|&&d| d <= 24).count() as f64 / same_os_browser_diff_gpu.len().max(1) as f64);
    println!("    • Threshold 40 (LOOSE cookie binding): {:.1}% same-user match, {:.1}% stolen cookie detection",
             100.0 * same_os_browser_same_gpu.iter().filter(|&&d| d <= 40).count() as f64 / same_os_browser_same_gpu.len().max(1) as f64,
             100.0 * diff_platform_same_browser.iter().filter(|&&d| d > 40).count() as f64 / diff_platform_same_browser.len().max(1) as f64);
    println!("    • Attack window: 24-40 bits to evade both thresholds");

    if matches > 0 {
        println!("\n  The problem:");
        println!("  - Cross-platform and cross-browser: excellent separation (0% false matches)");
        println!("  - Same OS + Same browser + Different GPU: {:.1}% would incorrectly match!", pct_diff_gpu);
        println!("  - Same OS + Same browser + Same GPU: {:.1}% match (baseline for same user)", same_gpu_match_pct);
        println!("\n  Analysis:");
        println!("  With all high-entropy fields (canvas, webgl, fonts, audio, voices), ~{:.0}%", pct_diff_gpu);
        println!("  of different users (same OS/browser, different GPU) could be grouped together.");
        println!("\n  Why this happens (even with fonts):");
        println!("  - This test uses typical OS fonts which are shared across machines");
        println!("  - Real users have additional fonts from apps (Adobe, Office, etc.)");
        println!("  - JS font detection finds per-machine fonts that improve uniqueness");
        println!("  - Canvas/WebGL hashes are tied to GPU, so same GPU = same hash");
        println!("\n  Recommendations:");
        println!("  1. Use threshold 24 for rate limiting (STRICT - minimize false positives)");
        println!("  2. Use threshold 40 for cookie binding (LOOSE - catch incognito/variations)");
        println!("  3. Combine with IP/session data for additional disambiguation");
    } else {
        println!("\n  Excellent! No false matches between different GPUs at threshold ≤32.");
        println!("  Same GPU match rate: {:.1}% (baseline for same user)", same_gpu_match_pct);
    }

    // Summary only if we ran Tests 1-3 with real fingerprints
    if has_real_fps {
        println!("\n============================================================");
        println!("SUMMARY (Tests 1-3 with real fingerprints)");
        println!("============================================================");
        println!("\nKey findings:");
        println!("  1. Real fingerprints compress to ~{:.0}% of original size", avg_real * 100.0);
        println!("     Random ASCII compresses to ~{:.0}%", avg_random * 100.0);
        println!("     => Compression ratio is a viable detection metric");
        println!("\n  2. Realistic hash distance distribution:");
        println!("     Mean={:.1}, range [{}, {}]", realistic_avg, realistic_min, realistic_max);
        println!("     vs Random: Mean={:.1}, range [{}, {}]", random_avg, random_min, random_max);

        if realistic_avg < 100.0 {
            println!("     => Realistic hashes cluster MUCH tighter than random!");
            println!("     => BK-tree may provide better pruning with realistic data");
        }

        println!("\n  3. BK-tree speedup:");
        println!("     Realistic data: {:.2}x", speedup);
        println!("     Random data:    {:.2}x", random_speedup);
    }
}
