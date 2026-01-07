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
    const MAX_MODIFICATIONS: usize = 200;

    // ========== 64-bit analysis ==========
    println!("========================================");
    println!("=== 64-bit Nilsimsa Hash Analysis ===");
    println!("========================================\n");

    println!("=== Baseline: Chrome vs Firefox (64-bit) ===");
    let chrome_hash_64 = stable_document_hash_64(&chrome_long);
    let firefox_hash_64 = stable_document_hash_64(&firefox_long);
    let baseline_64 = hamming_distance_64(chrome_hash_64, firefox_hash_64);
    println!("Chrome hash:  {:016x}", chrome_hash_64);
    println!("Firefox hash: {:016x}", firefox_hash_64);
    println!("Hamming distance: {} / 64 bits ({:.1}%)\n", baseline_64, baseline_64 as f64 / 64.0 * 100.0);

    println!("=== Running 64-bit Statistical Analysis ===");
    println!("Trials per modification count: {}", NUM_TRIALS);
    println!("Max modifications: {}\n", MAX_MODIFICATIONS);

    let chrome_stats_64 = run_analysis_64(&chrome_long, NUM_TRIALS, MAX_MODIFICATIONS, "Chrome");
    let firefox_stats_64 = run_analysis_64(&firefox_long, NUM_TRIALS, MAX_MODIFICATIONS, "Firefox");

    print_results_table(&chrome_stats_64, &firefox_stats_64, 64);

    // ========== 256-bit analysis ==========
    println!("\n========================================");
    println!("=== 256-bit Nilsimsa Hash Analysis ===");
    println!("========================================\n");

    println!("=== Baseline: Chrome vs Firefox (256-bit) ===");
    let chrome_hash_256 = stable_document_hash_256(&chrome_long);
    let firefox_hash_256 = stable_document_hash_256(&firefox_long);
    let baseline_256 = hamming_distance_256(&chrome_hash_256, &firefox_hash_256);
    println!("Chrome hash:  {}", hex_encode(&chrome_hash_256));
    println!("Firefox hash: {}", hex_encode(&firefox_hash_256));
    println!("Hamming distance: {} / 256 bits ({:.1}%)\n", baseline_256, baseline_256 as f64 / 256.0 * 100.0);

    println!("=== Running 256-bit Statistical Analysis ===");
    println!("Trials per modification count: {}", NUM_TRIALS);
    println!("Max modifications: {}\n", MAX_MODIFICATIONS);

    let chrome_stats_256 = run_analysis_256(&chrome_long, NUM_TRIALS, MAX_MODIFICATIONS, "Chrome");
    let firefox_stats_256 = run_analysis_256(&firefox_long, NUM_TRIALS, MAX_MODIFICATIONS, "Firefox");

    print_results_table(&chrome_stats_256, &firefox_stats_256, 256);

    // Generate PNG graphs for both
    generate_mean_plot(&chrome_stats_64, &firefox_stats_64, "hamming_distance_mean_64bit.png", "64-bit")
        .expect("Failed to generate 64-bit mean plot");
    generate_combined_plot(&chrome_stats_64, &firefox_stats_64, "hamming_distance_combined_64bit.png", "64-bit", 64)
        .expect("Failed to generate 64-bit combined plot");

    generate_mean_plot(&chrome_stats_256, &firefox_stats_256, "hamming_distance_mean_256bit.png", "256-bit")
        .expect("Failed to generate 256-bit mean plot");
    generate_combined_plot(&chrome_stats_256, &firefox_stats_256, "hamming_distance_combined_256bit.png", "256-bit", 256)
        .expect("Failed to generate 256-bit combined plot");

    // Comparison plot
    generate_comparison_plot(&chrome_stats_64, &chrome_stats_256, &firefox_stats_64, &firefox_stats_256,
                             "hamming_distance_comparison.png")
        .expect("Failed to generate comparison plot");

    println!("\nGenerated PNG files:");
    println!("  - hamming_distance_mean_64bit.png");
    println!("  - hamming_distance_combined_64bit.png");
    println!("  - hamming_distance_mean_256bit.png");
    println!("  - hamming_distance_combined_256bit.png");
    println!("  - hamming_distance_comparison.png (normalized comparison)");
}

#[derive(Clone)]
struct Stats {
    num_modifications: usize,
    mean: f64,
    stdev: f64,
    min: u32,
    max: u32,
}

fn run_analysis_64(input: &str, num_trials: usize, max_mods: usize, name: &str) -> Vec<Stats> {
    println!("Analyzing {} signature (64-bit)...", name);
    let original_hash = stable_document_hash_64(input);
    let mut results = Vec::with_capacity(max_mods);

    for num_mods in 1..=max_mods {
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

        if num_mods % 20 == 0 {
            println!("  {} modifications: mean={:.2}, stdev={:.2}", num_mods, mean, stdev);
        }
    }

    println!("  Done.\n");
    results
}

fn run_analysis_256(input: &str, num_trials: usize, max_mods: usize, name: &str) -> Vec<Stats> {
    println!("Analyzing {} signature (256-bit)...", name);
    let original_hash = stable_document_hash_256(input);
    let mut results = Vec::with_capacity(max_mods);

    for num_mods in 1..=max_mods {
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

        if num_mods % 20 == 0 {
            println!("  {} modifications: mean={:.2}, stdev={:.2}", num_mods, mean, stdev);
        }
    }

    println!("  Done.\n");
    results
}

fn print_results_table(chrome_stats: &[Stats], firefox_stats: &[Stats], bits: u32) {
    println!("=== Results Summary (selected points) ===");
    println!("{:>8} | {:>12} {:>8} | {:>12} {:>8} | {:>8}",
             "Mods", "Chrome Mean", "StdDev", "Firefox Mean", "StdDev", "Diff");
    println!("{}", "-".repeat(75));

    // Print at key points: 1, 5, 10, 20, 40, 60, 80, 100, 150, 200
    let key_points = [1, 5, 10, 20, 40, 60, 80, 100, 150, 200];

    for &n in &key_points {
        if n <= chrome_stats.len() {
            let c = &chrome_stats[n - 1];
            let f = &firefox_stats[n - 1];
            let diff = (c.mean - f.mean).abs();
            println!("{:>8} | {:>12.2} {:>8.2} | {:>12.2} {:>8.2} | {:>8.2}",
                     n, c.mean, c.stdev, f.mean, f.stdev, diff);
        }
    }

    // Find threshold crossings - scale thresholds based on bit width
    let max_threshold = if bits == 256 { 64 } else { 16 };
    let step = if bits == 256 { 4 } else { 1 };

    println!("\n=== Hamming Distance Threshold Crossings ===");
    println!("{:>10} | {:>15} | {:>15}", "Threshold", "Chrome (mods)", "Firefox (mods)");
    println!("{}", "-".repeat(50));

    for threshold in (step..=max_threshold).step_by(step as usize) {
        let chrome_cross = find_threshold_crossing(chrome_stats, threshold as f64);
        let firefox_cross = find_threshold_crossing(firefox_stats, threshold as f64);

        let chrome_str = chrome_cross.map_or("N/A".to_string(), |v| v.to_string());
        let firefox_str = firefox_cross.map_or("N/A".to_string(), |v| v.to_string());

        println!("{:>10} | {:>15} | {:>15}", threshold, chrome_str, firefox_str);
    }
}

fn find_threshold_crossing(stats: &[Stats], threshold: f64) -> Option<usize> {
    for s in stats {
        if s.mean >= threshold {
            return Some(s.num_modifications);
        }
    }
    None
}

fn generate_mean_plot(chrome: &[Stats], firefox: &[Stats], filename: &str, label: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_x = chrome.len() as f64;
    let max_y = chrome.iter().chain(firefox.iter()).map(|s| s.mean).fold(0.0_f64, f64::max) * 1.1;

    let mut chart = ChartBuilder::on(&root)
        .caption(format!("Mean Hamming Distance vs Char Mods ({})", label), ("sans-serif", 24))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0.0..max_x, 0.0..max_y)?;

    chart.configure_mesh()
        .x_desc("Number of Random Character Modifications")
        .y_desc("Mean Hamming Distance")
        .draw()?;

    chart.draw_series(LineSeries::new(
        chrome.iter().map(|s| (s.num_modifications as f64, s.mean)),
        &BLUE,
    ))?.label("Chrome").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    chart.draw_series(LineSeries::new(
        firefox.iter().map(|s| (s.num_modifications as f64, s.mean)),
        &RED,
    ))?.label("Firefox").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    Ok(())
}

fn generate_combined_plot(chrome: &[Stats], firefox: &[Stats], filename: &str, label: &str, bits: u32) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_x = chrome.len() as f64;
    let max_y = chrome.iter().chain(firefox.iter()).map(|s| s.mean + 2.0 * s.stdev).fold(0.0_f64, f64::max) * 1.1;

    let mut chart = ChartBuilder::on(&root)
        .caption(format!("Hamming Distance vs Char Mods ({}, mean ± 2σ)", label), ("sans-serif", 24))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0.0..max_x, 0.0..max_y)?;

    chart.configure_mesh()
        .x_desc("Number of Random Character Modifications")
        .y_desc("Hamming Distance")
        .draw()?;

    // Chrome error band - ±2σ, floor at 0
    let chrome_band: Vec<_> = chrome.iter()
        .map(|s| (s.num_modifications as f64, (s.mean - 2.0 * s.stdev).max(0.0), s.mean + 2.0 * s.stdev))
        .collect();
    chart.draw_series(chrome_band.iter().map(|(x, lo, hi)| {
        Rectangle::new([(*x - 0.5, *lo), (*x + 0.5, *hi)], BLUE.mix(0.2).filled())
    }))?;

    // Firefox error band - ±2σ, floor at 0
    let firefox_band: Vec<_> = firefox.iter()
        .map(|s| (s.num_modifications as f64, (s.mean - 2.0 * s.stdev).max(0.0), s.mean + 2.0 * s.stdev))
        .collect();
    chart.draw_series(firefox_band.iter().map(|(x, lo, hi)| {
        Rectangle::new([(*x - 0.5, *lo), (*x + 0.5, *hi)], RED.mix(0.2).filled())
    }))?;

    // Chrome mean line
    chart.draw_series(LineSeries::new(
        chrome.iter().map(|s| (s.num_modifications as f64, s.mean)),
        BLUE.stroke_width(2),
    ))?.label("Chrome (mean ± 2σ)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    // Firefox mean line
    chart.draw_series(LineSeries::new(
        firefox.iter().map(|s| (s.num_modifications as f64, s.mean)),
        RED.stroke_width(2),
    ))?.label("Firefox (mean ± 2σ)").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    // Horizontal lines for hamming thresholds - scale based on bit width
    let thresholds: Vec<u32> = if bits == 256 { vec![32, 64, 96, 128] } else { vec![8, 16, 24, 32] };
    for threshold in thresholds.iter() {
        if (*threshold as f64) < max_y {
            chart.draw_series(LineSeries::new(
                [(0.0, *threshold as f64), (max_x, *threshold as f64)],
                BLACK.stroke_width(1),
            ))?;
        }
    }

    chart.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    Ok(())
}

/// Comparison plot showing normalized (percentage) hamming distance for both bit widths
fn generate_comparison_plot(chrome_64: &[Stats], chrome_256: &[Stats],
                            firefox_64: &[Stats], firefox_256: &[Stats],
                            filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_x = chrome_64.len() as f64;

    let mut chart = ChartBuilder::on(&root)
        .caption("Normalized Hamming Distance (% of max) - 64-bit vs 256-bit", ("sans-serif", 22))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0.0..max_x, 0.0..25.0)?;  // 0-25% range

    chart.configure_mesh()
        .x_desc("Number of Random Character Modifications")
        .y_desc("Hamming Distance (% of max bits)")
        .draw()?;

    // Chrome 64-bit (solid blue)
    chart.draw_series(LineSeries::new(
        chrome_64.iter().map(|s| (s.num_modifications as f64, s.mean / 64.0 * 100.0)),
        BLUE.stroke_width(2),
    ))?.label("Chrome 64-bit").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE.stroke_width(2)));

    // Chrome 256-bit (dashed blue)
    chart.draw_series(LineSeries::new(
        chrome_256.iter().map(|s| (s.num_modifications as f64, s.mean / 256.0 * 100.0)),
        BLUE.stroke_width(2),
    ).point_size(2))?.label("Chrome 256-bit").legend(|(x, y)| {
        Rectangle::new([(x, y - 2), (x + 20, y + 2)], BLUE.mix(0.5).filled())
    });

    // Firefox 64-bit (solid red)
    chart.draw_series(LineSeries::new(
        firefox_64.iter().map(|s| (s.num_modifications as f64, s.mean / 64.0 * 100.0)),
        RED.stroke_width(2),
    ))?.label("Firefox 64-bit").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.stroke_width(2)));

    // Firefox 256-bit (dashed red)
    chart.draw_series(LineSeries::new(
        firefox_256.iter().map(|s| (s.num_modifications as f64, s.mean / 256.0 * 100.0)),
        RED.stroke_width(2),
    ).point_size(2))?.label("Firefox 256-bit").legend(|(x, y)| {
        Rectangle::new([(x, y - 2), (x + 20, y + 2)], RED.mix(0.5).filled())
    });

    chart.configure_series_labels()
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
