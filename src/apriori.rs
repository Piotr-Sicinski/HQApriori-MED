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
        // Convert LHS IDs to names
        let lhs_names: Vec<String> = self
            .lhs
            .iter()
            .filter_map(|&id| {
                repository
                    .node_by_id
                    .get(&id)
                    .map(|&node_id| repository.arena[node_id].get().name.clone())
            })
            .collect();

        // Convert RHS IDs to names
        let rhs_names: Vec<String> = self
            .rhs
            .iter()
            .filter_map(|&id| {
                repository
                    .node_by_id
                    .get(&id)
                    .map(|&node_id| repository.arena[node_id].get().name.clone())
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
}

pub struct Apriori {
    pub repository: Repository,
    pub max_level: usize,
    pub level_configs: HashMap<i32, LevelConfig>,
}

impl Apriori {
    pub fn new(
        transactions_path: &str,
        taxonomy_path: &str,
        level_configs: HashMap<i32, LevelConfig>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let repository = Repository::new(transactions_path, taxonomy_path)?;
        let max_level = repository.arena[repository.root].get().height;

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
            max_level,
            level_configs: configs,
        })
    }

    pub fn mine(&mut self) -> HashMap<usize, Vec<Rule>> {
        let mut rules_by_level = HashMap::new();
        let mut current_level = self.max_level - 1;
        let mut current_nodes: HashSet<NodeId> = self
            .repository
            .root
            .children(&self.repository.arena)
            .collect();

        while !current_nodes.is_empty() && current_level > 0 {
            // Check if we have config for this level
            let config = self.level_configs.get(&(current_level as i32));

            println!("Current level: {}", current_level);
            let mut node_names: Vec<_> = current_nodes
                .iter()
                .map(|&nid| self.repository.arena[nid].get().name.clone())
                .collect();
            node_names.sort();
            println!("Current nodes: {:?}", node_names);

            if let Some(config) = config {
                println!("Mining level {}", current_level);

                // Filter by support (prune)
                let min_support_count =
                    (config.support * self.repository.transaction_count as f64) as u32;

                // Collect nodes to remove (can't modify while iterating)
                let nodes_to_remove: Vec<NodeId> = current_nodes
                    .iter()
                    .filter(|&&node_id| {
                        let node = self.repository.arena[node_id].get();
                        (node.bitmap.len() as u32) < min_support_count
                    })
                    .cloned()
                    .collect();

                // Remove nodes with insufficient support
                for node_id in nodes_to_remove {
                    node_id.remove_subtree(&mut self.repository.arena);
                    current_nodes.remove(&node_id);
                }

                if !current_nodes.is_empty() {
                    // Generate frequent itemsets
                    let itemsets = self.generate_itemsets(&current_nodes, min_support_count);

                    // Generate rules
                    let rules = self.generate_rules(&itemsets, config.confidence);

                    println!("  Found {} rules at level {}", rules.len(), current_level);
                    rules_by_level.insert(current_level, rules);
                }
            }

            current_level -= 1;

            // Get children of frequent nodes for next iteration
            current_nodes = self
                .repository
                .get_next_level_nodes(&current_nodes, current_level);
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
            let prev_itemsets: Vec<Itemset> = large_itemsets[&(k - 1)].keys().cloned().collect();
            let mut sorted_itemsets = prev_itemsets;
            sorted_itemsets.sort();

            // Join step
            let candidates = self.join_step(&sorted_itemsets);

            // Prune step
            let unpruned_candidates = self.prune_step(&large_itemsets[&(k - 1)], &candidates);

            // Calculate support for candidates
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

        for i in 0..n {
            let itemset_i = &itemsets[i];
            let prefix_i = &itemset_i[..itemset_i.len() - 1];
            let last_i = itemset_i[itemset_i.len() - 1];

            let mut tail_items = vec![last_i];

            for j in (i + 1)..n {
                let itemset_j = &itemsets[j];
                let prefix_j = &itemset_j[..itemset_j.len() - 1];
                let last_j = itemset_j[itemset_j.len() - 1];

                if prefix_i == prefix_j {
                    tail_items.push(last_j);
                } else {
                    break;
                }
            }

            // Generate candidates from tail items
            for a_idx in 0..tail_items.len() {
                for b_idx in (a_idx + 1)..tail_items.len() {
                    let mut candidate = prefix_i.to_vec();
                    candidate.push(tail_items[a_idx]);
                    candidate.push(tail_items[b_idx]);
                    candidates.push(candidate);
                }
            }
        }

        candidates
    }

    fn prune_step(
        &self,
        itemmap: &HashMap<Itemset, u32>,
        candidates: &Vec<Itemset>,
    ) -> Vec<Itemset> {
        let mut unpruned = Vec::new();

        for candidate in candidates {
            // Check all k-1 subsets
            let mut should_keep = true;
            for i in 0..((*candidate).len() - 2) {
                let subset = [&candidate[..i], &candidate[i + 1..]].concat();

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
        let first_node_id = self.repository.node_by_id.get(&itemset[0]);
        if first_node_id.is_none() {
            return 0;
        }

        let mut result = self.repository.arena[*first_node_id.unwrap()]
            .get()
            .bitmap
            .clone();

        // Intersect with other nodes
        for &item_id in &itemset[1..] {
            if let Some(&node_id) = self.repository.node_by_id.get(&item_id) {
                let node = self.repository.arena[node_id].get();
                result &= &node.bitmap;
            } else {
                return 0;
            }
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
                let mut h_1 = Vec::new();

                for i in 0..itemset.len() {
                    let rhs = vec![itemset[i]];
                    let mut lhs: Itemset = itemset
                        .iter()
                        .enumerate()
                        .filter(|(idx, _)| *idx != i)
                        .map(|(_, &val)| val)
                        .collect();
                    lhs.sort();

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
        let mut sorted_h_m: Vec<Itemset> = h_m.to_vec();
        for h in &mut sorted_h_m {
            h.sort();
        }
        sorted_h_m.sort();

        let h_m_next = self.join_step(&sorted_h_m);
        // Convert sorted_h_m to HashMap for prune_step (using dummy value since we only need keys)
        let h_m_map: HashMap<Itemset, u32> = sorted_h_m.iter().map(|h| (h.clone(), 0)).collect();
        let h_m_next = self.prune_step(&h_m_map, &h_m_next);

        if h_m_next.is_empty() {
            return;
        }

        let mut h_m_copy = Vec::new();

        for rhs in &h_m_next {
            let itemset_set: HashSet<u32> = itemset.iter().cloned().collect();
            let rhs_set: HashSet<u32> = rhs.iter().cloned().collect();
            let lhs_set: HashSet<u32> = itemset_set.difference(&rhs_set).cloned().collect();
            let mut lhs: Itemset = lhs_set.into_iter().collect();
            lhs.sort();

            let count_full = itemsets
                .get(&itemset.len())
                .and_then(|m| m.get(itemset))
                .cloned()
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
