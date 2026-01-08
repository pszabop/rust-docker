//! Bit uniformity analysis for hash segmentation

use std::fs;
use plotters::prelude::*;

use crate::hash::stable_document_hash_256;
use crate::modification::{ModType, randomly_modify_letters, randomly_modify_words, randomly_insert_chars, randomly_remove_chars};

#[derive(Clone)]
pub struct BitUniformityStats {
    pub bit_flip_counts: [u32; 256],
    pub trials: u32,
    pub mean: f64,
    pub std_dev: f64,
    pub min: u32,
    pub max: u32,
    pub coeff_of_variation: f64,
}

pub fn analyze_bit_uniformity(
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

pub fn print_uniformity_results(stats: &BitUniformityStats, label: &str) {
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

pub fn generate_bit_uniformity_plot(
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

pub fn run() {
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
