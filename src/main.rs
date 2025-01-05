// use fastrand; // not fast enough, 40-50ns per random number!
use tokio::task;
use tokio::time::Instant;

const BUFFER_SIZE: usize = 1 << 28; // 256MB per thread

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

async fn access_range(range: BufferRange, buffer: &mut [u64], iterations: usize, rng: &mut SimpleRng) {
    println!("Accessing range: {} - {}, iterations: {}", range.start, range.end, iterations);
    for _ in 0..iterations {
        let index = rng.next_usize(range.start..range.end);
        buffer[index] = buffer[index].wrapping_add(1);
    }
}

#[tokio::main]
async fn main() {
    let range_sizes = [4 * 1024 / 8, 256 * 1024 / 8, 4 * 1024 * 1024 / 8, 128 * 1024 * 1024 / 8]; // Sizes in u64s
    let iterations = 4_000_000;
    let threads = 1;

    let ranges = create_ranges(BUFFER_SIZE / 8, &range_sizes);

    // Measure latency for each range size
    for &range in &ranges {
        println!("Testing range: {} - {} ({} u64s)", range.start, range.end, range.end - range.start);

        // Allocate buffers (256MB each)
        let mut buffers: Vec<_> = (0..threads)
            .map(|_| vec![0u64; BUFFER_SIZE / 8])
            .collect();

        let start_time = Instant::now();

        // Spawn tasks
        let mut tasks = Vec::new();
        for mut buffer in buffers.drain(..) {
            let range_clone = range;
            let mut rng = SimpleRng::new(12345); // Seed the RNG
            tasks.push(task::spawn(async move {
                access_range(range_clone, &mut buffer, iterations / threads, &mut rng).await;
            }));
        }

        for t in tasks {
            t.await.unwrap();
        }

        let elapsed = start_time.elapsed();
        let total_accesses = iterations * threads;
        let ns_per_access = elapsed.as_nanos() / total_accesses as u128;
        println!("Range: {} - {} completed in {:?} ({} ns per access)", range.start, range.end, elapsed, ns_per_access);
    }
}