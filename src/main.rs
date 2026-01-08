#![allow(dead_code)]
use rand::Rng;
use std::fs;
use plotters::prelude::*;

fn main() {
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

    // ==================== SUMMARY ====================
    println!("============================================================");
    println!("=== SUMMARY ===");
    println!("============================================================");
    println!("                    64-bit      256-bit");
    println!("Random Letters:     ~{:<6}     ~{:<6} chars", letter_limit_64, letter_limit_256);
    println!("Random Words:       ~{:<6}     ~{:<6} chars", word_limit_64, word_limit_256);
    if word_limit_64 > letter_limit_64 || word_limit_256 > letter_limit_256 {
        println!("\n=> Word changes show LOCALITY BENEFIT (more tolerant of concentrated changes)");
    } else if word_limit_64 < letter_limit_64 || word_limit_256 < letter_limit_256 {
        println!("\n=> Letter changes show better tolerance (no locality benefit)");
    } else {
        println!("\n=> No significant difference (nilsimsa treats both similarly)");
    }

    // Generate comparison plots
    generate_comparison_plot(
        &letter_chrome_64, &letter_firefox_64,
        &word_chrome_64, &word_firefox_64,
        baseline_64, "discrimination_64bit.png", 64
    ).expect("Failed to generate 64-bit plot");

    generate_comparison_plot(
        &letter_chrome_256, &letter_firefox_256,
        &word_chrome_256, &word_firefox_256,
        baseline_256, "discrimination_256bit.png", 256
    ).expect("Failed to generate 256-bit plot");

    println!("\nGenerated: discrimination_64bit.png, discrimination_256bit.png");
}

#[derive(Clone, Copy)]
enum ModType { Letter, Word }

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
    let type_str = match mod_type { ModType::Letter => "letter", ModType::Word => "word" };
    println!("  Analyzing {} ({})...", name, type_str);

    let mut results = Vec::with_capacity(mod_counts.len());

    for &target_chars in mod_counts {
        let mut distances: Vec<u32> = Vec::with_capacity(num_trials);

        for _ in 0..num_trials {
            let modified = match mod_type {
                ModType::Letter => randomly_modify_letters(input, target_chars),
                ModType::Word => randomly_modify_words(input, target_chars),
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

fn generate_comparison_plot(
    letter_chrome: &[Stats], letter_firefox: &[Stats],
    word_chrome: &[Stats], word_firefox: &[Stats],
    baseline: u32, filename: &str, bits: u32
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (1000, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_x = letter_chrome.last().unwrap().num_modifications as f64 * 1.1;

    // Calculate max Y from all data
    let all_combined: Vec<f64> = letter_chrome.iter().zip(letter_firefox.iter())
        .chain(word_chrome.iter().zip(word_firefox.iter()))
        .map(|(c, f)| (c.mean + 2.0 * c.stdev) + (f.mean + 2.0 * f.stdev))
        .collect();
    let max_y = all_combined.iter().cloned().fold(baseline as f64, f64::max) * 1.2;

    let mut chart = ChartBuilder::on(&root)
        .caption(format!("Letter vs Word Changes - Discrimination Threshold ({}-bit)", bits), ("sans-serif", 22))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d((1.0_f64).log2()..max_x.log2(), 0.0..max_y)?;

    chart.configure_mesh()
        .x_desc("Characters Modified (log scale)")
        .y_desc("Combined Noise: Chrome(mean+2σ) + Firefox(mean+2σ)")
        .x_label_formatter(&|x| format!("{}", (2.0_f64).powf(*x) as u32))
        .draw()?;

    // Letter changes combined noise (solid magenta)
    let letter_combined: Vec<_> = letter_chrome.iter().zip(letter_firefox.iter())
        .map(|(c, f)| ((c.num_modifications as f64).log2(),
                       (c.mean + 2.0 * c.stdev) + (f.mean + 2.0 * f.stdev)))
        .collect();
    chart.draw_series(LineSeries::new(letter_combined.iter().cloned(), MAGENTA.stroke_width(3)))?
        .label("Random Letters").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], MAGENTA.stroke_width(3)));

    // Word changes combined noise (solid cyan)
    let word_combined: Vec<_> = word_chrome.iter().zip(word_firefox.iter())
        .map(|(c, f)| ((c.num_modifications as f64).log2(),
                       (c.mean + 2.0 * c.stdev) + (f.mean + 2.0 * f.stdev)))
        .collect();
    chart.draw_series(LineSeries::new(word_combined.iter().cloned(), CYAN.stroke_width(3)))?
        .label("Random Words").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], CYAN.stroke_width(3)));

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
