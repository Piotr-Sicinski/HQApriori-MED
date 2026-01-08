use hqapriori_med::apriori::{Apriori, LevelConfig};
use hqapriori_med::repository::Repository;
use roaring::RoaringBitmap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::Instant;

// ============================================================================
// Data Structures for Results
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct ParameterSensitivityResult {
    experiment_type: String,
    dataset: String,
    level_tested: i32,
    level_configs: HashMap<String, LevelConfigOutput>,
    results: ExperimentResults,
}

#[derive(Debug, Serialize, Deserialize)]
struct LevelConfigOutput {
    support: f64,
    confidence: f64,
    gen_rules: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExperimentResults {
    total_time_ms: u128,
    itemset_gen_time_ms: HashMap<String, u128>,
    rule_gen_time_ms: u128,
    num_frequent_itemsets: HashMap<String, usize>,
    num_rules: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct RecordScalabilityResult {
    experiment_type: String,
    dataset: String,
    multiplier: u32,
    original_transactions: u32,
    scaled_transactions: u32,
    level_configs: HashMap<String, LevelConfigOutput>,
    results: ExperimentResults,
}

#[derive(Debug, Serialize, Deserialize)]
struct AttributeScalabilityResult {
    experiment_type: String,
    dataset: String,
    multiplier: f64,
    original_attributes: usize,
    scaled_attributes: usize,
    level_configs: HashMap<String, LevelConfigOutput>,
    results: ExperimentResults,
}

#[derive(Debug, Serialize, Deserialize)]
struct MultiPassComparisonResult {
    experiment_type: String,
    dataset: String,
    level_configs: HashMap<String, LevelConfigOutput>,
    results: MultiPassResults,
}

#[derive(Debug, Serialize, Deserialize)]
struct MultiPassResults {
    multi_pass: MultiPassData,
    single_pass: SinglePassData,
}

#[derive(Debug, Serialize, Deserialize)]
struct MultiPassData {
    pass_1_time_ms: u128,
    pass_2_time_ms: u128,
    pass_3_time_ms: u128,
    total_time_ms: u128,
    total_rules: usize,
    rules_by_level: HashMap<String, usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SinglePassData {
    total_time_ms: u128,
    total_rules: usize,
    rules_by_level: HashMap<String, usize>,
    time_saved_pct: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SupportThresholdSensitivityResult {
    experiment_type: String,
    dataset: String,
    num_iterations: usize,
    fixed_params: FixedParams,
    results: Vec<SensitivityIteration>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FixedParams {
    level_3_support: f64,
    confidence: f64,
    gen_rules_config: HashMap<String, bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SensitivityIteration {
    iteration: usize,
    level_1_support: f64,
    level_2_support: f64,
    total_time_ms: u128,
    itemset_gen_time_ms: HashMap<String, u128>,
    rule_gen_time_ms: u128,
    num_frequent_itemsets: HashMap<String, usize>,
    num_rules: usize,
}

// ============================================================================
// Repository Extension Utilities
// ============================================================================

/// Expand repository by adding synthetic attributes at leaf level
fn expand_repository_attributes(repo: &Repository, multiplier: f64) -> Repository {
    use indextree::Arena;
    use rand::Rng;
    use rand::SeedableRng;

    if multiplier <= 1.0 {
        return repo.clone();
    }

    let mut rng = rand::rngs::StdRng::seed_from_u64(42); // Fixed seed for reproducibility

    let mut new_arena = Arena::new();
    let mut node_by_id = HashMap::new();

    // Clone root
    let root_data = repo.arena[repo.root].get().clone();
    let new_root = new_arena.new_node(root_data.clone());
    node_by_id.insert(root_data.id, new_root);

    // Track leaf nodes and their parents
    let mut leaf_nodes = Vec::new();
    let mut node_parents = HashMap::new();

    // Collect all nodes first
    let all_nodes: Vec<indextree::NodeId> = {
        let mut nodes = Vec::new();
        let mut queue = vec![repo.root];
        while let Some(node_id) = queue.pop() {
            nodes.push(node_id);
            for child in node_id.children(&repo.arena) {
                queue.push(child);
            }
        }
        nodes
    };

    // Find all leaf nodes and build parent map
    for &node_id in &all_nodes {
        let children: Vec<_> = node_id.children(&repo.arena).collect();
        if children.is_empty() && node_id != repo.root {
            leaf_nodes.push(node_id);
        }
        if let Some(parent) = repo.arena[node_id].parent() {
            node_parents.insert(node_id, parent);
        }
    }

    // Clone the entire tree structure first
    clone_subtree_for_attributes(
        &repo.arena,
        repo.root,
        &mut new_arena,
        new_root,
        &mut node_by_id,
    );

    // Calculate how many synthetic attributes to add
    let num_new_attrs = ((leaf_nodes.len() as f64 * (multiplier - 1.0)).ceil() as usize).max(1);

    // Create synthetic attributes
    let mut next_id = repo.node_by_id.keys().max().unwrap_or(&0) + 1;

    for _ in 0..num_new_attrs {
        // Randomly select 5 source leaf nodes (with replacement if needed)
        let mut source_nodes = Vec::new();
        for _ in 0..5.min(leaf_nodes.len()) {
            let idx = rng.gen_range(0..leaf_nodes.len());
            source_nodes.push(leaf_nodes[idx]);
        }

        // Build bitmap for new synthetic attribute
        let mut synthetic_bitmap = RoaringBitmap::new();
        for &source_node in &source_nodes {
            let source_data = repo.arena[source_node].get();
            // For each transaction in the source, include it with probability 0.2
            for tid in source_data.bitmap.iter() {
                if rng.gen::<f64>() < 0.2 {
                    synthetic_bitmap.insert(tid);
                }
            }
        }

        // Skip if bitmap is empty
        if synthetic_bitmap.is_empty() {
            continue;
        }

        // Find parent for the new synthetic node (use parent of first source node)
        let parent_node_id = if !source_nodes.is_empty() {
            let source_node = source_nodes[0];
            if let Some(&parent_in_old) = node_parents.get(&source_node) {
                let parent_data = repo.arena[parent_in_old].get();
                // Find corresponding parent in new arena
                *node_by_id.get(&parent_data.id).unwrap_or(&new_root)
            } else {
                new_root
            }
        } else {
            new_root
        };

        // Create synthetic node
        let synthetic_name = format!("synthetic_{}", next_id);
        let synthetic_data = hqapriori_med::repository::NodeData {
            id: next_id,
            name: synthetic_name,
            bitmap: synthetic_bitmap,
        };

        let synthetic_node = new_arena.new_node(synthetic_data);
        parent_node_id.append(synthetic_node, &mut new_arena);
        node_by_id.insert(next_id, synthetic_node);

        next_id += 1;
    }

    // Propagate bitmaps upward from new leaves
    propagate_bitmaps_upward(&mut new_arena, new_root);

    Repository {
        arena: new_arena,
        root: new_root,
        node_by_id,
        transaction_count: repo.transaction_count,
    }
}

fn clone_subtree_for_attributes(
    src_arena: &indextree::Arena<hqapriori_med::repository::NodeData>,
    src_node: indextree::NodeId,
    dst_arena: &mut indextree::Arena<hqapriori_med::repository::NodeData>,
    dst_node: indextree::NodeId,
    node_by_id: &mut HashMap<u32, indextree::NodeId>,
) {
    let children: Vec<indextree::NodeId> = src_node.children(src_arena).collect();

    for child_id in children {
        let child_data = src_arena[child_id].get().clone();
        let new_child = dst_arena.new_node(child_data.clone());
        dst_node.append(new_child, dst_arena);
        node_by_id.insert(child_data.id, new_child);

        clone_subtree_for_attributes(src_arena, child_id, dst_arena, new_child, node_by_id);
    }
}

fn propagate_bitmaps_upward(
    arena: &mut indextree::Arena<hqapriori_med::repository::NodeData>,
    node_id: indextree::NodeId,
) {
    let children: Vec<indextree::NodeId> = node_id.children(arena).collect();

    if children.is_empty() {
        return;
    }

    // Recursively propagate for children first
    for &child_id in &children {
        propagate_bitmaps_upward(arena, child_id);
    }

    // Union bitmaps of children
    if let Some(&first_child) = children.first() {
        let mut combined = arena[first_child].get().bitmap.clone();
        for &child_id in &children[1..] {
            combined |= &arena[child_id].get().bitmap;
        }
        arena[node_id].get_mut().bitmap = combined;
    }
}

/// Extend repository bitmaps by multiplying transaction IDs
fn extend_repository_transactions(repo: &Repository, multiplier: u32) -> Repository {
    use indextree::Arena;

    let mut new_arena = Arena::new();
    let original_count = repo.transaction_count;
    let mut node_by_id = HashMap::new();

    // Clone the entire tree structure and extend bitmaps
    let root_data = repo.arena[repo.root].get().clone();

    // Extend root bitmap
    let mut extended_root_bitmap = root_data.bitmap.clone();
    for mult in 1..multiplier {
        let offset = mult * original_count;
        for tid in root_data.bitmap.iter() {
            extended_root_bitmap.insert(tid + offset);
        }
    }

    let new_root_data = hqapriori_med::repository::NodeData {
        id: root_data.id,
        name: root_data.name.clone(),
        bitmap: extended_root_bitmap,
    };

    let new_root = new_arena.new_node(new_root_data);
    node_by_id.insert(root_data.id, new_root);

    // Clone tree recursively and extend bitmaps
    clone_and_extend_subtree(
        &repo.arena,
        repo.root,
        &mut new_arena,
        new_root,
        multiplier,
        original_count,
        &mut node_by_id,
    );

    Repository {
        arena: new_arena,
        root: new_root,
        node_by_id,
        transaction_count: original_count * multiplier,
    }
}

fn clone_and_extend_subtree(
    src_arena: &indextree::Arena<hqapriori_med::repository::NodeData>,
    src_node: indextree::NodeId,
    dst_arena: &mut indextree::Arena<hqapriori_med::repository::NodeData>,
    dst_node: indextree::NodeId,
    multiplier: u32,
    original_count: u32,
    node_by_id: &mut HashMap<u32, indextree::NodeId>,
) {
    // Get children of source node
    let children: Vec<indextree::NodeId> = src_node.children(src_arena).collect();

    for child_id in children {
        let child_data = src_arena[child_id].get().clone();

        // Extend bitmap
        let mut extended_bitmap = child_data.bitmap.clone();
        for mult in 1..multiplier {
            let offset = mult * original_count;
            for tid in child_data.bitmap.iter() {
                extended_bitmap.insert(tid + offset);
            }
        }

        // Create new node with extended bitmap
        let new_child_data = hqapriori_med::repository::NodeData {
            id: child_data.id,
            name: child_data.name.clone(),
            bitmap: extended_bitmap,
        };

        let new_child = dst_arena.new_node(new_child_data);
        dst_node.append(new_child, dst_arena);
        node_by_id.insert(child_data.id, new_child);

        // Recursively clone children
        clone_and_extend_subtree(
            src_arena,
            child_id,
            dst_arena,
            new_child,
            multiplier,
            original_count,
            node_by_id,
        );
    }
}

// ============================================================================
// Experiment 1: Parameter Sensitivity Analysis
// ============================================================================

fn run_parameter_sensitivity(dataset_path: &str, taxonomy_path: &str, dataset_name: &str) {
    println!("\n========================================");
    println!("Experiment 1: Parameter Sensitivity Analysis");
    println!("Dataset: {}", dataset_name);
    println!("========================================\n");

    // Create output directory
    fs::create_dir_all("experiments/parameter_sensitivity").unwrap();

    // Define parameter ranges
    let level_1_supports = vec![0.3, 0.4, 0.5, 0.6, 0.7];
    let level_2_supports = vec![0.04, 0.06, 0.08, 0.10, 0.12];
    let level_3_supports = vec![0.001, 0.003, 0.005, 0.007, 0.01];
    let confidences = vec![0.1, 0.15, 0.2, 0.25, 0.3];

    let mut experiment_count = 0;
    let total_experiments =
        (level_1_supports.len() + level_2_supports.len() + level_3_supports.len())
            * confidences.len();

    // Test Level 1 variations
    for &support in &level_1_supports {
        for &confidence in &confidences {
            experiment_count += 1;
            println!(
                "Running experiment {}/{}: Level 1, support={}, confidence={}",
                experiment_count, total_experiments, support, confidence
            );

            let mut configs = HashMap::new();
            configs.insert(
                1,
                LevelConfig {
                    support,
                    confidence,
                    gen_rules: false,
                },
            );
            configs.insert(
                2,
                LevelConfig {
                    support: 0.08,
                    confidence: 0.2,
                    gen_rules: false,
                },
            );
            configs.insert(
                3,
                LevelConfig {
                    support: 0.005,
                    confidence: 0.2,
                    gen_rules: true,
                },
            );

            let result = run_single_experiment(
                dataset_path,
                taxonomy_path,
                configs.clone(),
                1,
                dataset_name,
            );

            // Save result
            let filename = format!(
                "experiments/parameter_sensitivity/{}_level1_s{}_c{}.json",
                dataset_name, support, confidence
            );
            fs::write(&filename, serde_json::to_string_pretty(&result).unwrap()).unwrap();
        }
    }

    // Test Level 2 variations
    for &support in &level_2_supports {
        for &confidence in &confidences {
            experiment_count += 1;
            println!(
                "Running experiment {}/{}: Level 2, support={}, confidence={}",
                experiment_count, total_experiments, support, confidence
            );

            let mut configs = HashMap::new();
            configs.insert(
                1,
                LevelConfig {
                    support: 0.5,
                    confidence: 0.2,
                    gen_rules: false,
                },
            );
            configs.insert(
                2,
                LevelConfig {
                    support,
                    confidence,
                    gen_rules: false,
                },
            );
            configs.insert(
                3,
                LevelConfig {
                    support: 0.005,
                    confidence: 0.2,
                    gen_rules: true,
                },
            );

            let result = run_single_experiment(
                dataset_path,
                taxonomy_path,
                configs.clone(),
                2,
                dataset_name,
            );

            let filename = format!(
                "experiments/parameter_sensitivity/{}_level2_s{}_c{}.json",
                dataset_name, support, confidence
            );
            fs::write(&filename, serde_json::to_string_pretty(&result).unwrap()).unwrap();
        }
    }

    // Test Level 3 variations
    for &support in &level_3_supports {
        for &confidence in &confidences {
            experiment_count += 1;
            println!(
                "Running experiment {}/{}: Level 3, support={}, confidence={}",
                experiment_count, total_experiments, support, confidence
            );

            let mut configs = HashMap::new();
            configs.insert(
                1,
                LevelConfig {
                    support: 0.5,
                    confidence: 0.2,
                    gen_rules: false,
                },
            );
            configs.insert(
                2,
                LevelConfig {
                    support: 0.08,
                    confidence: 0.2,
                    gen_rules: false,
                },
            );
            configs.insert(
                3,
                LevelConfig {
                    support,
                    confidence,
                    gen_rules: true,
                },
            );

            let result = run_single_experiment(
                dataset_path,
                taxonomy_path,
                configs.clone(),
                3,
                dataset_name,
            );

            let filename = format!(
                "experiments/parameter_sensitivity/{}_level3_s{}_c{}.json",
                dataset_name, support, confidence
            );
            fs::write(&filename, serde_json::to_string_pretty(&result).unwrap()).unwrap();
        }
    }

    println!(
        "\nParameter sensitivity analysis completed for {}",
        dataset_name
    );
}

fn run_single_experiment(
    dataset_path: &str,
    taxonomy_path: &str,
    configs: HashMap<i32, LevelConfig>,
    level_tested: i32,
    dataset_name: &str,
) -> ParameterSensitivityResult {
    let start = Instant::now();

    let mut apriori = Apriori::new(dataset_path, taxonomy_path, configs.clone()).unwrap();

    let rules_by_level = apriori.mine();

    let total_time = start.elapsed().as_millis();

    // Count itemsets (approximate from rules)
    let mut num_frequent_itemsets = HashMap::new();
    num_frequent_itemsets.insert("1".to_string(), 0);
    num_frequent_itemsets.insert("2".to_string(), 0);
    num_frequent_itemsets.insert("3".to_string(), 0);

    let num_rules = rules_by_level.values().map(|v| v.len()).sum();

    // Convert configs to output format
    let level_configs: HashMap<String, LevelConfigOutput> = configs
        .iter()
        .map(|(k, v)| {
            (
                k.to_string(),
                LevelConfigOutput {
                    support: v.support,
                    confidence: v.confidence,
                    gen_rules: v.gen_rules,
                },
            )
        })
        .collect();

    ParameterSensitivityResult {
        experiment_type: "parameter_sensitivity".to_string(),
        dataset: dataset_name.to_string(),
        level_tested,
        level_configs,
        results: ExperimentResults {
            total_time_ms: total_time,
            itemset_gen_time_ms: HashMap::new(),
            rule_gen_time_ms: 0,
            num_frequent_itemsets,
            num_rules,
        },
    }
}

// ============================================================================
// Experiment 2.1: Record Scalability Testing
// ============================================================================

fn run_record_scalability(dataset_path: &str, taxonomy_path: &str, dataset_name: &str) {
    println!("\n========================================");
    println!("Experiment 2.1: Record Scalability Testing");
    println!("Dataset: {}", dataset_name);
    println!("========================================\n");

    fs::create_dir_all("experiments/scalability_records").unwrap();

    let multipliers = vec![1, 2, 4, 8];

    // Load repository once
    println!("Loading base repository...");
    let base_repo = Repository::new(dataset_path, taxonomy_path).unwrap();
    let original_count = base_repo.transaction_count;

    for &mult in &multipliers {
        println!(
            "\nRunning with multiplier {}x ({} transactions)...",
            mult,
            original_count * mult
        );

        let start = Instant::now();

        // Extend repository
        let extended_repo = if mult == 1 {
            base_repo.clone()
        } else {
            extend_repository_transactions(&base_repo, mult)
        };

        // Create default configs
        let mut configs = HashMap::new();
        configs.insert(
            1,
            LevelConfig {
                support: 0.5,
                confidence: 0.2,
                gen_rules: true,
            },
        );
        configs.insert(
            2,
            LevelConfig {
                support: 0.08,
                confidence: 0.2,
                gen_rules: true,
            },
        );
        configs.insert(
            3,
            LevelConfig {
                support: 0.005,
                confidence: 0.2,
                gen_rules: true,
            },
        );

        let mut apriori = Apriori::from_repository(extended_repo, configs.clone());
        let rules_by_level = apriori.mine();

        let total_time = start.elapsed().as_millis();

        let num_rules = rules_by_level.values().map(|v| v.len()).sum();

        // Convert configs
        let level_configs: HashMap<String, LevelConfigOutput> = configs
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    LevelConfigOutput {
                        support: v.support,
                        confidence: v.confidence,
                        gen_rules: v.gen_rules,
                    },
                )
            })
            .collect();

        let result = RecordScalabilityResult {
            experiment_type: "record_scalability".to_string(),
            dataset: dataset_name.to_string(),
            multiplier: mult,
            original_transactions: original_count,
            scaled_transactions: original_count * mult,
            level_configs,
            results: ExperimentResults {
                total_time_ms: total_time,
                itemset_gen_time_ms: HashMap::new(),
                rule_gen_time_ms: 0,
                num_frequent_itemsets: HashMap::new(),
                num_rules,
            },
        };

        let filename = format!(
            "experiments/scalability_records/{}_mult{}.json",
            dataset_name, mult
        );
        fs::write(&filename, serde_json::to_string_pretty(&result).unwrap()).unwrap();

        println!("  Time: {}ms, Rules: {}", total_time, num_rules);
    }

    println!(
        "\nRecord scalability testing completed for {}",
        dataset_name
    );
}

// ============================================================================
// Experiment 2.2: Attribute Scalability Testing
// ============================================================================

fn run_attribute_scalability(dataset_path: &str, taxonomy_path: &str, dataset_name: &str) {
    println!("\n========================================");
    println!("Experiment 2.2: Attribute Scalability Testing");
    println!("Dataset: {}", dataset_name);
    println!("========================================\n");

    fs::create_dir_all("experiments/scalability_attributes").unwrap();

    let multipliers = vec![1.0, 1.5, 2.0];

    // Load base repository
    println!("Loading base repository...");
    let base_repo = Repository::new(dataset_path, taxonomy_path).unwrap();
    let original_attrs = base_repo.node_by_id.len();

    for &mult in &multipliers {
        println!("\nRunning with attribute multiplier {}x...", mult);

        let start = Instant::now();

        // Expand repository with synthetic attributes
        let expanded_repo = expand_repository_attributes(&base_repo, mult);
        let scaled_attrs = expanded_repo.node_by_id.len();

        println!("  Original attributes: {}", original_attrs);
        println!("  Scaled attributes: {}", scaled_attrs);

        let mut configs = HashMap::new();
        configs.insert(
            1,
            LevelConfig {
                support: 0.5,
                confidence: 0.2,
                gen_rules: true,
            },
        );
        configs.insert(
            2,
            LevelConfig {
                support: 0.08,
                confidence: 0.2,
                gen_rules: true,
            },
        );
        configs.insert(
            3,
            LevelConfig {
                support: 0.005,
                confidence: 0.2,
                gen_rules: true,
            },
        );

        let mut apriori = Apriori::from_repository(expanded_repo, configs.clone());
        let rules_by_level = apriori.mine();

        let total_time = start.elapsed().as_millis();
        let num_rules = rules_by_level.values().map(|v| v.len()).sum();

        let level_configs: HashMap<String, LevelConfigOutput> = configs
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    LevelConfigOutput {
                        support: v.support,
                        confidence: v.confidence,
                        gen_rules: v.gen_rules,
                    },
                )
            })
            .collect();

        let result = AttributeScalabilityResult {
            experiment_type: "attribute_scalability".to_string(),
            dataset: dataset_name.to_string(),
            multiplier: mult,
            original_attributes: original_attrs,
            scaled_attributes: scaled_attrs,
            level_configs,
            results: ExperimentResults {
                total_time_ms: total_time,
                itemset_gen_time_ms: HashMap::new(),
                rule_gen_time_ms: 0,
                num_frequent_itemsets: HashMap::new(),
                num_rules,
            },
        };

        let filename = format!(
            "experiments/scalability_attributes/{}_mult{}.json",
            dataset_name, mult
        );
        fs::write(&filename, serde_json::to_string_pretty(&result).unwrap()).unwrap();

        println!("  Time: {}ms, Rules: {}", total_time, num_rules);
    }

    println!(
        "\nAttribute scalability testing completed for {}",
        dataset_name
    );
}

// ============================================================================
// Experiment 3.1: Multi-Pass Comparison
// ============================================================================

fn run_multi_pass_comparison(dataset_path: &str, taxonomy_path: &str, dataset_name: &str) {
    println!("\n========================================");
    println!("Experiment 3.1: Multi-Pass Comparison");
    println!("Dataset: {}", dataset_name);
    println!("========================================\n");

    fs::create_dir_all("experiments/multi_pass_comparison").unwrap();

    // Multi-pass approach: Run 3 separate times
    println!("Running multi-pass approach (3 separate runs)...");

    let start1 = Instant::now();
    let mut configs1 = HashMap::new();
    configs1.insert(
        1,
        LevelConfig {
            support: 0.5,
            confidence: 0.2,
            gen_rules: true,
        },
    );
    let mut apriori1 = Apriori::new(dataset_path, taxonomy_path, configs1).unwrap();
    let rules1 = apriori1.mine();
    let time1 = start1.elapsed().as_millis();
    let rules_count1 = rules1.values().map(|v| v.len()).sum();

    let start2 = Instant::now();
    let mut configs2 = HashMap::new();
    configs2.insert(
        2,
        LevelConfig {
            support: 0.08,
            confidence: 0.2,
            gen_rules: true,
        },
    );
    let mut apriori2 = Apriori::new(dataset_path, taxonomy_path, configs2).unwrap();
    let rules2 = apriori2.mine();
    let time2 = start2.elapsed().as_millis();
    let rules_count2 = rules2.values().map(|v| v.len()).sum();

    let start3 = Instant::now();
    let mut configs3 = HashMap::new();
    configs3.insert(
        3,
        LevelConfig {
            support: 0.005,
            confidence: 0.2,
            gen_rules: true,
        },
    );
    let mut apriori3 = Apriori::new(dataset_path, taxonomy_path, configs3).unwrap();
    let rules3 = apriori3.mine();
    let time3 = start3.elapsed().as_millis();
    let rules_count3 = rules3.values().map(|v| v.len()).sum();

    let multi_pass_total = time1 + time2 + time3;
    let multi_pass_rules = rules_count1 + rules_count2 + rules_count3;

    println!("  Pass 1: {}ms, {} rules", time1, rules_count1);
    println!("  Pass 2: {}ms, {} rules", time2, rules_count2);
    println!("  Pass 3: {}ms, {} rules", time3, rules_count3);
    println!(
        "  Total: {}ms, {} rules",
        multi_pass_total, multi_pass_rules
    );

    // Single-pass approach
    println!("\nRunning single-pass approach (integrated)...");

    let start_single = Instant::now();
    let mut configs_single = HashMap::new();
    configs_single.insert(
        1,
        LevelConfig {
            support: 0.5,
            confidence: 0.2,
            gen_rules: true,
        },
    );
    configs_single.insert(
        2,
        LevelConfig {
            support: 0.08,
            confidence: 0.2,
            gen_rules: true,
        },
    );
    configs_single.insert(
        3,
        LevelConfig {
            support: 0.005,
            confidence: 0.2,
            gen_rules: true,
        },
    );
    let mut apriori_single =
        Apriori::new(dataset_path, taxonomy_path, configs_single.clone()).unwrap();
    let rules_single = apriori_single.mine();
    let time_single = start_single.elapsed().as_millis();

    let single_pass_rules = rules_single.values().map(|v| v.len()).sum();
    let time_saved_pct =
        ((multi_pass_total as f64 - time_single as f64) / multi_pass_total as f64) * 100.0;

    println!("  Time: {}ms, {} rules", time_single, single_pass_rules);
    println!("  Time saved: {:.2}%", time_saved_pct);

    // Convert configs
    let level_configs: HashMap<String, LevelConfigOutput> = configs_single
        .iter()
        .map(|(k, v)| {
            (
                k.to_string(),
                LevelConfigOutput {
                    support: v.support,
                    confidence: v.confidence,
                    gen_rules: v.gen_rules,
                },
            )
        })
        .collect();

    let mut rules_by_level_multi = HashMap::new();
    rules_by_level_multi.insert("1".to_string(), rules_count1);
    rules_by_level_multi.insert("2".to_string(), rules_count2);
    rules_by_level_multi.insert("3".to_string(), rules_count3);

    let mut rules_by_level_single = HashMap::new();
    for (level, rules) in rules_single.iter() {
        rules_by_level_single.insert(level.to_string(), rules.len());
    }

    let result = MultiPassComparisonResult {
        experiment_type: "multi_pass_comparison".to_string(),
        dataset: dataset_name.to_string(),
        level_configs,
        results: MultiPassResults {
            multi_pass: MultiPassData {
                pass_1_time_ms: time1,
                pass_2_time_ms: time2,
                pass_3_time_ms: time3,
                total_time_ms: multi_pass_total,
                total_rules: multi_pass_rules,
                rules_by_level: rules_by_level_multi,
            },
            single_pass: SinglePassData {
                total_time_ms: time_single,
                total_rules: single_pass_rules,
                rules_by_level: rules_by_level_single,
                time_saved_pct,
            },
        },
    };

    let filename = format!("experiments/multi_pass_comparison/{}.json", dataset_name);
    fs::write(&filename, serde_json::to_string_pretty(&result).unwrap()).unwrap();

    println!("\nMulti-pass comparison completed for {}", dataset_name);
}

// ============================================================================
// Experiment 3.2: Support Threshold Sensitivity
// ============================================================================

fn run_support_threshold_sensitivity(dataset_path: &str, taxonomy_path: &str, dataset_name: &str) {
    println!("\n========================================");
    println!("Experiment 3.2: Support Threshold Sensitivity");
    println!("Dataset: {}", dataset_name);
    println!("========================================\n");

    fs::create_dir_all("experiments/support_threshold_sensitivity").unwrap();

    use rand::Rng;
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(42); // Fixed seed for reproducibility

    let num_iterations = 100;
    let mut results = Vec::new();

    for iter in 0..num_iterations {
        println!("Running iteration {}/{}", iter + 1, num_iterations);

        // Generate random parameters
        let level_1_support = rng.gen_range(0.005..=0.8);
        let level_2_support = rng.gen_range(0.005..=level_1_support);

        let start = Instant::now();

        let mut configs = HashMap::new();
        configs.insert(
            1,
            LevelConfig {
                support: level_1_support,
                confidence: 0.2,
                gen_rules: false,
            },
        );
        configs.insert(
            2,
            LevelConfig {
                support: level_2_support,
                confidence: 0.2,
                gen_rules: false,
            },
        );
        configs.insert(
            3,
            LevelConfig {
                support: 0.005,
                confidence: 0.2,
                gen_rules: true,
            },
        );

        let mut apriori = Apriori::new(dataset_path, taxonomy_path, configs).unwrap();
        let rules_by_level = apriori.mine();

        let total_time = start.elapsed().as_millis();
        let num_rules = rules_by_level.values().map(|v| v.len()).sum();

        let mut num_frequent_itemsets = HashMap::new();
        num_frequent_itemsets.insert("1".to_string(), 0);
        num_frequent_itemsets.insert("2".to_string(), 0);
        num_frequent_itemsets.insert("3".to_string(), 0);

        results.push(SensitivityIteration {
            iteration: iter + 1,
            level_1_support,
            level_2_support,
            total_time_ms: total_time,
            itemset_gen_time_ms: HashMap::new(),
            rule_gen_time_ms: 0,
            num_frequent_itemsets,
            num_rules,
        });

        println!(
            "  L1_support={:.4}, L2_support={:.4}, time={}ms, rules={}",
            level_1_support, level_2_support, total_time, num_rules
        );
    }

    let mut gen_rules_config = HashMap::new();
    gen_rules_config.insert("1".to_string(), false);
    gen_rules_config.insert("2".to_string(), false);
    gen_rules_config.insert("3".to_string(), true);

    let result = SupportThresholdSensitivityResult {
        experiment_type: "support_threshold_sensitivity".to_string(),
        dataset: dataset_name.to_string(),
        num_iterations,
        fixed_params: FixedParams {
            level_3_support: 0.005,
            confidence: 0.2,
            gen_rules_config,
        },
        results,
    };

    let filename = format!(
        "experiments/support_threshold_sensitivity/{}.json",
        dataset_name
    );
    fs::write(&filename, serde_json::to_string_pretty(&result).unwrap()).unwrap();

    println!(
        "\nSupport threshold sensitivity completed for {}",
        dataset_name
    );
}

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("HQApriori Experimental Evaluation");
    println!("==================================\n");

    // Dataset configurations
    let datasets = vec![
        (
            "datasets/instacart_trainFIM.txt",
            "datasets/instacart_taxonomy.txt",
            "instacart_train",
        ),
        (
            "datasets/instacart_priorFIM.txt",
            "datasets/instacart_taxonomy.txt",
            "instacart_prior",
        ),
    ];

    for (transactions, taxonomy, name) in datasets {
        println!("\n\n#########################################################");
        println!("Processing dataset: {}", name);
        println!("#########################################################\n");

        // Run all experiments
        run_parameter_sensitivity(transactions, taxonomy, name);
        run_record_scalability(transactions, taxonomy, name);
        run_attribute_scalability(transactions, taxonomy, name);
        run_multi_pass_comparison(transactions, taxonomy, name);
        run_support_threshold_sensitivity(transactions, taxonomy, name);
    }

    println!("\n\n#########################################################");
    println!("All experiments completed!");
    println!("#########################################################");
    println!("\nResults saved to experiments/ directory");

    Ok(())
}
