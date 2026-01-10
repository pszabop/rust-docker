//! BK-Tree based fuzzy hash database with bulk leaves
//!
//! Uses a Burkhard-Keller tree for logarithmic-ish search, with bulk leaf nodes
//! for cache-friendly linear scans at the bottom level.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use parking_lot::RwLock;
use rand::Rng;

use crate::hash::hamming_distance_u64x4;

/// Record tracking hit count and recency for eviction scoring
#[repr(C)]
#[derive(Default)]
pub struct HotRecord {
    pub last_hit: AtomicU32,
    pub hit_count: AtomicU32,
}

impl HotRecord {
    #[inline]
    pub fn hit(&self, now: u32) {
        self.last_hit.store(now, Ordering::Relaxed);
        self.hit_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn eviction_score(&self, now: u32) -> u32 {
        let age = now.wrapping_sub(self.last_hit.load(Ordering::Relaxed));
        let hits = self.hit_count.load(Ordering::Relaxed);
        hits.saturating_mul(60).saturating_sub(age)
    }

    #[inline]
    pub fn reset(&self, now: u32) {
        self.last_hit.store(now, Ordering::Relaxed);
        self.hit_count.store(1, Ordering::Relaxed);
    }
}

/// A leaf node containing up to LEAF_CAPACITY entries for linear scan
struct BKLeaf {
    hashes: Vec<[u64; 4]>,
    records: Vec<HotRecord>,
}

impl BKLeaf {
    fn new(capacity: usize) -> Self {
        Self {
            hashes: Vec::with_capacity(capacity),
            records: Vec::with_capacity(capacity),
        }
    }

    fn len(&self) -> usize {
        self.hashes.len()
    }

    fn is_full(&self, capacity: usize) -> bool {
        self.hashes.len() >= capacity
    }

    /// Find entry within threshold, return index if found
    fn find(&self, query: &[u64; 4], threshold: u32) -> Option<usize> {
        for (i, hash) in self.hashes.iter().enumerate() {
            if hamming_distance_u64x4(query, hash) <= threshold {
                return Some(i);
            }
        }
        None
    }

    /// Insert entry, return index
    fn insert(&mut self, hash: [u64; 4], now: u32) -> usize {
        let idx = self.hashes.len();
        self.hashes.push(hash);
        self.records.push(HotRecord::default());
        self.records[idx].reset(now);
        idx
    }

    /// Find victim for eviction using stochastic sampling
    fn find_victim(&self, now: u32) -> usize {
        let len = self.hashes.len();
        if len == 0 {
            return 0;
        }

        let window = (len / 8).max(1);
        let start = rand::thread_rng().gen_range(0..len);

        let mut min_score = u32::MAX;
        let mut victim = start;

        for i in 0..window {
            let idx = (start + i) % len;
            let score = self.records[idx].eviction_score(now);
            if score < min_score {
                min_score = score;
                victim = idx;
            }
        }

        victim
    }

    /// Evict victim and replace with new entry
    fn evict_and_insert(&mut self, hash: [u64; 4], now: u32) -> usize {
        let victim = self.find_victim(now);
        self.hashes[victim] = hash;
        self.records[victim].reset(now);
        victim
    }
}

/// Internal BK-Tree node
struct BKNode {
    /// The pivot hash for this node
    pivot: [u64; 4],
    /// Children indexed by distance from pivot (sparse)
    /// Key = hamming distance, Value = child node index
    children: Vec<(u32, usize)>,  // (distance, node_index)
    /// Leaf data (only populated for leaf nodes)
    leaf: Option<BKLeaf>,
    /// Record for the pivot itself
    pivot_record: HotRecord,
}

impl BKNode {
    fn new_internal(pivot: [u64; 4]) -> Self {
        Self {
            pivot,
            children: Vec::new(),
            leaf: None,
            pivot_record: HotRecord::default(),
        }
    }

    fn new_leaf(pivot: [u64; 4], leaf_capacity: usize) -> Self {
        Self {
            pivot,
            children: Vec::new(),
            leaf: Some(BKLeaf::new(leaf_capacity)),
            pivot_record: HotRecord::default(),
        }
    }

    fn is_leaf(&self) -> bool {
        self.leaf.is_some()
    }

    fn get_child(&self, distance: u32) -> Option<usize> {
        self.children.iter()
            .find(|(d, _)| *d == distance)
            .map(|(_, idx)| *idx)
    }

    fn add_child(&mut self, distance: u32, node_idx: usize) {
        self.children.push((distance, node_idx));
    }
}

/// BK-Tree based fuzzy hash database
pub struct BKTreeDB {
    nodes: RwLock<Vec<BKNode>>,
    root: AtomicUsize,  // Index of root node (0 if empty, usize::MAX if no root)
    threshold: u32,
    leaf_capacity: usize,
    total_entries: AtomicUsize,
    max_entries: usize,
}

impl BKTreeDB {
    pub fn new(threshold: u32, leaf_capacity: usize, max_entries: usize) -> Self {
        Self {
            nodes: RwLock::new(Vec::new()),
            root: AtomicUsize::new(usize::MAX),
            threshold,
            leaf_capacity,
            total_entries: AtomicUsize::new(0),
            max_entries,
        }
    }

    /// Find or insert an entry. Returns (hit_count, is_new)
    pub fn find_or_insert(&self, query: &[u64; 4], now: u32) -> (u32, bool) {
        // Fast path: check if tree exists
        let root_idx = self.root.load(Ordering::Acquire);

        if root_idx == usize::MAX {
            // Empty tree - insert as root
            return self.insert_root(query, now);
        }

        // Search the tree
        let mut nodes = self.nodes.write();

        if let Some((node_idx, record_idx)) = self.search_tree(&nodes, root_idx, query) {
            // Found - update record
            let node = &nodes[node_idx];
            if let Some(ref leaf) = node.leaf {
                if let Some(idx) = record_idx {
                    leaf.records[idx].hit(now);
                    return (leaf.records[idx].hit_count.load(Ordering::Relaxed), false);
                }
            }
            // Matched pivot
            node.pivot_record.hit(now);
            return (node.pivot_record.hit_count.load(Ordering::Relaxed), false);
        }

        // Not found - insert
        self.insert_into_tree(&mut nodes, root_idx, query, now)
    }

    fn insert_root(&self, query: &[u64; 4], now: u32) -> (u32, bool) {
        let mut nodes = self.nodes.write();

        // Double-check after acquiring lock
        if self.root.load(Ordering::Acquire) != usize::MAX {
            let root_idx = self.root.load(Ordering::Acquire);
            if let Some((node_idx, record_idx)) = self.search_tree(&nodes, root_idx, query) {
                let node = &nodes[node_idx];
                if let Some(ref leaf) = node.leaf {
                    if let Some(idx) = record_idx {
                        leaf.records[idx].hit(now);
                        return (leaf.records[idx].hit_count.load(Ordering::Relaxed), false);
                    }
                }
                node.pivot_record.hit(now);
                return (node.pivot_record.hit_count.load(Ordering::Relaxed), false);
            }
            return self.insert_into_tree(&mut nodes, root_idx, query, now);
        }

        // Create root node as leaf
        let mut node = BKNode::new_leaf(*query, self.leaf_capacity);
        node.pivot_record.reset(now);
        let idx = nodes.len();
        nodes.push(node);
        self.root.store(idx, Ordering::Release);
        self.total_entries.fetch_add(1, Ordering::Relaxed);

        (1, true)
    }

    /// Search tree for entry within threshold
    /// Returns (node_index, Some(record_index)) for leaf match, (node_index, None) for pivot match
    fn search_tree(&self, nodes: &[BKNode], root_idx: usize, query: &[u64; 4]) -> Option<(usize, Option<usize>)> {
        let mut stack = vec![root_idx];

        while let Some(node_idx) = stack.pop() {
            let node = &nodes[node_idx];
            let dist = hamming_distance_u64x4(query, &node.pivot);

            // Check pivot
            if dist <= self.threshold {
                return Some((node_idx, None));
            }

            // Check leaf entries
            if let Some(ref leaf) = node.leaf {
                if let Some(idx) = leaf.find(query, self.threshold) {
                    return Some((node_idx, Some(idx)));
                }
            }

            // Add children within range [dist - threshold, dist + threshold]
            let min_dist = dist.saturating_sub(self.threshold);
            let max_dist = dist.saturating_add(self.threshold);

            for &(child_dist, child_idx) in &node.children {
                if child_dist >= min_dist && child_dist <= max_dist {
                    stack.push(child_idx);
                }
            }
        }

        None
    }

    /// Insert into tree (must not already exist)
    fn insert_into_tree(&self, nodes: &mut Vec<BKNode>, root_idx: usize, query: &[u64; 4], now: u32) -> (u32, bool) {
        // Check if we need to evict
        if self.total_entries.load(Ordering::Relaxed) >= self.max_entries {
            // For now, just evict from a random leaf
            self.evict_random(nodes, now);
        }

        // Navigate to insertion point
        let mut current_idx = root_idx;

        loop {
            let dist = hamming_distance_u64x4(query, &nodes[current_idx].pivot);

            // Check if child exists at this distance
            if let Some(child_idx) = nodes[current_idx].get_child(dist) {
                current_idx = child_idx;
                continue;
            }

            // No child at this distance - insert here
            if nodes[current_idx].is_leaf() {
                let leaf = nodes[current_idx].leaf.as_mut().unwrap();

                if leaf.is_full(self.leaf_capacity) {
                    // Leaf is full - create new child node
                    let mut new_node = BKNode::new_leaf(*query, self.leaf_capacity);
                    new_node.pivot_record.reset(now);
                    let new_idx = nodes.len();
                    nodes.push(new_node);
                    nodes[current_idx].add_child(dist, new_idx);
                    self.total_entries.fetch_add(1, Ordering::Relaxed);
                    return (1, true);
                } else {
                    // Add to leaf
                    leaf.insert(*query, now);
                    self.total_entries.fetch_add(1, Ordering::Relaxed);
                    return (1, true);
                }
            } else {
                // Internal node - create leaf child
                let mut new_node = BKNode::new_leaf(*query, self.leaf_capacity);
                new_node.pivot_record.reset(now);
                let new_idx = nodes.len();
                nodes.push(new_node);
                nodes[current_idx].add_child(dist, new_idx);
                self.total_entries.fetch_add(1, Ordering::Relaxed);
                return (1, true);
            }
        }
    }

    /// Evict a random entry (simple strategy for now)
    fn evict_random(&self, nodes: &mut [BKNode], now: u32) {
        // Find a random leaf and evict from it
        let mut rng = rand::thread_rng();

        for _ in 0..10 {  // Try up to 10 times to find a non-empty leaf
            let node_idx = rng.gen_range(0..nodes.len());
            if let Some(ref mut leaf) = nodes[node_idx].leaf {
                if leaf.len() > 1 {
                    let victim = leaf.find_victim(now);
                    // Mark as "evicted" by setting to a sentinel value
                    // (In practice, we'd want a proper deletion mechanism)
                    leaf.hashes[victim] = [u64::MAX; 4];
                    self.total_entries.fetch_sub(1, Ordering::Relaxed);
                    return;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.total_entries.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn node_count(&self) -> usize {
        self.nodes.read().len()
    }

    /// Fill with random entries for benchmarking
    pub fn fill_random(&self, count: usize, now: u32) {
        let mut rng = rand::thread_rng();
        for _ in 0..count {
            let hash: [u64; 4] = [
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
            ];
            self.find_or_insert(&hash, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bktree_basic() {
        let db = BKTreeDB::new(32, 100, 10000);

        let hash1: [u64; 4] = [1, 2, 3, 4];
        let (count, is_new) = db.find_or_insert(&hash1, 1000);
        assert_eq!(count, 1);
        assert!(is_new);

        let (count, is_new) = db.find_or_insert(&hash1, 1001);
        assert_eq!(count, 2);
        assert!(!is_new);
    }

    #[test]
    fn test_bktree_similar_hashes() {
        let db = BKTreeDB::new(32, 100, 10000);

        let hash1: [u64; 4] = [0, 0, 0, 0];
        let hash2: [u64; 4] = [1, 0, 0, 0];  // Distance 1 from hash1

        db.find_or_insert(&hash1, 1000);

        // hash2 should match hash1 (within threshold 32)
        let (count, is_new) = db.find_or_insert(&hash2, 1001);
        assert!(!is_new, "Similar hash should match existing entry");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_bktree_fill_and_search() {
        let db = BKTreeDB::new(32, 1000, 10000);
        db.fill_random(5000, 1000);

        assert!(db.len() >= 4000, "Should have many entries: {}", db.len());

        // Search for random hashes (should mostly miss)
        let mut rng = rand::thread_rng();
        let mut found = 0;
        for _ in 0..100 {
            let hash: [u64; 4] = [rng.gen(), rng.gen(), rng.gen(), rng.gen()];
            let (_, is_new) = db.find_or_insert(&hash, 2000);
            if !is_new {
                found += 1;
            }
        }

        println!("BK-Tree: {} entries, {} nodes, found {} of 100 random queries",
                 db.len(), db.node_count(), found);
    }
}
