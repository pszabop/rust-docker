#![allow(dead_code)]
use rand::Rng;
use rand::distributions::Alphanumeric;
use std::collections::HashMap;
use std::time::Instant;
use arrayvec::ArrayVec;
use nilsimsa;

const SEGMENT_SIZE: usize = 24; // Number of bits per segment
const OVERLAP_SIZE: usize = 8; // Number of overlapping bits
const NUM_SEGMENTS: usize = (64 - SEGMENT_SIZE) / (SEGMENT_SIZE - OVERLAP_SIZE) + 1; // Number of segments
const NUM_STRINGS: usize = 1_000_000; // Number of random strings to generate
const STRING_SIZE: usize = 128; // Size of each random string in bytes
const INITIAL_VEC_CAPACITY: usize = 8;
const BATCH_SIZE: u64 = 10_000;

fn main() {
    let mut rng = rand::thread_rng();
    //let mut sets: HashMap<String, Vec<u64>> = HashMap::with_capacity(NUM_STRINGS * NUM_SEGMENTS);
    let mut sets: HashMap<(usize, u32), ArrayVec<u64, INITIAL_VEC_CAPACITY>> = HashMap::with_capacity(NUM_STRINGS * NUM_SEGMENTS);
    //let mut sets: HashMap<String, Vec<u64>> = HashMap::new();
    let mut ctr:u64 = 0;

    let mut total_gen_time = 0;
    let mut total_hash_time = 0;
    let mut total_segment_time = 0;
    let mut total_set_time = 0;

    for _ in 0..NUM_STRINGS {
        ctr += 1;

        let start = Instant::now();
        let random_string: String = (0..STRING_SIZE)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();
        total_gen_time += start.elapsed().as_micros();

        let start = Instant::now();
        let hash = stable_document_hash(&random_string);
        total_hash_time += start.elapsed().as_micros();

        let start = Instant::now();
        let segments = create_segments(hash);
        total_segment_time += start.elapsed().as_micros(); 

        for (i, segment) in segments.iter().enumerate() {
            //let bucket_key = format!("bucket:{}:{}", i, segment);
            let bucket_key = (i, *segment);
            let start = Instant::now();
            sets.entry(bucket_key).or_insert_with(|| ArrayVec::new()).push(hash);
            total_set_time += start.elapsed().as_micros();
        }
        if ctr % BATCH_SIZE == 0 {
            println!("Processed {} strings", ctr);
            println!("Average time per {} strings:", BATCH_SIZE);
            println!("  Generation: {} µs", total_gen_time / BATCH_SIZE as u128 );
            println!("  Hashing: {} µs", total_hash_time / BATCH_SIZE as u128 );
            println!("  Segment creation: {} µs", total_segment_time / BATCH_SIZE as u128 );
            println!("  hash insertion time {} µs", total_set_time / BATCH_SIZE as u128 );
            println!("  HashMap size: {}", sets.len());
            println!("  HashMap capacity: {}", sets.capacity());
            total_gen_time = 0;
            total_hash_time = 0;
            total_segment_time = 0;
            total_set_time = 0;

            // Monitor for hash collisions
            let mut bucket_sizes: Vec<usize> = sets.values().map(|v| v.len()).collect();
            bucket_sizes.sort_unstable();
            let median_bucket_size = bucket_sizes[bucket_sizes.len() / 2];
            let max_bucket_size = bucket_sizes[bucket_sizes.len() - 1];
            println!("  Median bucket size: {}", median_bucket_size);
            println!("  Max bucket size: {}", max_bucket_size);
        }
    }

    // Calculate the size of each set
    let mut max_set_size = 0;
    let mut total_set_size = 0;
    let mut num_sets = 0;

    for set in sets.values() {
        let set_size = set.len();
        if set_size > max_set_size {
            max_set_size = set_size;
        }
        total_set_size += set_size;
        num_sets += 1;
    }

    let average_set_size = total_set_size as f64 / num_sets as f64;

    println!("Number of sets: {}", num_sets);
    println!("Maximum set size: {}", max_set_size);
    println!("Average set size: {:.2}", average_set_size);
}

fn stable_document_hash(input: &str) -> u64 {
    let mut hasher = nilsimsa::Nilsimsa::new();
    hasher.update(input);
    let hex_str = hasher.digest();

    let truncated_hex = &hex_str[0..16]; // 16 hex digits = 64 bits
    u64::from_str_radix(truncated_hex, 16).expect("Invalid hex from Nilsimsa")
}

fn create_segments(hash: u64) -> Vec<u32> {
    let mut segments = Vec::new();

    for i in 0..NUM_SEGMENTS {
        let shift = i * (SEGMENT_SIZE - OVERLAP_SIZE);
        let segment = ((hash >> shift) & ((1 << SEGMENT_SIZE) - 1)) as u32;
        segments.push(segment);
    }

    segments
}