# HQApriori Experimental Evaluation

## Overview

This document describes the experimental evaluation framework for HQApriori, a hierarchical multi-level association rule mining algorithm. The experiments are designed to evaluate performance, scalability, and the benefits of hierarchical processing compared to traditional flat Apriori approaches.

## Datasets

The experiments are conducted on two Instacart grocery datasets:

1. **instacart_trainFIM.txt** - Training dataset with instacart_taxonomy.txt
2. **instacart_priorFIM.txt** - Prior orders dataset with instacart_taxonomy.txt

Both datasets use the same hierarchical taxonomy (NUM_LEVELS = 3) representing product categories at different abstraction levels.

## Default Multi-Level Configuration

For all experiments (except 3.2), the following default multi-level configuration is used:

```rust
let mut configs = HashMap::new();
configs.insert(1, LevelConfig {
    support: 0.5,
    confidence: 0.2,
    gen_rules: false/true,
});
configs.insert(2, LevelConfig {
    support: 0.08,
    confidence: 0.2,
    gen_rules: false/true,
});
configs.insert(3, LevelConfig {
    support: 0.005,
    confidence: 0.2,
    gen_rules: true,
});
```

This configuration is modified per experiment as specified below.

## Experiment Categories

### 1. Parameter Sensitivity Analysis

**Objective**: Evaluate the impact of support and confidence thresholds on execution time and rule generation across different taxonomy levels.

**Methodology**:
- For each level, vary support thresholds around the default value (5 values per level):
  - **Level 1** (default=0.5): [0.3, 0.4, 0.5, 0.6, 0.7]
  - **Level 2** (default=0.08): [0.04, 0.06, 0.08, 0.10, 0.12]
  - **Level 3** (default=0.005): [0.001, 0.003, 0.005, 0.007, 0.01]
- Vary confidence thresholds (common for all levels): [0.1, 0.2, 0.5, 0.7, 0.9]
- Test all combinations (5 support × 5 confidence = 25 configurations per level)
- Run multi-level mining with all 3 levels configured (only level 3 generates rules)
- Measure for each configuration:
  - Total execution time (ms)
  - Itemset generation time per level (ms)
  - Rule generation time (ms)
  - Number of frequent itemsets per level
  - Number of association rules generated (level 3 only)

**Output Structure**:
```json
{
  "experiment_type": "parameter_sensitivity",
  "dataset": "instacart_trainFIM.txt",
  "level_tested": 3,
  "level_configs": {
    "1": {"support": 0.5, "confidence": 0.2, "gen_rules": false},
    "2": {"support": 0.08, "confidence": 0.2, "gen_rules": false},
    "3": {"support": 0.005, "confidence": 0.2, "gen_rules": true}
  },
  "results": {
    "total_time_ms": 1234,
    "itemset_gen_time_ms": {"1": 200, "2": 400, "3": 200},
    "rule_gen_time_ms": 434,
    "num_frequent_itemsets": {"1": 500, "2": 800, "3": 1500},
    "num_rules": 3200
  }
}
```

### 2. Scalability Testing

**Objective**: Evaluate how the algorithm scales with increasing data volume and attribute space.

#### 2.1 Record Scalability

**Methodology**:
- Multiply transaction count by factors: [1, 2, 4, 8]
- Implementation: 
  - Load Repository once from original files
  - For each multiplier, use `Apriori::from_repository()` with extended bitmaps
  - Extend bitmaps by duplicating transaction IDs with offset:
    - Original: transactions 0-N
    - 2x: transactions 0-N, N+1-2N (duplicated with offset)
    - 4x: quadruple the bitmap
    - 8x: octuple the bitmap
- Use default multi-level configuration (all 3 levels)
- Measure execution time vs data size

**Rationale**: By extending bitmaps rather than duplicating files, we simulate larger datasets while maintaining the same distribution of itemsets, allowing us to isolate the impact of data volume on computational performance.

**Output Structure**:
```json
{
  "experiment_type": "record_scalability",
  "dataset": "instacart_trainFIM.txt",
  "multiplier": 4,
  "original_transactions": 50000,
  "scaled_transactions": 200000,
  "level_configs": {
    "1": {"support": 0.5, "confidence": 0.2, "gen_rules": false},
    "2": {"support": 0.08, "confidence": 0.2, "gen_rules": false},
    "3": {"support": 0.005, "confidence": 0.2, "gen_rules": true}
  },
  "results": {
    "total_time_ms": 4500,
    "itemset_gen_time_ms": {"1": 800, "2": 1500, "3": 900},
    "rule_gen_time_ms": 1300,
    "num_rules": 3200
  }
}
```

#### 2.2 Attribute Scalability

**Methodology**:
- Add synthetic attributes to the taxonomy at the leaf level
- Create duplicate subtrees with modified names (e.g., "category_1", "category_2")
- Multipliers: [1, 1.5, 2] (attribute space expansion)
- Assignment strategy for new attributes:
  - For each transaction, randomly draw 5 items from the original itemset
  - Go over each of the five drawn attributes individually
  - Go over each transaction of an attribute
  - For each transaction form bitmap, with probability 0.2, decide whether to include it in the new attribute
  - This maintains sparsity while introducing synthetic attributes
- Implementation:
  - Load Repository once with expanded taxonomy
  - Use `Apriori::from_repository()` with modified transaction bitmaps
- Use default multi-level configuration (all 3 levels)
- Measure impact on execution time and memory usage

**Note**: This tests the algorithm's ability to handle larger attribute spaces while maintaining the hierarchical structure and realistic transaction patterns.

### 3. Hierarchical vs Flat Apriori Comparison

**Objective**: Demonstrate the computational advantages of hierarchical multi-level mining over running flat Apriori multiple times.

#### 3.1 Multi-Pass vs Single-Pass Execution

**Methodology**:
- **Baseline (Multi-Pass)**: Run Apriori 3 times independently
  - Pass 1: Level 1 with support=0.5, confidence=0.2, gen_rules=true
  - Pass 2: Level 2 with support=0.08, confidence=0.2, gen_rules=true
  - Pass 3: Level 3 with support=0.005, confidence=0.2, gen_rules=true
  - Each pass mines only that specific level
  - Total time = sum of all three passes
  
- **HQApriori (Single-Pass)**: Run once with all three LevelConfigs (using default configuration)
  - Single execution with hierarchy-aware pruning
  - Reuses itemsets across levels
  - All levels generate rules (gen_rules=true for all)
  
- Compare total execution times and rule counts

**Output Structure**:
```json
{
  "experiment_type": "multi_pass_comparison",
  "dataset": "instacart_priorFIM.txt",
  "level_configs": {
    "1": {"support": 0.5, "confidence": 0.2},
    "2": {"support": 0.08, "confidence": 0.2},
    "3": {"support": 0.005, "confidence": 0.2}
  },
  "results": {
    "multi_pass": {
      "pass_1_time_ms": 2000,
      "pass_2_time_ms": 3500,
      "pass_3_time_ms": 5000,
      "total_time_ms": 10500,
      "total_rules": 5000,
      "rules_by_level": {"1": 1500, "2": 2000, "3": 1500}
    },
    "single_pass": {
      "total_time_ms": 6800,
      "total_rules": 5000,
      "rules_by_level": {"1": 1500, "2": 2000, "3": 1500},
      "time_saved_pct": 35.2
    }
  }
}
```

#### 3.2 Support Threshold Sensitivity for Hierarchical Pruning

**Objective**: Evaluate the impact of support thresholds at higher taxonomy levels (1 and 2) on computation time and rule generation at the target level (3), to understand the pruning effectiveness of the hierarchical approach.

**Methodology**:
- **Configuration**: Generate rules only at level 3 (gen_rules=true), levels 1-2 have gen_rules=false
- **Fixed Parameters**:
  - Level 3 support: 0.005 (default)
  - Level 3 confidence: 0.2 (default)
- **Randomized Parameters** (100 iterations):
  - Level 1 support: Uniform random from [0.005, 0.8]
  - Level 2 support: Uniform random from [0.005, level_1_support]
  - Level 1 confidence: 0.2 (fixed)
  - Level 2 confidence: 0.2 (fixed)
  
- For each iteration, measure:
  - Total execution time (ms)
  - Itemset generation time per level (ms)
  - Rule generation time (ms)
  - Number of frequent itemsets per level
  - Number of association rules at level 3

**Rationale**: Higher support thresholds at upper levels prune the search space more aggressively, potentially speeding up computation but also affecting the number of itemsets and rules discovered at lower levels. This experiment provides data for scatter plot analysis of these trade-offs.

**Output Structure** (single JSON file with 100 results):
```json
{
  "experiment_type": "support_threshold_sensitivity",
  "dataset": "instacart_trainFIM.txt",
  "num_iterations": 100,
  "fixed_params": {
    "level_3_support": 0.005,
    "confidence": 0.2,
    "gen_rules_config": {"1": false, "2": false, "3": true}
  },
  "results": [
    {
      "iteration": 1,
      "level_1_support": 0.234,
      "level_2_support": 0.089,
      "total_time_ms": 5430,
      "itemset_gen_time_ms": {"1": 1200, "2": 2100, "3": 1500},
      "rule_gen_time_ms": 630,
      "num_frequent_itemsets": {"1": 450, "2": 1200, "3": 2800},
      "num_rules": 4500
    },
    {
      "iteration": 2,
      "level_1_support": 0.678,
      "level_2_support": 0.145,
      "total_time_ms": 3210,
      "itemset_gen_time_ms": {"1": 800, "2": 1300, "3": 900},
      "rule_gen_time_ms": 210,
      "num_frequent_itemsets": {"1": 120, "2": 450, "3": 1200},
      "num_rules": 1800
    }
  ]
}
```

## Implementation Details

### Timing Measurements

All timing measurements use high-precision system timers with millisecond granularity. Key measurement points:

1. **Total Execution Time**: From mine() start to completion
2. **Itemset Generation Time**: Time spent in generate_itemsets() per level
3. **Rule Generation Time**: Time spent in generate_rules() per level

### Bitmap Extension for Scalability Testing

To simulate larger datasets without creating duplicate files, experiments 2.1 and 2.2 use the following approach:

1. Load the Repository once from the original files
2. Create modified copies of the Repository with extended/modified bitmaps
3. Use `Apriori::from_repository()` to run mining on the modified repository

For record scalability (2.1), bitmaps are extended by duplicating and offsetting transaction IDs:
- Multiplier = 1: Original dataset
- Multiplier = 2: Each transaction appears twice (original ID + offset)
- Multiplier = 4: Each transaction appears 4 times
- Multiplier = 8: Each transaction appears 8 times

For attribute scalability (2.2), the taxonomy is expanded with synthetic attributes and transaction bitmaps are selectively modified according to the assignment strategy.

### Result Serialization

All experiment results are serialized to JSON format using serde_json. Results are organized in a directory structure:

```
experiments/
├── parameter_sensitivity/
│   ├── instacart_train_level1_s0.3_c0.1.json
│   ├── instacart_train_level1_s0.3_c0.2.json
│   └── ...
├── scalability_records/
│   ├── instacart_train_mult1.json
│   ├── instacart_train_mult2.json
│   └── ...
├── scalability_attributes/
│   ├── instacart_train_mult1.0.json
│   ├── instacart_train_mult1.5.json
│   └── ...
├── multi_pass_comparison/
│   ├── instacart_train.json
│   └── instacart_prior.json
└── support_threshold_sensitivity/
    ├── instacart_train.json  (contains all 100 iterations)
    └── instacart_prior.json  (contains all 100 iterations)
```

## Expected Outcomes

1. **Parameter Sensitivity**: Lower support/confidence thresholds should exponentially increase execution time and rule count, with diminishing returns at very low thresholds. The effect should be most pronounced at level 3 where the most itemsets are generated.

2. **Record Scalability**: Execution time should scale linearly or sub-linearly with data volume due to efficient bitmap operations (RoaringBitmap compression).

3. **Attribute Scalability**: Execution time should scale with the number of attributes, but the sparse transaction modification strategy should maintain reasonable performance.

4. **Multi-Pass Comparison**: HQApriori should demonstrate 30-50% time savings compared to multiple independent Apriori runs, due to hierarchy-aware pruning and itemset reuse.

5. **Support Threshold Sensitivity**: Higher support thresholds at levels 1 and 2 should reduce computation time by pruning the search space more aggressively. However, overly aggressive pruning may reduce the number of rules discovered at level 3. Scatter plot analysis should reveal the optimal balance point.

## Reproducibility

All experiments can be reproduced by running:

```bash
cargo run --release --bin experiments
```

Random seeds are fixed for the following experiments to ensure reproducible results:
- Attribute scalability tests (2.2)
- Support threshold sensitivity (3.2) - 100 iterations with fixed seed

Configuration parameters are embedded in output JSON files for full traceability.
