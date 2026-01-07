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

    // Show baseline comparison
    println!("=== Baseline: Chrome vs Firefox ===");
    let chrome_hash = stable_document_hash(&chrome_long);
    let firefox_hash = stable_document_hash(&firefox_long);
    let baseline_distance = hamming_distance(chrome_hash, firefox_hash);
    println!("Chrome hash:  {:016x}", chrome_hash);
    println!("Firefox hash: {:016x}", firefox_hash);
    println!("Hamming distance between Chrome and Firefox: {}\n", baseline_distance);

    // Configuration
    const NUM_TRIALS: usize = 100;
    const MAX_MODIFICATIONS: usize = 200;

    println!("=== Running Statistical Analysis ===");
    println!("Trials per modification count: {}", NUM_TRIALS);
    println!("Max modifications: {}\n", MAX_MODIFICATIONS);

    // Run analysis for both signatures
    let chrome_stats = run_analysis(&chrome_long, NUM_TRIALS, MAX_MODIFICATIONS, "Chrome");
    let firefox_stats = run_analysis(&firefox_long, NUM_TRIALS, MAX_MODIFICATIONS, "Firefox");

    // Print results table
    print_results_table(&chrome_stats, &firefox_stats);

    // Generate PNG graphs
    generate_mean_plot(&chrome_stats, &firefox_stats, "hamming_distance_mean.png")
        .expect("Failed to generate mean plot");
    generate_stdev_plot(&chrome_stats, &firefox_stats, "hamming_distance_stdev.png")
        .expect("Failed to generate stdev plot");
    generate_combined_plot(&chrome_stats, &firefox_stats, "hamming_distance_combined.png")
        .expect("Failed to generate combined plot");

    println!("\nGenerated PNG files:");
    println!("  - hamming_distance_mean.png");
    println!("  - hamming_distance_stdev.png");
    println!("  - hamming_distance_combined.png");
}

#[derive(Clone)]
struct Stats {
    num_modifications: usize,
    mean: f64,
    stdev: f64,
    min: u32,
    max: u32,
}

fn run_analysis(input: &str, num_trials: usize, max_mods: usize, name: &str) -> Vec<Stats> {
    println!("Analyzing {} signature...", name);
    let original_hash = stable_document_hash(input);
    let mut results = Vec::with_capacity(max_mods);

    for num_mods in 1..=max_mods {
        let mut distances: Vec<u32> = Vec::with_capacity(num_trials);

        for _ in 0..num_trials {
            let modified = randomly_modify_string(input, num_mods);
            let modified_hash = stable_document_hash(&modified);
            let dist = hamming_distance(original_hash, modified_hash);
            distances.push(dist);
        }

        let mean = distances.iter().map(|&d| d as f64).sum::<f64>() / num_trials as f64;
        let variance = distances.iter().map(|&d| (d as f64 - mean).powi(2)).sum::<f64>() / num_trials as f64;
        let stdev = variance.sqrt();
        let min = *distances.iter().min().unwrap();
        let max = *distances.iter().max().unwrap();

        results.push(Stats { num_modifications: num_mods, mean, stdev, min, max });

        // Progress indicator every 20 modifications
        if num_mods % 20 == 0 {
            println!("  {} modifications: mean={:.2}, stdev={:.2}", num_mods, mean, stdev);
        }
    }

    println!("  Done.\n");
    results
}

fn print_results_table(chrome_stats: &[Stats], firefox_stats: &[Stats]) {
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

    // Find threshold crossings (when mean hamming distance crosses 1, 2, 3, ... 16)
    println!("\n=== Hamming Distance Threshold Crossings ===");
    println!("{:>10} | {:>15} | {:>15}", "Threshold", "Chrome (mods)", "Firefox (mods)");
    println!("{}", "-".repeat(50));

    for threshold in 1..=16 {
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

fn generate_mean_plot(chrome: &[Stats], firefox: &[Stats], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_x = chrome.len() as f64;
    let max_y = chrome.iter().chain(firefox.iter()).map(|s| s.mean).fold(0.0_f64, f64::max) * 1.1;

    let mut chart = ChartBuilder::on(&root)
        .caption("Mean Hamming Distance vs Character Modifications", ("sans-serif", 24))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0.0..max_x, 0.0..max_y)?;

    chart.configure_mesh()
        .x_desc("Number of Random Character Modifications")
        .y_desc("Mean Hamming Distance")
        .draw()?;

    // Chrome line
    chart.draw_series(LineSeries::new(
        chrome.iter().map(|s| (s.num_modifications as f64, s.mean)),
        &BLUE,
    ))?.label("Chrome").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    // Firefox line
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

fn generate_stdev_plot(chrome: &[Stats], firefox: &[Stats], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_x = chrome.len() as f64;
    let max_y = chrome.iter().chain(firefox.iter()).map(|s| s.stdev).fold(0.0_f64, f64::max) * 1.1;

    let mut chart = ChartBuilder::on(&root)
        .caption("Std Dev of Hamming Distance vs Character Modifications", ("sans-serif", 24))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0.0..max_x, 0.0..max_y)?;

    chart.configure_mesh()
        .x_desc("Number of Random Character Modifications")
        .y_desc("Standard Deviation")
        .draw()?;

    chart.draw_series(LineSeries::new(
        chrome.iter().map(|s| (s.num_modifications as f64, s.stdev)),
        &BLUE,
    ))?.label("Chrome").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    chart.draw_series(LineSeries::new(
        firefox.iter().map(|s| (s.num_modifications as f64, s.stdev)),
        &RED,
    ))?.label("Firefox").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    Ok(())
}

fn generate_combined_plot(chrome: &[Stats], firefox: &[Stats], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_x = chrome.len() as f64;
    let max_y = chrome.iter().chain(firefox.iter()).map(|s| s.mean + 2.0 * s.stdev).fold(0.0_f64, f64::max) * 1.1;

    let mut chart = ChartBuilder::on(&root)
        .caption("Hamming Distance vs Character Modifications (mean ± 2σ)", ("sans-serif", 24))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0.0..max_x, 0.0..max_y)?;

    chart.configure_mesh()
        .x_desc("Number of Random Character Modifications")
        .y_desc("Hamming Distance")
        .draw()?;

    // Chrome error band (draw bands first, then lines on top) - ±2σ, floor at 0
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

    // Horizontal lines for hamming thresholds
    for threshold in [8, 16, 24, 32].iter() {
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

fn stable_document_hash(input: &str) -> u64 {
    let mut hasher = nilsimsa::Nilsimsa::new();
    hasher.update(input);
    let hex_str = hasher.digest();

    let truncated_hex = &hex_str[0..16]; // 16 hex digits = 64 bits
    u64::from_str_radix(truncated_hex, 16).expect("Invalid hex from Nilsimsa")
}

/// Compute Hamming distance (number of differing bits)
pub fn hamming_distance(first: u64, second: u64) -> u32 {
    (first ^ second).count_ones()
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
