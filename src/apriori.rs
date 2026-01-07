use crate::repository::Repository;
use indextree::NodeId;
use std::collections::{HashMap, HashSet};

type Itemset = Vec<u32>;

#[derive(Debug, Clone)]
pub struct LevelConfig {
    pub support: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub lhs: Itemset,
    pub rhs: Itemset,
    pub count_full: u32,
    pub count_lhs: u32,
    pub num_transactions: u32,
}

impl Rule {
    pub fn confidence(&self) -> f64 {
        if self.count_lhs == 0 {
            0.0
        } else {
            self.count_full as f64 / self.count_lhs as f64
        }
    }

    pub fn support(&self) -> f64 {
        if self.num_transactions == 0 {
            0.0
        } else {
            self.count_full as f64 / self.num_transactions as f64
        }
    }

    pub fn print(&self, repository: &Repository) {
        // Convert LHS IDs to names (avoid unnecessary clones by using references)
        let lhs_names: Vec<&str> = self
            .lhs
            .iter()
            .filter_map(|&id| {
                repository
                    .node_by_id
                    .get(&id)
                    .map(|&node_id| repository.arena[node_id].get().name.as_str())
            })
            .collect();

        // Convert RHS IDs to names
        let rhs_names: Vec<&str> = self
            .rhs
            .iter()
            .filter_map(|&id| {
                repository
                    .node_by_id
                    .get(&id)
                    .map(|&node_id| repository.arena[node_id].get().name.as_str())
            })
            .collect();

        println!(
            "{:?} -> {:?} (conf: {:.3}, supp: {:.3})",
            lhs_names,
            rhs_names,
            self.confidence(),
            self.support()
        );
    }

    // pub fn print_ids(&self) {
    //     println!(
    //         "{:?} -> {:?} (conf: {:.3}, supp: {:.3})",
    //         self.lhs,
    //         self.rhs,
    //         self.confidence(),
    //         self.support()
    //     );
    // }
}

pub struct Apriori {
    pub repository: Repository,
    pub level_configs: HashMap<i32, LevelConfig>,
}

impl Apriori {
    pub fn new(
        transactions_path: &str,
        taxonomy_path: &str,
        level_configs: HashMap<i32, LevelConfig>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let repository = Repository::new(transactions_path, taxonomy_path)?;

        // Calculate max level by traversing the tree
        let max_level = Self::calculate_max_level(&repository);

        // Convert negative levels to absolute values
        let mut configs = HashMap::new();
        for (level, config) in level_configs {
            let actual_level = if level < 0 {
                (max_level as i32) + level + 1
            } else {
                level
            };
            configs.insert(actual_level, config);
        }

        Ok(Apriori {
            repository,
            level_configs: configs,
        })
    }

    fn calculate_max_level(repository: &Repository) -> usize {
        let mut max_level = 0;
        let mut queue = vec![(repository.root, 0usize)];

        while let Some((node_id, level)) = queue.pop() {
            max_level = max_level.max(level);
            for child_id in node_id.children(&repository.arena) {
                queue.push((child_id, level + 1));
            }
        }

        max_level
    }

    pub fn mine(&mut self) -> HashMap<usize, Vec<Rule>> {
        let mut rules_by_level = HashMap::new();
        let mut current_level = 1;
        let mut current_nodes: HashSet<NodeId> = self
            .repository
            .root
            .children(&self.repository.arena)
            .collect();

        while !current_nodes.is_empty() {
            // Check if we have config for this level
            let config = self.level_configs.get(&(current_level as i32));

            if let Some(config) = config {
                println!("Mining level {}", current_level);

                // Filter by support (prune)
                let min_support_count =
                    (config.support * self.repository.transaction_count as f64) as u32;

                // Remove nodes with insufficient support
                // Use retain for better performance than collect+remove
                let mut nodes_to_remove = Vec::new();
                current_nodes.retain(|&node_id| {
                    let node = self.repository.arena[node_id].get();
                    let should_keep = (node.bitmap.len() as u32) >= min_support_count;
                    if !should_keep {
                        nodes_to_remove.push(node_id);
                    }
                    should_keep
                });

                // Remove subtrees after filtering
                for node_id in nodes_to_remove {
                    node_id.remove_subtree(&mut self.repository.arena);
                }

                if !current_nodes.is_empty() {
                    // Generate frequent itemsets
                    println!("Generating itemsets for level {}", current_level);
                    let itemsets = self.generate_itemsets(&current_nodes, min_support_count);

                    // Generate rules
                    println!("Generating rules for level {}", current_level);
                    let rules = self.generate_rules(&itemsets, config.confidence);

                    println!("  Found {} rules at level {}", rules.len(), current_level);
                    rules_by_level.insert(current_level, rules);
                }
            }

            // Get children of frequent nodes for next iteration
            current_nodes = self.repository.get_children(&current_nodes);

            current_level += 1;
        }

        rules_by_level
    }

    fn generate_itemsets(
        &self,
        nodes: &HashSet<NodeId>,
        min_support_count: u32,
    ) -> HashMap<usize, HashMap<Itemset, u32>> {
        let mut large_itemsets: HashMap<usize, HashMap<Itemset, u32>> = HashMap::new();

        // Generate 1-itemsets
        println!("Generating 1-itemsets");
        let mut itemsets_1 = HashMap::new();
        for node_id in nodes.iter() {
            let node = self.repository.arena[*node_id].get();
            let support_count = node.bitmap.len() as u32;

            if support_count >= min_support_count {
                itemsets_1.insert(vec![node.id], support_count);
            }
        }

        if itemsets_1.is_empty() {
            return large_itemsets;
        }

        large_itemsets.insert(1, itemsets_1);

        // Generate k-itemsets for k >= 2
        let mut k = 2;
        while large_itemsets.contains_key(&(k - 1)) {
            println!("Generating k-itemsets for k = {}", k);
            // Collect and sort more efficiently
            let mut sorted_itemsets: Vec<Itemset> =
                large_itemsets[&(k - 1)].keys().cloned().collect();
            sorted_itemsets.sort();

            // Join step
            println!("Joining step");
            let candidates = self.join_step(&sorted_itemsets);

            // Prune step
            println!("Prune step");
            let unpruned_candidates = self.prune_step(&large_itemsets[&(k - 1)], &candidates);

            // Calculate support for candidates
            println!("Calculating support for candidates");
            let mut itemsets_k = HashMap::new();
            for candidate in unpruned_candidates {
                let support_count = self.calculate_support(&candidate);

                if support_count >= min_support_count {
                    itemsets_k.insert(candidate, support_count);
                }
            }

            if itemsets_k.is_empty() {
                break;
            }

            large_itemsets.insert(k, itemsets_k);
            k += 1;
        }

        large_itemsets
    }

    fn join_step(&self, itemsets: &[Itemset]) -> Vec<Itemset> {
        let mut candidates = Vec::new();
        let n = itemsets.len();

        let mut i = 0;
        // Iterate over every itemset in the itemsets
        while i < n {
            // The number of rows to skip in the while-loop, initially set to 1
            let mut skip = 1;

            let itemset_i = &itemsets[i];
            let prefix_i = &itemset_i[..itemset_i.len() - 1];
            let last_i = itemset_i[itemset_i.len() - 1];

            // Keep a list of all last elements, i.e. tail elements, to perform
            // 2-combinations on later on
            let mut tail_items = Vec::with_capacity(n - i);
            tail_items.push(last_i);

            // Iterate over every itemset following this itemset
            for j in (i + 1)..n {
                let itemset_j = &itemsets[j];
                let prefix_j = &itemset_j[..itemset_j.len() - 1];
                let last_j = itemset_j[itemset_j.len() - 1];

                // If it's the same, append and skip this itemset in while-loop
                if prefix_i == prefix_j {
                    tail_items.push(last_j);
                    skip += 1;
                } else {
                    // If it's not the same, break out of the for-loop
                    break;
                }
            }

            // For every 2-combination in the tail items, yield a new candidate
            // itemset, which is sorted.
            let tail_len = tail_items.len();
            for a_idx in 0..tail_len {
                for b_idx in (a_idx + 1)..tail_len {
                    let mut candidate = Vec::with_capacity(prefix_i.len() + 2);
                    candidate.extend_from_slice(prefix_i);
                    candidate.push(tail_items[a_idx]);
                    candidate.push(tail_items[b_idx]);
                    candidates.push(candidate);
                }
            }

            // Increment the while-loop counter
            i += skip;
        }

        candidates
    }

    fn prune_step(&self, itemmap: &HashMap<Itemset, u32>, candidates: &[Itemset]) -> Vec<Itemset> {
        let mut unpruned = Vec::with_capacity(candidates.len());

        for candidate in candidates {
            // Check all k-1 subsets (excluding last two items which are guaranteed by join_step)
            let mut should_keep = true;
            let candidate_len = candidate.len();

            // Only check first k-2 subsets (last two are guaranteed by join_step)
            for i in 0..(candidate_len.saturating_sub(2)) {
                // Build subset more efficiently without concat
                let mut subset = Vec::with_capacity(candidate_len - 1);
                subset.extend_from_slice(&candidate[..i]);
                subset.extend_from_slice(&candidate[i + 1..]);

                if !itemmap.contains_key(&subset) {
                    should_keep = false;
                    break;
                }
            }

            if should_keep {
                unpruned.push(candidate.clone());
            }
        }

        unpruned
    }

    fn calculate_support(&self, itemset: &Itemset) -> u32 {
        if itemset.is_empty() {
            return 0;
        }

        // Get first node's bitmap
        let first_node_id = match self.repository.node_by_id.get(&itemset[0]) {
            Some(&node_id) => node_id,
            None => return 0,
        };

        // Use reference to first bitmap and clone only when needed
        let first_bitmap = &self.repository.arena[first_node_id].get().bitmap;

        // If only one item, return its support directly
        if itemset.len() == 1 {
            return first_bitmap.len() as u32;
        }

        // Clone only when we need to intersect with other bitmaps
        let mut result = first_bitmap.clone();

        // Intersect with other nodes
        for &item_id in &itemset[1..] {
            let node_id = match self.repository.node_by_id.get(&item_id) {
                Some(&node_id) => node_id,
                None => return 0,
            };
            let node = self.repository.arena[node_id].get();
            result &= &node.bitmap;
        }

        result.len() as u32
    }

    fn generate_rules(
        &self,
        itemsets: &HashMap<usize, HashMap<Itemset, u32>>,
        min_confidence: f64,
    ) -> Vec<Rule> {
        let mut rules = Vec::new();

        // For each size >= 2
        for (&size, itemsets_map) in itemsets.iter() {
            if size < 2 {
                continue;
            }

            for (itemset, &count_full) in itemsets_map.iter() {
                // Try single-item consequents
                let mut h_1 = Vec::with_capacity(itemset.len());

                for i in 0..itemset.len() {
                    // Build LHS more efficiently
                    let mut lhs = Vec::with_capacity(itemset.len() - 1);
                    lhs.extend_from_slice(&itemset[..i]);
                    lhs.extend_from_slice(&itemset[i + 1..]);
                    // lhs is already sorted if itemset is sorted

                    let count_lhs = self.calculate_support(&lhs);
                    let confidence = count_full as f64 / count_lhs as f64;

                    if confidence >= min_confidence {
                        let rhs = vec![itemset[i]];
                        rules.push(Rule {
                            lhs: lhs.clone(),
                            rhs: rhs.clone(),
                            count_full,
                            count_lhs,
                            num_transactions: self.repository.transaction_count,
                        });
                        h_1.push(rhs);
                    }
                }

                if h_1.len() >= 2 {
                    self.ap_genrules(itemset, &h_1, itemsets, min_confidence, &mut rules);
                }
            }
        }

        rules
    }

    fn ap_genrules(
        &self,
        itemset: &[u32],
        h_m: &[Itemset],
        itemsets: &HashMap<usize, HashMap<Itemset, u32>>,
        min_confidence: f64,
        rules: &mut Vec<Rule>,
    ) {
        if itemset.len() <= h_m[0].len() + 1 {
            return;
        }

        // Generate larger consequents
        // Pre-allocate and sort more efficiently
        let mut sorted_h_m: Vec<Itemset> = Vec::with_capacity(h_m.len());
        for h in h_m {
            let mut sorted_h = h.clone();
            sorted_h.sort();
            sorted_h_m.push(sorted_h);
        }
        sorted_h_m.sort();

        let h_m_next = self.join_step(&sorted_h_m);
        // Convert sorted_h_m to HashMap for prune_step (using dummy value since we only need keys)
        let h_m_map: HashMap<Itemset, u32> = sorted_h_m.iter().map(|h| (h.clone(), 0)).collect();
        let h_m_next = self.prune_step(&h_m_map, &h_m_next);

        if h_m_next.is_empty() {
            return;
        }

        let mut h_m_copy = Vec::with_capacity(h_m_next.len());
        let itemset_len = itemset.len();

        for rhs in &h_m_next {
            // Build LHS more efficiently without HashSet conversion
            // Since itemset and rhs are sorted, we can use a merge-like approach
            let mut lhs = Vec::with_capacity(itemset_len - rhs.len());
            let mut itemset_idx = 0;
            let mut rhs_idx = 0;

            while itemset_idx < itemset_len {
                if rhs_idx < rhs.len() {
                    match itemset[itemset_idx].cmp(&rhs[rhs_idx]) {
                        std::cmp::Ordering::Less => {
                            lhs.push(itemset[itemset_idx]);
                            itemset_idx += 1;
                        }
                        std::cmp::Ordering::Equal => {
                            itemset_idx += 1;
                            rhs_idx += 1;
                        }
                        std::cmp::Ordering::Greater => {
                            rhs_idx += 1;
                        }
                    }
                } else {
                    // No more rhs items, add remaining itemset items
                    lhs.extend_from_slice(&itemset[itemset_idx..]);
                    break;
                }
            }
            // lhs is already sorted since itemset is sorted

            let count_full = itemsets
                .get(&itemset_len)
                .and_then(|m| m.get(itemset))
                .copied()
                .unwrap_or(0);
            let count_lhs = self.calculate_support(&lhs);
            let confidence = count_full as f64 / count_lhs as f64;

            if confidence >= min_confidence {
                rules.push(Rule {
                    lhs: lhs.clone(),
                    rhs: rhs.clone(),
                    count_full,
                    count_lhs,
                    num_transactions: self.repository.transaction_count,
                });
                h_m_copy.push(rhs.clone());
            }
        }

        if !h_m_copy.is_empty() {
            self.ap_genrules(itemset, &h_m_copy, itemsets, min_confidence, rules);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_repository() -> Repository {
        use std::fs;
        use std::thread;
        use std::time::{SystemTime, UNIX_EPOCH};

        // Create temporary test files with unique names based on timestamp and thread ID
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let thread_id = format!("{:?}", thread::current().id())
            .replace("ThreadId(", "")
            .replace(")", "");
        let taxonomy_path = format!("test_taxonomy_temp_{}_{}.txt", timestamp, thread_id);
        let transactions_path = format!("test_transactions_temp_{}_{}.txt", timestamp, thread_id);

        // Write files atomically
        fs::write(&taxonomy_path, "a,ROOT\nb,ROOT\nc,ROOT\nd,ROOT\n").unwrap();

        fs::write(&transactions_path, "a b c\na b d\na c d\nb c\n").unwrap();

        let repo = Repository::new(&transactions_path, &taxonomy_path).unwrap();

        // Cleanup
        fs::remove_file(taxonomy_path).ok();
        fs::remove_file(transactions_path).ok();

        repo
    }

    #[test]
    fn test_join_step() {
        let repo = create_test_repository();
        let apriori = Apriori {
            repository: repo,
            level_configs: HashMap::new(),
        };

        // Test case from Python implementation
        // itemsets = [(1, 2, 3), (1, 2, 4), (1, 3, 4), (1, 3, 5), (2, 3, 4)]
        // Expected: [(1, 2, 3, 4), (1, 3, 4, 5)]

        let itemsets = vec![
            vec![1, 2, 3],
            vec![1, 2, 4],
            vec![1, 3, 4],
            vec![1, 3, 5],
            vec![2, 3, 4],
        ];

        let candidates = apriori.join_step(&itemsets);

        // Should generate candidates
        assert!(!candidates.is_empty());

        // Check that we get the expected candidates
        let expected_1 = vec![1, 2, 3, 4];
        let expected_2 = vec![1, 3, 4, 5];

        let has_expected_1 = candidates.iter().any(|c| c == &expected_1);
        let has_expected_2 = candidates.iter().any(|c| c == &expected_2);

        assert!(
            has_expected_1 || has_expected_2,
            "Join step should generate expected candidates"
        );
    }

    #[test]
    fn test_prune_step() {
        let repo = create_test_repository();
        let apriori = Apriori {
            repository: repo,
            level_configs: HashMap::new(),
        };

        // Test case: itemsets of size 3
        let itemsets = HashMap::from([
            (vec![1, 2, 3], 2),
            (vec![1, 2, 4], 2),
            (vec![1, 3, 4], 2),
            (vec![2, 3, 4], 2),
        ]);

        // Candidate: [1, 2, 3, 4] - all subsets should be in itemsets
        let candidates = vec![vec![1, 2, 3, 4]];

        let unpruned = apriori.prune_step(&itemsets, &candidates);

        // Should keep the candidate since all subsets exist
        assert_eq!(unpruned.len(), 1);
        assert_eq!(unpruned[0], vec![1, 2, 3, 4]);

        // Test with invalid candidate (missing subset)
        let itemsets_incomplete = HashMap::from([
            (vec![1, 2, 3], 2),
            (vec![1, 2, 4], 2),
            // Missing [1, 3, 4] or [2, 3, 4]
        ]);

        let unpruned_invalid = apriori.prune_step(&itemsets_incomplete, &candidates);
        // Should prune since not all subsets exist
        assert_eq!(unpruned_invalid.len(), 0);
    }

    #[test]
    fn test_calculate_support() {
        let repo = create_test_repository();
        let apriori = Apriori {
            repository: repo,
            level_configs: HashMap::new(),
        };

        // Get node IDs from repository
        let mut node_ids = Vec::new();
        for (&id, _) in &apriori.repository.node_by_id {
            if id < 4 {
                // Use first 3 nodes (a, b, c)
                node_ids.push(id);
            }
        }
        node_ids.sort();

        if node_ids.len() >= 2 {
            // Test support for single item
            let single_item = vec![node_ids[0]];
            let support = apriori.calculate_support(&single_item);
            assert!(support > 0, "Single item should have support > 0");

            // Test support for two items
            if node_ids.len() >= 2 {
                let two_items = vec![node_ids[0], node_ids[1]];
                let support_two = apriori.calculate_support(&two_items);
                // Support is always >= 0 (u32)
                // Support of two items should be <= support of single item
                assert!(
                    support_two <= support,
                    "Support should decrease with more items"
                );
            }
        }

        // Test empty itemset
        assert_eq!(apriori.calculate_support(&vec![]), 0);

        // Test non-existent item
        assert_eq!(apriori.calculate_support(&vec![99999]), 0);
    }

    #[test]
    fn test_rule_confidence_calculation() {
        let rule = Rule {
            lhs: vec![1, 2],
            rhs: vec![3],
            count_full: 2,
            count_lhs: 3,
            num_transactions: 5,
        };

        assert_eq!(rule.confidence(), 2.0 / 3.0);
        assert_eq!(rule.support(), 2.0 / 5.0);
    }

    #[test]
    fn test_rule_edge_cases() {
        // Zero LHS count
        let rule1 = Rule {
            lhs: vec![1],
            rhs: vec![2],
            count_full: 0,
            count_lhs: 0,
            num_transactions: 10,
        };
        assert_eq!(rule1.confidence(), 0.0);

        // Zero transactions
        let rule2 = Rule {
            lhs: vec![1],
            rhs: vec![2],
            count_full: 5,
            count_lhs: 5,
            num_transactions: 0,
        };
        assert_eq!(rule2.support(), 0.0);
    }
}
