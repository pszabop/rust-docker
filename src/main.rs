#![allow(dead_code)]
use rand::Rng;
use std::fs;
use plotters::prelude::*;

fn main() {
    // Read the contents of the JSON files as strings
    let chrome_long = fs::read_to_string("chrome_values.json")
        .expect("Unable to read file chrome_values.json");
    let firefox_long = fs::read_to_string("firefox_values.json")
        .expect("Unable to read file firefox_values.json");

    // Configuration
    const NUM_TRIALS: usize = 100;
    // Exponential sampling: 1, 2, 4, 8, 16, 32, 64, 128, 256
    let mod_counts: Vec<usize> = (0..=8).map(|i| 1 << i).collect();

    // ========== 64-bit analysis ==========
    println!("========================================");
    println!("=== 64-bit Nilsimsa Hash Analysis ===");
    println!("========================================\n");

    let chrome_hash_64 = stable_document_hash_64(&chrome_long);
    let firefox_hash_64 = stable_document_hash_64(&firefox_long);
    let baseline_64 = hamming_distance_64(chrome_hash_64, firefox_hash_64);
    println!("Baseline Chrome vs Firefox: {} / 64 bits ({:.1}%)\n", baseline_64, baseline_64 as f64 / 64.0 * 100.0);

    let chrome_stats_64 = run_analysis_64(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome");
    let firefox_stats_64 = run_analysis_64(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox");

    print_results_table(&chrome_stats_64, &firefox_stats_64, baseline_64);
    print_discrimination_analysis(&chrome_stats_64, &firefox_stats_64, baseline_64, 64);

    // ========== 256-bit analysis ==========
    println!("\n========================================");
    println!("=== 256-bit Nilsimsa Hash Analysis ===");
    println!("========================================\n");

    let chrome_hash_256 = stable_document_hash_256(&chrome_long);
    let firefox_hash_256 = stable_document_hash_256(&firefox_long);
    let baseline_256 = hamming_distance_256(&chrome_hash_256, &firefox_hash_256);
    println!("Baseline Chrome vs Firefox: {} / 256 bits ({:.1}%)\n", baseline_256, baseline_256 as f64 / 256.0 * 100.0);

    let chrome_stats_256 = run_analysis_256(&chrome_long, NUM_TRIALS, &mod_counts, "Chrome");
    let firefox_stats_256 = run_analysis_256(&firefox_long, NUM_TRIALS, &mod_counts, "Firefox");

    print_results_table(&chrome_stats_256, &firefox_stats_256, baseline_256);
    print_discrimination_analysis(&chrome_stats_256, &firefox_stats_256, baseline_256, 256);

    // Generate discrimination threshold plots
    generate_discrimination_plot(&chrome_stats_64, &firefox_stats_64, baseline_64,
                                 "discrimination_threshold_64bit.png", 64)
        .expect("Failed to generate 64-bit discrimination plot");

    generate_discrimination_plot(&chrome_stats_256, &firefox_stats_256, baseline_256,
                                 "discrimination_threshold_256bit.png", 256)
        .expect("Failed to generate 256-bit discrimination plot");

    println!("\nGenerated PNG files:");
    println!("  - discrimination_threshold_64bit.png");
    println!("  - discrimination_threshold_256bit.png");
}

#[derive(Clone)]
struct Stats {
    num_modifications: usize,
    mean: f64,
    stdev: f64,
    min: u32,
    max: u32,
}

fn run_analysis_64(input: &str, num_trials: usize, mod_counts: &[usize], name: &str) -> Vec<Stats> {
    println!("Analyzing {} (64-bit)...", name);
    let original_hash = stable_document_hash_64(input);
    let mut results = Vec::with_capacity(mod_counts.len());

    for &num_mods in mod_counts {
        let mut distances: Vec<u32> = Vec::with_capacity(num_trials);

        for _ in 0..num_trials {
            let modified = randomly_modify_string(input, num_mods);
            let modified_hash = stable_document_hash_64(&modified);
            let dist = hamming_distance_64(original_hash, modified_hash);
            distances.push(dist);
        }

        let mean = distances.iter().map(|&d| d as f64).sum::<f64>() / num_trials as f64;
        let variance = distances.iter().map(|&d| (d as f64 - mean).powi(2)).sum::<f64>() / num_trials as f64;
        let stdev = variance.sqrt();
        let min = *distances.iter().min().unwrap();
        let max = *distances.iter().max().unwrap();

        results.push(Stats { num_modifications: num_mods, mean, stdev, min, max });
        println!("  {} chars: mean={:.2}, stdev={:.2}, mean+2σ={:.2}", num_mods, mean, stdev, mean + 2.0 * stdev);
    }

    println!("  Done.\n");
    results
}

fn run_analysis_256(input: &str, num_trials: usize, mod_counts: &[usize], name: &str) -> Vec<Stats> {
    println!("Analyzing {} (256-bit)...", name);
    let original_hash = stable_document_hash_256(input);
    let mut results = Vec::with_capacity(mod_counts.len());

    for &num_mods in mod_counts {
        let mut distances: Vec<u32> = Vec::with_capacity(num_trials);

        for _ in 0..num_trials {
            let modified = randomly_modify_string(input, num_mods);
            let modified_hash = stable_document_hash_256(&modified);
            let dist = hamming_distance_256(&original_hash, &modified_hash);
            distances.push(dist);
        }

        let mean = distances.iter().map(|&d| d as f64).sum::<f64>() / num_trials as f64;
        let variance = distances.iter().map(|&d| (d as f64 - mean).powi(2)).sum::<f64>() / num_trials as f64;
        let stdev = variance.sqrt();
        let min = *distances.iter().min().unwrap();
        let max = *distances.iter().max().unwrap();

        results.push(Stats { num_modifications: num_mods, mean, stdev, min, max });
        println!("  {} chars: mean={:.2}, stdev={:.2}, mean+2σ={:.2}", num_mods, mean, stdev, mean + 2.0 * stdev);
    }

    println!("  Done.\n");
    results
}

fn print_results_table(chrome_stats: &[Stats], firefox_stats: &[Stats], baseline: u32) {
    println!("=== Results Table ===");
    println!("{:>6} | {:>8} {:>8} | {:>8} {:>8} | {:>10} {:>10} | {:>8}",
             "Chars", "C Mean", "C +2σ", "F Mean", "F +2σ", "Combined", "% of Base", "Baseline");
    println!("{}", "-".repeat(95));

    for (c, f) in chrome_stats.iter().zip(firefox_stats.iter()) {
        let c_upper = c.mean + 2.0 * c.stdev;
        let f_upper = f.mean + 2.0 * f.stdev;
        let combined = c_upper + f_upper;
        let combined_pct = combined / baseline as f64 * 100.0;
        println!("{:>6} | {:>8.2} {:>8.2} | {:>8.2} {:>8.2} | {:>10.2} {:>9.1}% | {:>8}",
                 c.num_modifications, c.mean, c_upper, f.mean, f_upper, combined, combined_pct, baseline);
    }
}

fn print_discrimination_analysis(chrome_stats: &[Stats], firefox_stats: &[Stats], baseline: u32, _bits: u32) {
    println!("\n=== Discrimination Analysis ===");
    println!("Baseline browser difference: {} bits", baseline);
    println!("Combined noise = Chrome(mean+2σ) + Firefox(mean+2σ)");
    println!("When combined noise >= baseline, can't distinguish browsers.\n");

    // Find where combined (chrome + firefox mean+2σ) crosses baseline
    let mut cross: Option<(usize, usize)> = None;

    for i in 0..chrome_stats.len() {
        let c = &chrome_stats[i];
        let f = &firefox_stats[i];
        let c_upper = c.mean + 2.0 * c.stdev;
        let f_upper = f.mean + 2.0 * f.stdev;
        let combined = c_upper + f_upper;

        if cross.is_none() && combined >= baseline as f64 {
            let prev_mods = if i > 0 { chrome_stats[i-1].num_modifications } else { 0 };
            cross = Some((prev_mods, c.num_modifications));
        }
    }

    match cross {
        Some((lo, hi)) => {
            println!("Discrimination lost between {} and {} char modifications", lo, hi);
            println!("Safe modification limit: ~{} chars", lo);
        }
        None => println!("Safe modification limit: >{} chars", chrome_stats.last().unwrap().num_modifications),
    }
}

/// Plot showing when combined modification noise crosses the baseline browser difference
fn generate_discrimination_plot(chrome: &[Stats], firefox: &[Stats], baseline: u32,
                                 filename: &str, bits: u32) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_x = chrome.last().unwrap().num_modifications as f64 * 1.1;
    // Combined noise can be higher than baseline
    let max_y = chrome.iter().zip(firefox.iter())
        .map(|(c, f)| (c.mean + 2.0 * c.stdev) + (f.mean + 2.0 * f.stdev))
        .fold(baseline as f64, f64::max) * 1.2;

    let mut chart = ChartBuilder::on(&root)
        .caption(format!("Browser Discrimination Threshold ({}-bit)", bits), ("sans-serif", 24))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d((1.0_f64).log2()..max_x.log2(), 0.0..max_y)?;

    chart.configure_mesh()
        .x_desc("Character Modifications (log scale)")
        .y_desc("Hamming Distance (bits)")
        .x_label_formatter(&|x| format!("{}", (2.0_f64).powf(*x) as u32))
        .draw()?;

    // Combined noise line (Chrome + Firefox mean+2σ) - this is the key line
    let combined: Vec<_> = chrome.iter().zip(firefox.iter())
        .map(|(c, f)| {
            let c_upper = c.mean + 2.0 * c.stdev;
            let f_upper = f.mean + 2.0 * f.stdev;
            ((c.num_modifications as f64).log2(), c_upper + f_upper)
        })
        .collect();
    chart.draw_series(LineSeries::new(
        combined.iter().cloned(),
        MAGENTA.stroke_width(3),
    ))?.label("Combined noise (C+F mean+2σ)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &MAGENTA));

    // Baseline threshold line
    chart.draw_series(LineSeries::new(
        [(1.0_f64.log2(), baseline as f64), (max_x.log2(), baseline as f64)],
        GREEN.stroke_width(3),
    ))?.label(format!("Baseline ({} bits)", baseline)).legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &GREEN));

    // Individual browser lines (lighter, for reference)
    chart.draw_series(LineSeries::new(
        chrome.iter().map(|s| ((s.num_modifications as f64).log2(), s.mean + 2.0 * s.stdev)),
        BLUE.stroke_width(1),
    ))?.label("Chrome (mean + 2σ)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    chart.draw_series(LineSeries::new(
        firefox.iter().map(|s| ((s.num_modifications as f64).log2(), s.mean + 2.0 * s.stdev)),
        RED.stroke_width(1),
    ))?.label("Firefox (mean + 2σ)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart.configure_series_labels()
        .position(SeriesLabelPosition::UpperLeft)
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    Ok(())
}

fn stable_document_hash_64(input: &str) -> u64 {
    let mut hasher = nilsimsa::Nilsimsa::new();
    hasher.update(input);
    let hex_str = hasher.digest();

    let truncated_hex = &hex_str[0..16]; // 16 hex digits = 64 bits
    u64::from_str_radix(truncated_hex, 16).expect("Invalid hex from Nilsimsa")
}

fn stable_document_hash_256(input: &str) -> [u8; 32] {
    let mut hasher = nilsimsa::Nilsimsa::new();
    hasher.update(input);
    let hex_str = hasher.digest(); // 64 hex chars = 256 bits

    let mut result = [0u8; 32];
    for i in 0..32 {
        result[i] = u8::from_str_radix(&hex_str[i * 2..i * 2 + 2], 16)
            .expect("Invalid hex from Nilsimsa");
    }
    result
}

fn hex_encode(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Compute Hamming distance for 64-bit values
pub fn hamming_distance_64(first: u64, second: u64) -> u32 {
    (first ^ second).count_ones()
}

/// Compute Hamming distance for 256-bit values (as [u8; 32])
pub fn hamming_distance_256(first: &[u8; 32], second: &[u8; 32]) -> u32 {
    first.iter().zip(second.iter())
        .map(|(a, b)| (a ^ b).count_ones())
        .sum()
}

fn randomly_modify_string(input: &str, n: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    for _ in 0..n {
        let mut idx = rng.gen_range(0..len);
        while chars[idx] == ',' {
            idx = rng.gen_range(0..len);
        }
        let new_char = rng.gen_range(b'a'..=b'z') as char;
        chars[idx] = new_char;
    }

    chars.into_iter().collect()
}
