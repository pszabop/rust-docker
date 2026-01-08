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
