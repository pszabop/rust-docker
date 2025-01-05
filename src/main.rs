mod cache_thrasher;
use std::cmp::Ordering;
use std::time::Instant;
use rand::Rng;

#[derive(Debug)]
struct TreeNode {
    key: String,
    value: String,
    left: Option<Box<TreeNode>>,
    right: Option<Box<TreeNode>>,
}

impl TreeNode {
    fn new(key: String, value: String) -> Self {
        TreeNode {
            key,
            value,
            left: None,
            right: None,
        }
    }

    fn insert(&mut self, key: String, value: String) {
        match key.cmp(&self.key) {
            Ordering::Less => {
                if let Some(ref mut left) = self.left {
                    left.insert(key, value);
                } else {
                    self.left = Some(Box::new(TreeNode::new(key, value)));
                }
            }
            Ordering::Greater => {
                if let Some(ref mut right) = self.right {
                    right.insert(key, value);
                } else {
                    self.right = Some(Box::new(TreeNode::new(key, value)));
                }
            }
            Ordering::Equal => {
                self.value = value;
            }
        }
    }
}

#[derive(Debug)]
struct BinaryTree {
    root: Option<Box<TreeNode>>,
    elapsed_time_ns: u128,
}

impl BinaryTree {
    fn new() -> Self {
        BinaryTree { root: None, elapsed_time_ns: 0 }
    }

    fn insert(&mut self, key: String, value: String) {
        let start_time = Instant::now();
        if let Some(ref mut root) = self.root {
            root.insert(key, value);
        } else {
            self.root = Some(Box::new(TreeNode::new(key, value)));
        }
        let elapsed = start_time.elapsed().as_nanos();
        self.elapsed_time_ns += elapsed;
    }
}

fn generate_random_string(rng: &mut impl Rng, length: usize) -> String {
    (0..length)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}

use crate::cache_thrasher::CacheThrasher;
#[tokio::main]
async fn main() {
    //use tokio::runtime::Runtime;

    /*
    #[tokio::test]
    async fn test_binary_tree_insertion() {
        let mut tree = BinaryTree::new();
        let mut rng = rand::thread_rng();

        for _ in 0..1_000_000 {
            let key = generate_random_string(&mut rng, 10);
            let value = generate_random_string(&mut rng, 20);
            tree.insert(key, value);
        }

        println!("Time taken for 1M insertions {:?}", tree.elapsed_time_ns/1_000_000 );
    }
    */

    let mut tree = BinaryTree::new();
    let mut rng = rand::thread_rng();
    let thrasher = CacheThrasher::new(1 << 28, 2);
    //let thrasher = CacheThrasher::new(1 << 12, 2);

    thrasher.start();

    let start_time = Instant::now();

    for _ in 0..1_000_000 {
        let key = generate_random_string(&mut rng, 10);
        let value = generate_random_string(&mut rng, 20);
        tree.insert(key, value);
    }
    thrasher.stop();

    let elapsed = start_time.elapsed();
    println!("Time taken for 1M insertions with cache thrasher: {:?}", elapsed.as_nanos() / 1_000_000);
}