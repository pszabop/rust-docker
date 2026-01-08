//! Thread-safe rate limiter benchmark

use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crate::hash::stable_document_hash_256;
use crate::modification::{randomly_modify_letters, generate_random_document};
use crate::rate_limiter::FuzzyRateLimiter;

pub fn run() {
    println!("\n============================================================");
    println!("=== THREAD-SAFE RATE LIMITER BENCHMARK (Tokio Compatible) ===");
    println!("============================================================\n");

    let mut rng = rand::thread_rng();

    // Configuration
    const NUM_BUCKETS: usize = 12;        // 12 buckets
    const BUCKET_DURATION: u64 = 10;       // 10 seconds each = 2 min window
    const THRESHOLD: u32 = 30;
    const CAPACITY_PER_BUCKET: usize = 10_000;

    let limiter = Arc::new(FuzzyRateLimiter::new(
        NUM_BUCKETS,
        BUCKET_DURATION,
        THRESHOLD,
        CAPACITY_PER_BUCKET,
    ));

    println!("Configuration:");
    println!("  Window: {} seconds ({} buckets × {} sec)",
             limiter.window_duration_secs(), NUM_BUCKETS, BUCKET_DURATION);
    println!("  Hamming threshold: {}", THRESHOLD);
    println!("  Using: parking_lot::RwLock (tokio-safe)\n");

    // Pre-generate test fingerprints
    const NUM_FINGERPRINTS: usize = 1000;
    const DOC_LENGTH: usize = 1000;

    println!("Generating {} test fingerprints...", NUM_FINGERPRINTS);
    let fingerprints: Vec<[u8; 32]> = (0..NUM_FINGERPRINTS)
        .map(|_| {
            let doc = generate_random_document(&mut rng, DOC_LENGTH);
            stable_document_hash_256(&doc)
        })
        .collect();

    // Benchmark single-threaded performance
    println!("\n--- Single-threaded benchmark ---");
    let timestamp = 100u64; // Fixed timestamp for testing

    let start = Instant::now();
    const ITERATIONS: usize = 10_000;
    for i in 0..ITERATIONS {
        let fp = &fingerprints[i % fingerprints.len()];
        let _count = limiter.check_request(fp, timestamp);
    }
    let elapsed = start.elapsed();
    let per_request = elapsed.as_micros() as f64 / ITERATIONS as f64;

    println!("  {} requests in {:?}", ITERATIONS, elapsed);
    println!("  {:.2}μs per request", per_request);
    println!("  Entries in limiter: {}", limiter.total_entries());

    // Benchmark with fuzzed fingerprints
    println!("\n--- Fuzzed fingerprint benchmark ---");
    let docs: Vec<String> = (0..NUM_FINGERPRINTS)
        .map(|_| generate_random_document(&mut rng, DOC_LENGTH))
        .collect();

    let limiter3 = Arc::new(FuzzyRateLimiter::new(
        NUM_BUCKETS, BUCKET_DURATION, THRESHOLD, CAPACITY_PER_BUCKET,
    ));

    // Insert originals
    for doc in &docs {
        let hash = stable_document_hash_256(doc);
        limiter3.check_request(&hash, timestamp);
    }
    println!("  Inserted {} original fingerprints", docs.len());

    // Query with fuzzed versions
    let start = Instant::now();
    let mut hits = 0;
    for doc in &docs {
        let fuzzed = randomly_modify_letters(doc, 16);
        let hash = stable_document_hash_256(&fuzzed);
        let count = limiter3.check_request(&hash, timestamp);
        if count > 1 {
            hits += 1;
        }
    }
    let elapsed = start.elapsed();

    println!("  Fuzzed queries: {} in {:?}", docs.len(), elapsed);
    println!("  Hit rate (found original): {:.1}%", hits as f64 / docs.len() as f64 * 100.0);
    println!("  {:.2}μs per fuzzed request", elapsed.as_micros() as f64 / docs.len() as f64);

    // Multi-threaded benchmark (simulating tokio runtime threads)
    println!("\n--- Multi-threaded benchmark (4 threads, simulating tokio workers) ---");
    let limiter4 = Arc::new(FuzzyRateLimiter::new(
        NUM_BUCKETS, BUCKET_DURATION, THRESHOLD, CAPACITY_PER_BUCKET,
    ));
    let fingerprints_arc = Arc::new(fingerprints.clone());

    let num_threads = 4;
    let requests_per_thread = 5000;

    let start = Instant::now();
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let limiter = Arc::clone(&limiter4);
            let fps = Arc::clone(&fingerprints_arc);
            thread::spawn(move || {
                for i in 0..requests_per_thread {
                    let fp = &fps[(thread_id * 1000 + i) % fps.len()];
                    let _count = limiter.check_request(fp, 100);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
    let elapsed = start.elapsed();
    let total_requests = num_threads * requests_per_thread;

    println!("  {} total requests across {} threads in {:?}",
             total_requests, num_threads, elapsed);
    println!("  {:.2}μs per request (wall clock / total requests)",
             elapsed.as_micros() as f64 / total_requests as f64);
    println!("  Throughput: {:.0} requests/sec",
             total_requests as f64 / elapsed.as_secs_f64());
    println!("  Entries in limiter: {}", limiter4.total_entries());

    // Tokio async benchmark
    println!("\n--- Tokio async benchmark ---");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let limiter5 = Arc::new(FuzzyRateLimiter::new(
        NUM_BUCKETS, BUCKET_DURATION, THRESHOLD, CAPACITY_PER_BUCKET,
    ));
    let fingerprints_arc2 = Arc::new(fingerprints);

    let tokio_result = rt.block_on(async {
        let num_tasks = 8;
        let requests_per_task = 2500;

        let start = Instant::now();
        let mut handles = Vec::new();

        for task_id in 0..num_tasks {
            let limiter = Arc::clone(&limiter5);
            let fps = Arc::clone(&fingerprints_arc2);

            handles.push(tokio::spawn(async move {
                for i in 0..requests_per_task {
                    let fp = &fps[(task_id * 500 + i) % fps.len()];
                    // Direct call - parking_lot is safe in async context for <100μs ops
                    let _count = limiter.check_request(fp, 100);
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let elapsed = start.elapsed();
        (num_tasks * requests_per_task, elapsed)
    });

    let (total_async_requests, async_elapsed) = tokio_result;
    println!("  {} total requests across 8 tokio tasks in {:?}",
             total_async_requests, async_elapsed);
    println!("  {:.2}μs per request",
             async_elapsed.as_micros() as f64 / total_async_requests as f64);
    println!("  Throughput: {:.0} requests/sec",
             total_async_requests as f64 / async_elapsed.as_secs_f64());
    println!("  Entries in limiter: {}", limiter5.total_entries());

    // Test is_rate_limited helper
    println!("\n--- Rate limiting demo ---");
    let demo_limiter = Arc::new(FuzzyRateLimiter::new(1, 60, 30, 1000));
    let test_hash = stable_document_hash_256("test fingerprint");

    for i in 1..=15 {
        let is_limited = demo_limiter.is_rate_limited(&test_hash, 100, 10);
        println!("  Request {}: count={}, rate_limited={}",
                 i,
                 demo_limiter.check_request(&test_hash, 100) - 1, // -1 because check increments
                 is_limited);
    }

    println!("\n============================================================");
    println!("SUMMARY");
    println!("============================================================");
    println!("\nTarget: 500 RPS per core = 2000μs budget per request");
    println!("Rate limiter overhead: {:.2}μs per request", per_request);
    println!("Budget used: {:.2}%", per_request / 2000.0 * 100.0);

    println!("\n--- Usage Example (Pingora/Tokio) ---");
    println!(r#"
```rust
use std::sync::Arc;
use std::time::{{SystemTime, UNIX_EPOCH}};

// Initialize once at startup
let limiter = Arc::new(FuzzyRateLimiter::new(
    12,      // 12 buckets
    10,      // 10 seconds each = 2 minute window
    30,      // Hamming distance threshold
    10_000,  // Capacity per bucket
));

// Start automatic rotation
let _rotation_handle = limiter.start_rotation_task();

// In your request handler (async fn):
async fn handle_request(
    limiter: &FuzzyRateLimiter,
    fingerprint_hash: [u8; 32],
) -> Result<Response, RateLimited> {{
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check rate limit (sub-50μs, safe in async)
    if limiter.is_rate_limited(&fingerprint_hash, now, 100) {{
        return Err(RateLimited);
    }}

    // Process request...
    Ok(Response::new())
}}
```
"#);
}
