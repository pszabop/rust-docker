//! Document modification functions for testing hash stability
//!
//! Provides various ways to modify documents to test how locality-sensitive
//! hashes respond to different types of changes.

use rand::Rng;

/// Types of modifications for testing
#[derive(Clone, Copy, Debug)]
pub enum ModType {
    Letter,
    Word,
    Insert,
    Remove,
}

/// Randomly change N individual characters (scattered changes)
pub fn randomly_modify_letters(input: &str, n: usize) -> String {
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
pub fn randomly_modify_words(input: &str, target_chars: usize) -> String {
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
pub fn randomly_insert_chars(input: &str, target_chars: usize) -> String {
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
pub fn randomly_remove_chars(input: &str, target_chars: usize) -> String {
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
pub fn find_csv_fields(chars: &[char]) -> Vec<(usize, usize)> {
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

/// Generate a random document (simulating browser fingerprint-like data)
pub fn generate_random_document(rng: &mut impl Rng, length: usize) -> String {
    // Mix of words, numbers, and punctuation like real fingerprint data
    let words = ["Chrome", "Firefox", "Safari", "Mozilla", "WebKit", "Gecko", "Intel", "AMD",
                 "Windows", "MacOS", "Linux", "Android", "true", "false", "null", "undefined",
                 "screen", "canvas", "audio", "webgl", "fonts", "plugins", "timezone"];

    let mut doc = String::with_capacity(length);
    while doc.len() < length {
        let choice = rng.gen_range(0..10);
        match choice {
            0..=4 => {
                // Random word
                let word = words[rng.gen_range(0..words.len())];
                doc.push_str(word);
            }
            5..=6 => {
                // Random number
                let num: u32 = rng.gen_range(0..10000);
                doc.push_str(&num.to_string());
            }
            7 => {
                // Random float
                let num: f32 = rng.gen_range(0.0..1000.0);
                doc.push_str(&format!("{:.2}", num));
            }
            _ => {
                // Random lowercase string
                let len = rng.gen_range(3..10);
                for _ in 0..len {
                    doc.push(rng.gen_range(b'a'..=b'z') as char);
                }
            }
        }
        // Add separator
        if doc.len() < length {
            let sep = match rng.gen_range(0..4) {
                0 => ", ",
                1 => "; ",
                2 => " ",
                _ => ", ",
            };
            doc.push_str(sep);
        }
    }
    doc.truncate(length);
    doc
}
