# Running Experiments

This document explains how to run the experimental evaluation for HQApriori.

## Prerequisites

1. Ensure you have the required datasets in the `datasets/` directory:
   - `instacart_trainFIM.txt`
   - `instacart_priorFIM.txt`
   - `instacart_taxonomy.txt`

2. Build the project:
   ```bash
   cargo build --release
   ```

## Running All Experiments

To run the complete experimental suite on both datasets:

```bash
cargo run --release --bin experiments
```

This will execute:
1. **Parameter Sensitivity Analysis** - Testing different support/confidence combinations across 3 levels
2. **Record Scalability Testing** - Testing with dataset multipliers (1x, 2x, 4x, 8x)
3. **Attribute Scalability Testing** - Testing with attribute multipliers (1x, 1.5x, 2x)
4. **Multi-Pass Comparison** - Comparing single-pass vs multi-pass execution
5. **Support Threshold Sensitivity** - 100 random configurations testing hierarchical pruning effectiveness

## Results

All results are saved as JSON files in the `experiments/` directory:

```
experiments/
├── parameter_sensitivity/
│   ├── instacart_train_level1_s0.3_c0.1.json
│   ├── instacart_train_level1_s0.3_c0.2.json
│   ├── instacart_train_level2_s0.04_c0.1.json
│   ├── instacart_train_level3_s0.001_c0.1.json
│   └── ... (75 files per dataset: 3 levels × 5 support × 5 confidence)
├── scalability_records/
│   ├── instacart_train_mult1.json
│   ├── instacart_train_mult2.json
│   ├── instacart_train_mult4.json
│   ├── instacart_train_mult8.json
│   └── ...
├── scalability_attributes/
│   ├── instacart_train_mult1.0.json
│   ├── instacart_train_mult1.5.json
│   ├── instacart_train_mult2.0.json
│   └── ...
├── multi_pass_comparison/
│   ├── instacart_train.json
│   └── instacart_prior.json
└── support_threshold_sensitivity/
    ├── instacart_train.json  (100 iterations)
    └── instacart_prior.json  (100 iterations)
```

## Running Individual Experiments

You can modify `src/experiments.rs` to run only specific experiments by commenting out the others in the `main()` function.

## Expected Runtime

- **Parameter Sensitivity**: ~1-2 hours per dataset (testing 75 configurations: 3 levels × 5 support × 5 confidence)
- **Record Scalability**: ~10-20 minutes per dataset (testing 4 multipliers)
- **Attribute Scalability**: ~15-25 minutes per dataset (testing 3 multipliers)
- **Multi-Pass Comparison**: ~10-15 minutes per dataset
- **Support Threshold Sensitivity**: ~2-3 hours per dataset (100 random iterations)

**Total estimated runtime**: 5-8 hours for both datasets

## Analyzing Results

Each JSON file contains structured results that can be loaded for analysis:

```python
import json
import pandas as pd

# Load parameter sensitivity results
results = []
for file in glob('experiments/parameter_sensitivity/*.json'):
    with open(file) as f:
        results.append(json.load(f))

df = pd.DataFrame(results)
print(df[['support', 'confidence', 'results.total_time_ms', 'results.num_rules']])
```

## Configuration

To modify experiment parameters, edit the constants in `src/experiments.rs`:

- `NUM_LEVELS`: Number of taxonomy levels (default: 3)
- Default configuration (hardcoded):
  - Level 1: support=0.5, confidence=0.2, gen_rules=false
  - Level 2: support=0.08, confidence=0.2, gen_rules=false
  - Level 3: support=0.005, confidence=0.2, gen_rules=true
- Support/confidence values in each experiment function
- Dataset multipliers for scalability testing
- Number of iterations for support threshold sensitivity (default: 100)
- Random seed for reproducibility (fixed)
