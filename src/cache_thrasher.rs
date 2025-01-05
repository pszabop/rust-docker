use tokio::task;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use tokio::time::Instant;

const DEFAULT_BUFFER_SIZE: usize = 1 << 28; // 256MB per thread

/// Struct representing a buffer range.
#[derive(Clone, Copy)]
struct BufferRange {
    start: usize,
    end: usize,
}

/// Creates non-overlapping buffer ranges.
fn create_ranges(total_size: usize, range_sizes: &[usize]) -> Vec<BufferRange> {
    let mut ranges = Vec::new();
    let mut current_start = 0;

    for &size in range_sizes {
        if current_start + size > total_size {
            break;
        }
        ranges.push(BufferRange {
            start: current_start,
            end: current_start + size,
        });
        current_start += size;
    }

    ranges
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        SimpleRng { state: seed }
    }

    fn next(&mut self) -> u64 {
        // Constants for the LCG
        const A: u64 = 6364136223846793005;
        const C: u64 = 1;
        self.state = self.state.wrapping_mul(A).wrapping_add(C);
        self.state
    }

    fn next_usize(&mut self, range: std::ops::Range<usize>) -> usize {
        (self.next() as usize % (range.end - range.start)) + range.start
    }
}

async fn access_range(range: BufferRange, buffer: &mut [u64], stop_flag: Arc<AtomicBool>, rng: &mut SimpleRng) {
    println!("Accessing range: {} - {}", range.start, range.end);
    while !stop_flag.load(Ordering::Relaxed) {
        let start_time = Instant::now();
        for _ in 0..1_000_000 {
            let index = rng.next_usize(range.start..range.end);
            buffer[index] = buffer[index].wrapping_add(1);
        }
        let elapsed = start_time.elapsed();
        let total_accesses = 1_000_000;
        let ns_per_access = elapsed.as_nanos() / total_accesses as u128;
        println!("ns per access: {}", ns_per_access);
    }
}


pub struct CacheThrasher {
    buffer_size: usize,
    threads: usize,
    stop_flag: Arc<AtomicBool>,
}

impl CacheThrasher {
    pub fn new(buffer_size: usize, threads: usize) -> Self {
        CacheThrasher {
            buffer_size,
            threads,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        //let range_sizes = [256 * 1024 * 1024 / 8]; // Sizes in u64s
        let range_sizes = [self.buffer_size / 8]; // Size in u64s
        let ranges = create_ranges(self.buffer_size / 8, &range_sizes);

        for &range in &ranges {
            println!("Testing range: {} - {} ({} u64s)", range.start, range.end, range.end - range.start);

            // Allocate buffers per thread
            let mut buffers: Vec<_> = (0..self.threads)
                .map(|_| vec![0u64; self.buffer_size / 8])
                .collect();

            // create tasks
            let mut tasks = Vec::new();
            for mut buffer in buffers.drain(..) {
                let range_clone = range;
                let stop_flag_clone = Arc::clone(&self.stop_flag);
                let mut rng = SimpleRng::new(12345); // Seed the RNG
                tasks.push(task::spawn(async move {
                    access_range(range_clone, &mut buffer, stop_flag_clone, &mut rng).await;
                }));
            }
            println!("created {} tasks", tasks.len());

            // Spawn tasks
            for t in tasks {
                println!("spawning tasks");
                tokio::spawn(async move {
                    t.await.unwrap();
                });
            }
        }
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_cache_thrasher() {
        let rt = Runtime::new().unwrap();
        let thrasher = CacheThrasher::new(DEFAULT_BUFFER_SIZE, 1);

        rt.block_on(async {
            thrasher.start();

            // Run for 5 seconds
            thread::sleep(Duration::from_secs(5));
            thrasher.stop();
        });
    }
}