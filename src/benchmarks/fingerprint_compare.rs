//! Fingerprint comparison - load fingerprints and visualize hamming distances

use std::fs;
use std::path::Path;
use plotters::prelude::*;
use serde::Deserialize;

use crate::hash::hamming_distance_u64x4;

#[derive(Deserialize)]
struct BrowserInfo {
    #[serde(rename = "userAgent")]
    user_agent: String,
    #[allow(dead_code)]
    platform: Option<String>,
}

#[derive(Deserialize)]
struct Fingerprint {
    browser_info: BrowserInfo,
    document_hash256: String,
}

struct LoadedFingerprint {
    label: String,
    filename: String,
    hash: [u64; 4],
}

/// Extract a short label from user agent
fn extract_label(user_agent: &str, filename: &str) -> String {
    // Extract browser and version
    let browser = if user_agent.contains("Firefox/") {
        let idx = user_agent.find("Firefox/").unwrap();
        let version_start = idx + 8;
        let version_end = user_agent[version_start..]
            .find(|c: char| !c.is_numeric() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(user_agent.len());
        let version = &user_agent[version_start..version_end];
        let major = version.split('.').next().unwrap_or(version);
        format!("FF{}", major)
    } else if user_agent.contains("Chrome/") {
        let idx = user_agent.find("Chrome/").unwrap();
        let version_start = idx + 7;
        let version_end = user_agent[version_start..]
            .find(|c: char| !c.is_numeric() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(user_agent.len());
        let version = &user_agent[version_start..version_end];
        let major = version.split('.').next().unwrap_or(version);
        format!("Chr{}", major)
    } else if user_agent.contains("Safari/") && !user_agent.contains("Chrome") {
        "Safari".to_string()
    } else {
        "Unknown".to_string()
    };

    // Extract unique ID from filename (the 5-char code like "fwnzn")
    let id = filename
        .strip_prefix("antibot-")
        .and_then(|s| s.split('_').next())
        .unwrap_or(&filename[..5.min(filename.len())]);

    format!("{}:{}", browser, id)
}

/// Parse hex hash string to [u64; 4]
fn parse_hash(hex: &str) -> Result<[u64; 4], String> {
    if hex.len() != 64 {
        return Err(format!("Expected 64 hex chars, got {}", hex.len()));
    }

    let mut result = [0u64; 4];
    for i in 0..4 {
        let start = i * 16;
        let end = start + 16;
        result[i] = u64::from_str_radix(&hex[start..end], 16)
            .map_err(|e| format!("Invalid hex at {}-{}: {}", start, end, e))?;
    }
    Ok(result)
}

/// Load all fingerprints from directory
fn load_fingerprints(dir: &str) -> Vec<LoadedFingerprint> {
    let mut fingerprints = Vec::new();

    let path = Path::new(dir);
    if !path.exists() {
        return fingerprints;
    }

    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return fingerprints,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(fp) = serde_json::from_str::<Fingerprint>(&content) {
                    if let Ok(hash) = parse_hash(&fp.document_hash256) {
                        let label = extract_label(&fp.browser_info.user_agent, &filename);
                        fingerprints.push(LoadedFingerprint {
                            label,
                            filename,
                            hash,
                        });
                    }
                }
            }
        }
    }

    // Sort by label for consistent ordering
    fingerprints.sort_by(|a, b| a.label.cmp(&b.label));
    fingerprints
}

/// Compute distance matrix
fn compute_distances(fingerprints: &[LoadedFingerprint]) -> Vec<Vec<u32>> {
    let n = fingerprints.len();
    let mut distances = vec![vec![0u32; n]; n];
    for i in 0..n {
        for j in 0..n {
            distances[i][j] = hamming_distance_u64x4(&fingerprints[i].hash, &fingerprints[j].hash);
        }
    }
    distances
}

/// Print distance matrix to console
fn print_matrix(fingerprints: &[LoadedFingerprint], distances: &[Vec<u32>], title: &str) {
    let n = fingerprints.len();

    println!("\n{}", title);
    println!("{}", "=".repeat(title.len()));

    // Header
    print!("{:>15}", "");
    for fp in fingerprints {
        print!("{:>10}", &fp.label[..fp.label.len().min(9)]);
    }
    println!();

    // Rows
    for (i, fp) in fingerprints.iter().enumerate() {
        print!("{:>15}", fp.label);
        for j in 0..n {
            let dist = distances[i][j];
            if i == j {
                print!("{:>10}", "-");
            } else if dist <= 32 {
                print!("{:>10}", format!("*{}*", dist));
            } else {
                print!("{:>10}", dist);
            }
        }
        println!();
    }
}

/// Check if results match expectations
fn validate_results(distances: &[Vec<u32>], expect_same: bool) -> (usize, usize, bool) {
    let n = distances.len();
    let mut pass = 0;
    let mut fail = 0;

    for i in 0..n {
        for j in (i+1)..n {
            let dist = distances[i][j];
            if expect_same {
                // Should be ≤32
                if dist <= 32 {
                    pass += 1;
                } else {
                    fail += 1;
                }
            } else {
                // Should be >32
                if dist > 32 {
                    pass += 1;
                } else {
                    fail += 1;
                }
            }
        }
    }

    let ok = fail == 0;
    (pass, fail, ok)
}

/// Generate a distance matrix heatmap
fn generate_heatmap(
    fingerprints: &[LoadedFingerprint],
    distances: &[Vec<u32>],
    filename: &str,
    title: &str,
    expect_same: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let n = fingerprints.len();
    if n == 0 {
        return Ok(());
    }

    let cell_size = 80;
    let label_margin = 120;
    let width = (n * cell_size + label_margin + 50) as u32;
    let height = (n * cell_size + label_margin + 100) as u32;

    let root = BitMapBackend::new(filename, (width, height)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_dist = distances.iter()
        .flat_map(|row| row.iter())
        .filter(|&&d| d > 0)
        .max()
        .copied()
        .unwrap_or(128) as f64;

    // Title
    root.draw(&Text::new(
        title.to_string(),
        (width as i32 / 2 - (title.len() as i32 * 5), 15),
        ("sans-serif", 18).into_font(),
    ))?;

    // Draw cells
    for i in 0..n {
        for j in 0..n {
            let x = label_margin + j * cell_size;
            let y = 50 + i * cell_size;
            let dist = distances[i][j];

            // Color based on distance and expectations
            // Green = PASS, Red = FAIL
            let color = if i == j {
                RGBColor(200, 200, 200) // Diagonal - gray
            } else if dist <= 32 {
                // Similar fingerprints
                if expect_same {
                    RGBColor(50, 180, 50) // PASS - expected to be same
                } else {
                    RGBColor(220, 50, 50) // FAIL - should be different!
                }
            } else if dist <= 64 {
                // Moderately different
                if expect_same {
                    RGBColor(255, 180, 50) // Yellow warning - getting too different
                } else {
                    RGBColor(50, 180, 50) // PASS - different as expected
                }
            } else {
                // Very different
                if expect_same {
                    RGBColor(220, 50, 50) // FAIL - way too different
                } else {
                    RGBColor(30, 140, 30) // PASS - very different, dark green
                }
            };

            root.draw(&Rectangle::new(
                [(x as i32, y as i32), ((x + cell_size - 2) as i32, (y + cell_size - 2) as i32)],
                color.filled(),
            ))?;

            // Draw distance value
            if i != j {
                let text_color = if dist <= 32 && expect_same { &BLACK } else { &WHITE };
                root.draw(&Text::new(
                    format!("{}", dist),
                    ((x + cell_size/2 - 10) as i32, (y + cell_size/2 - 5) as i32),
                    ("sans-serif", 14).into_font().color(text_color),
                ))?;
            }
        }
    }

    // Row labels (left side)
    for (i, fp) in fingerprints.iter().enumerate() {
        let y = 50 + i * cell_size + cell_size / 2;
        root.draw(&Text::new(
            fp.label.clone(),
            (5, y as i32 - 5),
            ("sans-serif", 11).into_font(),
        ))?;
    }

    // Column labels (bottom)
    for (j, fp) in fingerprints.iter().enumerate() {
        let x = label_margin + j * cell_size;
        let y = 50 + n * cell_size + 5;
        let short_label: String = fp.label.chars().take(8).collect();
        root.draw(&Text::new(
            short_label,
            (x as i32, y as i32),
            ("sans-serif", 10).into_font(),
        ))?;
    }

    // Legend
    let legend_y = height as i32 - 30;

    // Legend: Green = PASS, Red = FAIL
    root.draw(&Rectangle::new([(10, legend_y - 5), (30, legend_y + 10)], RGBColor(50, 180, 50).filled()))?;
    root.draw(&Text::new("PASS".to_string(), (35, legend_y), ("sans-serif", 11).into_font()))?;
    root.draw(&Rectangle::new([(80, legend_y - 5), (100, legend_y + 10)], RGBColor(220, 50, 50).filled()))?;
    root.draw(&Text::new("FAIL".to_string(), (105, legend_y), ("sans-serif", 11).into_font()))?;
    root.draw(&Rectangle::new([(150, legend_y - 5), (170, legend_y + 10)], RGBColor(255, 180, 50).filled()))?;
    root.draw(&Text::new("Warning".to_string(), (175, legend_y), ("sans-serif", 11).into_font()))?;

    let expect_text = if expect_same {
        "Same browser: expect ≤32 (green)"
    } else {
        "Different browsers: expect >32 (green)"
    };
    root.draw(&Text::new(expect_text.to_string(), (260, legend_y), ("sans-serif", 11).into_font()))?;

    root.present()?;
    Ok(())
}

pub fn run() {
    println!("\n============================================================");
    println!("=== FINGERPRINT COMPARISON ===");
    println!("============================================================");

    // Load from both directories
    let same_fps = load_fingerprints("fingerprints/same");
    let diff_fps = load_fingerprints("fingerprints/different");

    if same_fps.is_empty() && diff_fps.is_empty() {
        eprintln!("\nNo fingerprints found!");
        eprintln!("Place JSON files in:");
        eprintln!("  fingerprints/same/      - fingerprints that SHOULD match (distance ≤32)");
        eprintln!("  fingerprints/different/ - fingerprints that should NOT match (distance >32)");
        return;
    }

    let mut plots_generated = Vec::new();

    // Process "same" fingerprints
    if !same_fps.is_empty() {
        println!("\n--- SAME BROWSER fingerprints ({} loaded) ---", same_fps.len());
        for fp in &same_fps {
            println!("  {} ({})", fp.label, fp.filename);
        }

        let distances = compute_distances(&same_fps);
        print_matrix(&same_fps, &distances, "Same Browser Distance Matrix");

        let (pass, fail, ok) = validate_results(&distances, true);
        if ok {
            println!("\n✓ PASS: All {} pairs within threshold (≤32)", pass);
        } else {
            println!("\n✗ FAIL: {} pairs OK, {} pairs too different (>32)", pass, fail);
        }

        let plot_file = "fingerprints_same.png";
        if let Err(e) = generate_heatmap(&same_fps, &distances, plot_file,
                                          "Same Browser Fingerprints", true) {
            eprintln!("Failed to generate plot: {}", e);
        } else {
            plots_generated.push(plot_file);
        }
    }

    // Process "different" fingerprints
    if !diff_fps.is_empty() {
        println!("\n--- DIFFERENT BROWSER fingerprints ({} loaded) ---", diff_fps.len());
        for fp in &diff_fps {
            println!("  {} ({})", fp.label, fp.filename);
        }

        let distances = compute_distances(&diff_fps);
        print_matrix(&diff_fps, &distances, "Different Browser Distance Matrix");

        let (pass, fail, ok) = validate_results(&distances, false);
        if ok {
            println!("\n✓ PASS: All {} pairs properly different (>32)", pass);
        } else {
            println!("\n✗ FAIL: {} pairs OK, {} pairs too similar (≤32)", pass, fail);
        }

        let plot_file = "fingerprints_different.png";
        if let Err(e) = generate_heatmap(&diff_fps, &distances, plot_file,
                                          "Different Browser Fingerprints", false) {
            eprintln!("Failed to generate plot: {}", e);
        } else {
            plots_generated.push(plot_file);
        }
    }

    // Open plots
    if !plots_generated.is_empty() {
        println!("\nGenerated: {}", plots_generated.join(", "));
        for plot in &plots_generated {
            let _ = std::process::Command::new("eog")
                .arg(plot)
                .spawn();
        }
    }
}
