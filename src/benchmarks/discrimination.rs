//! Discrimination analysis - analyzing how much modification before browsers become indistinguishable

use std::fs;
use plotters::prelude::*;

use crate::hash::{stable_document_hash_64, stable_document_hash_256, hamming_distance_64, hamming_distance_256};
use crate::modification::{ModType, randomly_modify_letters, randomly_modify_words, randomly_insert_chars, randomly_remove_chars, find_csv_fields};
use super::common::{HashSize, Stats};

pub fn run() {
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
