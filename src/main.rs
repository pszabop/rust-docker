#![allow(dead_code)]
use rand::Rng;
use simhash;
use std::fs;
use xxhash_rust::xxh3::xxh3_64; // Example 32-bit hash
use std::cmp::min;
use ssdeep;
use nilsimsa;

/* 
    */
fn main() {
    // Read the contents of the JSON files as strings
    let first_long = fs::read_to_string("chrome_values.json")
        .expect("Unable to read file chrome_values.json");
    let second_long = fs::read_to_string("firefox_values.json")
        .expect("Unable to read file firefox_values.json");

    // Call the function to output the hashes and other results
    let first_hash = stable_document_hash(&first_long);
    let second_hash = stable_document_hash(&second_long);
    output_results(first_hash, second_hash);

    let modified_string = randomly_modify_string(&first_long, 10);
    let modified_hash = stable_document_hash(&modified_string);
    output_results(first_hash, modified_hash);
}


fn stable_document_hash(input: &str) -> u64 {
    let mut hasher = nilsimsa::Nilsimsa::new();
    hasher.update(input);
    let hex_str = hasher.digest();

    let truncated_hex = &hex_str[0..16]; // 16 hex digits = 64 bits
    u64::from_str_radix(truncated_hex, 16).expect("Invalid hex from Nilsimsa")
}

fn stable_document_hash6(input: &str) -> u64 {
    weighted_bit_count_hash(input, 8)
}

fn weighted_bit_count_hash(input: &str, chunk_size: usize) -> u64 {
    let mut bit_counts = [0i32; 64];
    let chunks: Vec<&str> = input.split(',').collect();

    for chunk in chunks {
        let chunk_hash = xxh3_64(chunk.as_bytes());

        for bit_pos in 0..64 {
            if (chunk_hash & (1 << bit_pos)) != 0 {
                bit_counts[bit_pos] += 1;
            } else {
                bit_counts[bit_pos] -= 1;
            }
        }
    }

    let mut result = 0u64;
    for bit_pos in 0..64 {
        if bit_counts[bit_pos] >= 0 {
            result |= 1 << bit_pos;
        }
    }

    result
}

/*
fn main() {
    let first_long = fs::read_to_string("chrome_values.json")
        .expect("Unable to read file chrome_values.json");
    println!("{:?}", ssdeep::hash_buf(&first_long.as_bytes()));
    let modified_string = randomly_modify_string(&first_long, 10);
    println!("{:?}", ssdeep::hash_buf(&modified_string.as_bytes()));
}
    */



// this one sucks worse than simhash itself
use tlsh::{Tlsh, Version, BucketKind, ChecksumKind, TlshBuilder};
fn stable_document_hash5(input: &str) -> u64 {
    let mut builder = TlshBuilder::new(
        BucketKind::Bucket128,
        ChecksumKind::OneByte,
        tlsh::Version::Version4,
     );
     builder.update(input.as_bytes());
     let tlsh = builder.build().unwrap();
         // `tlsh.hash()` returns a hex-encoded string; take the first 16 hex digits for 64 bits
    let hex_str = tlsh.hash();
    println!("TLSH: {}", hex_str);
    let truncated_hex = &hex_str[24..40]; // 16 hex digits = 64 bits
    u64::from_str_radix(truncated_hex, 16).expect("Invalid hex from TLSH")
}


fn stable_document_hash4(input: &str) -> u64 {
    stable_fuzzy_hash(input, 4)
}

/// Fuzzy hash that remains nearly the same if only a few characters change.
/// allegedyl
fn stable_fuzzy_hash(input: &str, chunk_size: usize) -> u64 {
    // We'll accumulate bit counts for 64 bits
    let mut bit_counts = [0i32; 64];

    // Split the input into fixed-size chunks
    let bytes = input.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        let end = min(start + chunk_size, bytes.len());
        let chunk = &bytes[start..end];

        // Hash this chunk to 64 bits
        let chunk_hash = xxh3_64(chunk);

        // For each bit in the chunk hash, increment or decrement the counter
        for bit_pos in 0..64 {
            if (chunk_hash & (1 << bit_pos)) != 0 {
                bit_counts[bit_pos] += 1;
            } else {
                bit_counts[bit_pos] -= 1;
            }
        }

        start += chunk_size;
    }

    // Construct final 64-bit hash: if count >= 0, set bit
    let mut result = 0u64;
    for bit_pos in 0..64 {
        if bit_counts[bit_pos] >= 0 {
            result |= 1 << bit_pos;
        }
    }

    result
}

/*
fn stable_document_hash3(input: &str) -> u64 {
    chunked_simhash(input, Some(8))
}
fn chunked_simhash(input: &str, chunk_size: Option<usize>) -> u64 {
    let size = chunk_size.unwrap_or(8);
    let mut combined_hash = 0u64;
    let len = input.len();
    let mut start = 0;

    while start < len {
        let end = std::cmp::min(start + size, len);
        let chunk_str = &input[start..end];
        let chunk_hash = simhash::simhash(&chunk_str);
        combined_hash ^= chunk_hash;
        start += size;
    }

    combined_hash
}
    */
fn stable_document_hash2(input: &str) -> u64 {
    const BASE: u64 = 257;  // Base for rolling hash
    const MOD: u64 = (1 << 61) - 1; // Large prime modulus
    const MASK_LOWER: u64 = 0xFFFFFFFF; // Mask for lower 32 bits

    let mut upper: u64 = 0; // Stable upper bits
    let mut lower: u64 = 0; // Volatile lower bits
    let mut base_pow: u64 = 1;

    for (i, &byte) in input.as_bytes().iter().enumerate() {
        let byte_val = byte as u64;

        // Update upper: weighted by position to favor stability
        upper = upper
            .wrapping_add(byte_val.wrapping_mul(base_pow % MOD))
            .wrapping_rem(MOD);

        // Update lower: sensitive to individual bytes
        lower = lower
            .wrapping_add(byte_val.wrapping_mul((i as u64 + 1)))
            .wrapping_rem(MOD);

        // Update base power for next round
        base_pow = base_pow.wrapping_mul(BASE).wrapping_rem(MOD);
    }

    // Combine upper and lower
    (upper & !MASK_LOWER) | (lower & MASK_LOWER)
}


fn stable_document_hash1(doc: &str) -> u64 {
    // Break the document into 16-byte chunks
    let chunk_size = 16;
    let bytes = doc.as_bytes();
    let mut bit_counts = [0i32; 64];

    // For each chunk, compute 32-bit hash, then spread bits into a 64-bit pattern
    for chunk_start in (0..bytes.len()).step_by(chunk_size) {
        let chunk_end = min(chunk_start + chunk_size, bytes.len());
        let hash_32 = xxh3_64(&bytes[chunk_start..chunk_end]) as u64;

        // For each bit, increment or decrement
        for bit_pos in 0..32 {
            if (hash_32 & (1 << bit_pos)) != 0 {
                bit_counts[bit_pos as usize] += 1;
            } else {
                bit_counts[bit_pos as usize] -= 1;
            }
        }
    }

    // Combine bits to produce a final 64-bit value (top 32 bits will remain zeroed)
    let mut result = 0u64;
    for bit_pos in 0..64 {
        if bit_pos < 32 && bit_counts[bit_pos] >= 0 {
            result |= 1 << bit_pos;
        }
    }

    result
}

fn stable_document_hash0(doc: &str) -> u64 {
    return simhash::simhash(doc);
}

fn output_results(h: u64, i: u64) {
    let bithamming = simhash::hamming_distance(h, i);
    let distance = simhash::hash_similarity(h, i);
    println!("Hamming distance: {}, float distance: {}", bithamming, distance);

    println!("{:<64} {:<16}", "Binary (64 bits)", "Hexadecimal (16 hex digits)");
    println!("{:<64} {:<16}", format!("{:064b}", h), format!("{:016x}", h));
    println!("{:<64} {:<16}", format!("{:064b}", i), format!("{:016x}", i));
}

fn output_results128(h: u128, i: u128) {
    //println!("Hamming distance: {}, float distance: {}", bithamming, distance);

    println!("{:<128} {:<16}", "Binary (64 bits)", "Hexadecimal (16 hex digits)");
    println!("{:<128} {:<16}", format!("{:128b}", h), format!("{:032x}", h));
    println!("{:<128} {:<16}", format!("{:128b}", i), format!("{:032x}", i));
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