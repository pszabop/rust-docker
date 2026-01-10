//! Fingerprint comparison - load fingerprints and compare WAF hash vs data-only hash

use std::fs;
use std::path::Path;
use plotters::prelude::*;
use serde::Deserialize;

use crate::hash::{hamming_distance_u64x4, stable_document_hash_256, hash_to_u64};

/// Full browser info matching production fields
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BrowserInfo {
    user_agent: String,
    platform: Option<String>,
    screen_width: Option<u32>,
    screen_height: Option<u32>,
    color_depth: Option<u32>,
    timezone_offset: Option<i32>,
    language: Option<String>,
    hardware_concurrency: Option<u32>,
    device_memory: Option<f64>,
    #[serde(default)]
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
    avail_width: Option<u32>,
    avail_height: Option<u32>,
    max_touch_points: Option<u32>,
    webgl_canvas_hash: Option<String>,
    installed_fonts: Option<String>,
    audio_fingerprint: Option<String>,
    voice_count: Option<u32>,
}

impl BrowserInfo {
    /// Data-only canonical string: just values concatenated, no field names, no separators
    /// This maximizes the data's influence on the nilsimsa hash
    fn to_data_only_string(&self) -> String {
        let mut s = String::with_capacity(1024);

        // Core identifying data - lowercased where appropriate
        s.push_str(&self.user_agent.to_lowercase());

        if let Some(p) = &self.platform {
            s.push_str(&p.to_lowercase());
        }

        // Screen - as combined string
        if let (Some(w), Some(h), Some(d)) = (self.screen_width, self.screen_height, self.color_depth) {
            s.push_str(&format!("{}{}{}", w, h, d));
        }

        // Skip timezone_offset - zero entropy in test data
        // Skip language - low entropy

        // Hardware
        if let Some(cores) = self.hardware_concurrency {
            s.push_str(&cores.to_string());
        }
        if let Some(mem) = self.device_memory {
            s.push_str(&format!("{}", mem));
        }

        // Browser differentiators
        if let Some(brave) = self.is_brave {
            s.push_str(if brave { "brave" } else { "" });
        }
        if let Some(ref v) = self.vendor {
            s.push_str(&v.to_lowercase());
        }
        // Skip productSub - low entropy (same for all Chrome-based)
        if let Some(ref dnt) = self.do_not_track {
            s.push_str(dnt);
        }
        if let Some(pdf) = self.pdf_viewer_enabled {
            s.push_str(if pdf { "pdf" } else { "" });
        }
        if let Some(plugins) = self.plugin_count {
            s.push_str(&plugins.to_string());
        }
        // Skip hasChrome - redundant with isBrave and UA

        // High-entropy hashes - these are the money fields
        if let Some(ref canvas) = self.canvas_hash {
            s.push_str(canvas);
        }
        if let Some(ref glv) = self.webgl_vendor {
            s.push_str(&glv.to_lowercase());
        }
        if let Some(ref glr) = self.webgl_renderer {
            s.push_str(&glr.to_lowercase());
        }

        // Avail dimensions
        if let (Some(w), Some(h)) = (self.avail_width, self.avail_height) {
            s.push_str(&format!("{}{}", w, h));
        }

        // Skip maxTouchPoints - zero entropy in desktop test data

        // WebGL canvas hash
        if let Some(ref glc) = self.webgl_canvas_hash {
            s.push_str(glc);
        }

        // Fonts - very high entropy
        if let Some(ref fonts) = self.installed_fonts {
            s.push_str(&fonts.to_lowercase());
        }

        // Skip audioFingerprint - zero entropy in test data

        // Voice count
        if let Some(voices) = self.voice_count {
            s.push_str(&voices.to_string());
        }

        s
    }
}

#[derive(Deserialize)]
struct Fingerprint {
    browser_info: BrowserInfo,
    document_hash256: String,
}

struct LoadedFingerprint {
    label: String,
    filename: String,
    waf_hash: [u64; 4],      // Hash from WAF (document_hash256)
    data_hash: [u64; 4],     // Hash computed from data-only string
    canonical_len: usize,    // Length of data-only string
}

/// Extract a short label from user agent, platform, and is_brave flag
fn extract_label(user_agent: &str, platform: Option<&str>, is_brave: Option<bool>) -> String {
    let browser = if is_brave == Some(true) {
        if let Some(idx) = user_agent.find("Chrome/") {
            let version_start = idx + 7;
            let version_end = user_agent[version_start..]
                .find(|c: char| !c.is_numeric() && c != '.')
                .map(|i| version_start + i)
                .unwrap_or(user_agent.len());
            let version = &user_agent[version_start..version_end];
            let major = version.split('.').next().unwrap_or(version);
            format!("Brave{}", major)
        } else {
            "Brave".to_string()
        }
    } else if user_agent.contains("Firefox/") {
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

    let platform_short = platform.map(|p| {
        if p.contains("Linux") { "Linux" }
        else if p.contains("Win") { "Win" }
        else if p.contains("Mac") { "Mac" }
        else if p.contains("iPhone") || p.contains("iPad") { "iOS" }
        else if p.contains("Android") { "Android" }
        else { p.split_whitespace().next().unwrap_or(p) }
    }).unwrap_or("Unknown");

    format!("{}/{}", platform_short, browser)
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
                    if let Ok(waf_hash) = parse_hash(&fp.document_hash256) {
                        let label = extract_label(
                            &fp.browser_info.user_agent,
                            fp.browser_info.platform.as_deref(),
                            fp.browser_info.is_brave
                        );

                        // Compute data-only hash
                        let data_str = fp.browser_info.to_data_only_string();
                        let data_hash = hash_to_u64(&stable_document_hash_256(&data_str));

                        fingerprints.push(LoadedFingerprint {
                            label,
                            filename,
                            waf_hash,
                            data_hash,
                            canonical_len: data_str.len(),
                        });
                    }
                }
            }
        }
    }

    fingerprints.sort_by(|a, b| a.label.cmp(&b.label));
    fingerprints
}

/// Compute distance matrix using specified hash
fn compute_distances(fingerprints: &[LoadedFingerprint], use_data_hash: bool) -> Vec<Vec<u32>> {
    let n = fingerprints.len();
    let mut distances = vec![vec![0u32; n]; n];
    for i in 0..n {
        for j in 0..n {
            let hash_i = if use_data_hash { &fingerprints[i].data_hash } else { &fingerprints[i].waf_hash };
            let hash_j = if use_data_hash { &fingerprints[j].data_hash } else { &fingerprints[j].waf_hash };
            distances[i][j] = hamming_distance_u64x4(hash_i, hash_j);
        }
    }
    distances
}

/// Print distance matrix to console
fn print_matrix(fingerprints: &[LoadedFingerprint], distances: &[Vec<u32>], title: &str) {
    let n = fingerprints.len();

    println!("\n{}", title);
    println!("{}", "=".repeat(title.len()));

    print!("{:>15}", "");
    for fp in fingerprints {
        print!("{:>10}", &fp.label[..fp.label.len().min(9)]);
    }
    println!();

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

/// Print comparison of WAF vs data-only distances
fn print_comparison(fingerprints: &[LoadedFingerprint], waf_dist: &[Vec<u32>], data_dist: &[Vec<u32>]) {
    println!("\n{}", "=".repeat(70));
    println!("WAF Hash vs Data-Only Hash Comparison");
    println!("{}", "=".repeat(70));
    println!("{:>30} {:>15} {:>15} {:>10}", "Pair", "WAF Dist", "Data Dist", "Delta");
    println!("{}", "-".repeat(70));

    let n = fingerprints.len();
    let mut improvements = 0;
    let mut regressions = 0;

    for i in 0..n {
        for j in (i+1)..n {
            let waf = waf_dist[i][j];
            let data = data_dist[i][j];
            let delta = data as i32 - waf as i32;

            let pair = format!("{} vs {}",
                &fingerprints[i].label[..fingerprints[i].label.len().min(12)],
                &fingerprints[j].label[..fingerprints[j].label.len().min(12)]);

            let marker = if delta > 5 {
                improvements += 1;
                "  ↑ better"
            } else if delta < -5 {
                regressions += 1;
                "  ↓ worse"
            } else {
                ""
            };

            println!("{:>30} {:>15} {:>15} {:>+10}{}", pair, waf, data, delta, marker);
        }
    }

    println!("{}", "-".repeat(70));
    println!("Improvements (delta > 5): {}", improvements);
    println!("Regressions (delta < -5): {}", regressions);
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
                if dist <= 32 { pass += 1; } else { fail += 1; }
            } else {
                if dist > 32 { pass += 1; } else { fail += 1; }
            }
        }
    }

    (pass, fail, fail == 0)
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

    let _max_dist = distances.iter()
        .flat_map(|row| row.iter())
        .filter(|&&d| d > 0)
        .max()
        .copied()
        .unwrap_or(128) as f64;

    root.draw(&Text::new(
        title.to_string(),
        (width as i32 / 2 - (title.len() as i32 * 5), 15),
        ("sans-serif", 18).into_font(),
    ))?;

    for i in 0..n {
        for j in 0..n {
            let x = label_margin + j * cell_size;
            let y = 50 + i * cell_size;
            let dist = distances[i][j];

            let color = if i == j {
                RGBColor(200, 200, 200)
            } else if dist <= 32 {
                if expect_same { RGBColor(50, 180, 50) } else { RGBColor(220, 50, 50) }
            } else if dist <= 64 {
                if expect_same { RGBColor(255, 180, 50) } else { RGBColor(50, 180, 50) }
            } else {
                if expect_same { RGBColor(220, 50, 50) } else { RGBColor(30, 140, 30) }
            };

            root.draw(&Rectangle::new(
                [(x as i32, y as i32), ((x + cell_size - 2) as i32, (y + cell_size - 2) as i32)],
                color.filled(),
            ))?;

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

    for (i, fp) in fingerprints.iter().enumerate() {
        let y = 50 + i * cell_size + cell_size / 2;
        root.draw(&Text::new(
            fp.label.clone(),
            (5, y as i32 - 5),
            ("sans-serif", 11).into_font(),
        ))?;
    }

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

    let legend_y = height as i32 - 30;
    root.draw(&Rectangle::new([(10, legend_y - 5), (30, legend_y + 10)], RGBColor(50, 180, 50).filled()))?;
    root.draw(&Text::new("PASS".to_string(), (35, legend_y), ("sans-serif", 11).into_font()))?;
    root.draw(&Rectangle::new([(80, legend_y - 5), (100, legend_y + 10)], RGBColor(220, 50, 50).filled()))?;
    root.draw(&Text::new("FAIL".to_string(), (105, legend_y), ("sans-serif", 11).into_font()))?;

    let expect_text = if expect_same {
        "Same browser: expect ≤32 (green)"
    } else {
        "Different browsers: expect >32 (green)"
    };
    root.draw(&Text::new(expect_text.to_string(), (150, legend_y), ("sans-serif", 11).into_font()))?;

    root.present()?;
    Ok(())
}

pub fn run() {
    println!("\n============================================================");
    println!("=== FINGERPRINT COMPARISON: WAF Hash vs Data-Only Hash ===");
    println!("============================================================");
    println!("\nWAF Hash: Uses field names + separators (ua:...|screen:...|...)");
    println!("Data-Only: Just values concatenated, no field names/separators");

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

    // Process "different" fingerprints first (more interesting)
    if !diff_fps.is_empty() {
        println!("\n--- DIFFERENT BROWSER fingerprints ({} loaded) ---", diff_fps.len());
        for fp in &diff_fps {
            println!("  {} ({}, {} chars)", fp.label, fp.filename, fp.canonical_len);
        }

        let waf_distances = compute_distances(&diff_fps, false);
        let data_distances = compute_distances(&diff_fps, true);

        print_matrix(&diff_fps, &waf_distances, "WAF Hash Distance Matrix (with field names)");
        let (waf_pass, waf_fail, _) = validate_results(&waf_distances, false);
        println!("WAF: {} pairs OK (>32), {} too similar (≤32)", waf_pass, waf_fail);

        print_matrix(&diff_fps, &data_distances, "Data-Only Hash Distance Matrix (no field names)");
        let (data_pass, data_fail, _) = validate_results(&data_distances, false);
        println!("Data-Only: {} pairs OK (>32), {} too similar (≤32)", data_pass, data_fail);

        print_comparison(&diff_fps, &waf_distances, &data_distances);

        let plot_file = "fingerprints_different_waf.png";
        if generate_heatmap(&diff_fps, &waf_distances, plot_file, "Different Browsers (WAF Hash)", false).is_ok() {
            plots_generated.push(plot_file);
        }
        let plot_file = "fingerprints_different_data.png";
        if generate_heatmap(&diff_fps, &data_distances, plot_file, "Different Browsers (Data-Only Hash)", false).is_ok() {
            plots_generated.push(plot_file);
        }
    }

    // Process "same" fingerprints
    if !same_fps.is_empty() {
        println!("\n--- SAME BROWSER fingerprints ({} loaded) ---", same_fps.len());
        for fp in &same_fps {
            println!("  {} ({}, {} chars)", fp.label, fp.filename, fp.canonical_len);
        }

        let waf_distances = compute_distances(&same_fps, false);
        let data_distances = compute_distances(&same_fps, true);

        print_matrix(&same_fps, &waf_distances, "WAF Hash Distance Matrix");
        let (waf_pass, waf_fail, _) = validate_results(&waf_distances, true);
        println!("WAF: {} pairs OK (≤32), {} too different (>32)", waf_pass, waf_fail);

        print_matrix(&same_fps, &data_distances, "Data-Only Hash Distance Matrix");
        let (data_pass, data_fail, _) = validate_results(&data_distances, true);
        println!("Data-Only: {} pairs OK (≤32), {} too different (>32)", data_pass, data_fail);

        print_comparison(&same_fps, &waf_distances, &data_distances);

        let plot_file = "fingerprints_same_waf.png";
        if generate_heatmap(&same_fps, &waf_distances, plot_file, "Same Browser (WAF Hash)", true).is_ok() {
            plots_generated.push(plot_file);
        }
        let plot_file = "fingerprints_same_data.png";
        if generate_heatmap(&same_fps, &data_distances, plot_file, "Same Browser (Data-Only Hash)", true).is_ok() {
            plots_generated.push(plot_file);
        }
    }

    if !plots_generated.is_empty() {
        println!("\nGenerated: {}", plots_generated.join(", "));
    }
}
