# Calculate Support Optimizations - Version 2

This document describes the optimizations made to `calculate_support` and related functions in `src/apriori.rs`.

## Problem
The `calculate_support` function was maxing out a single CPU core initially. After first optimizations, only half the cores were running at full speed.

## Root Cause Analysis
The "half cores at full speed" issue was caused by:
1. **Nested parallelism**: Using `par_iter` inside `par_iter` causing thread pool contention
2. **Poor work distribution**: Uneven chunk sizes leading to load imbalance
3. **Small work items**: Overhead of parallel dispatch outweighing actual work
4. **Memory contention**: Multiple threads competing for cache lines

## Optimizations Applied (v2)

### 1. **Bitmap Size Sorting** (Algorithmic Improvement)
- **Before**: Intersected bitmaps in the order items appeared in the itemset
- **After**: Sort bitmaps by size (smallest first) before intersection
- **Impact**: Reduces working set size faster, making subsequent intersections cheaper
- **Location**: Lines 340-372 in `calculate_support()`

```rust
// Collect all bitmaps and sort by size (smallest first)
bitmaps.sort_by_key(|b| b.len());
```

### 2. **Early Exit on Empty Results**
- **Added**: Check if result bitmap becomes empty during intersection
- **Impact**: Avoids unnecessary bitmap operations when result is already empty
- **Location**: Line 368 in `calculate_support()`

```rust
// Early exit if result becomes empty
if result.is_empty() {
    return 0;
}
```

### 3. **New Optimized Function with Min Support** 
- **Added**: `calculate_support_with_min()` function
- **Features**:
  - Early termination if any single bitmap is below min_support
  - Early exit during intersection if count drops below threshold
  - Returns `Option<u32>` to signal whether threshold was met
- **Impact**: Avoids completing calculations for itemsets that won't meet minimum support
- **Location**: Lines 376-430

### 4. **Parallel Support Calculation** (Multi-core Utilization)
- **Before**: Sequential loop calculating support for each candidate
- **After**: Parallel iteration using Rayon's `par_iter()`
- **Impact**: Utilizes multiple CPU cores instead of just one
- **Location**: Lines 233-240
- **Dependency Added**: `rayon = "1.10"` to Cargo.toml

```rust
let itemsets_k: HashMap<Itemset, u32> = unpruned_candidates
    .par_iter()
    .filter_map(|candidate| {
        self.calculate_support_with_min(candidate, min_support_count)
            .map(|count| (candidate.clone(), count))
    })
    .collect();
```

### 5. **Parallel Rule Generation** (Multi-core Utilization)
- **Before**: Sequential loop generating rules for each itemset
- **After**: Parallel iteration using Rayon's `par_iter()` and `flat_map()`
- **Impact**: Rule generation now utilizes multiple cores
- **Location**: Lines 431-476

### 6. **Chunked Parallel Iteration** (v2 - Load Balancing)
- **Added**: Adaptive chunk sizing for parallel operations
- **Formula**: `chunk_size = (items / (num_threads * multiplier)).max(1)`
- **Impact**: Better load balancing, reduced overhead, improved cache locality
- **Location**: Support calculation and rule generation loops
- **Why**: Small chunks cause thread scheduling overhead; large chunks cause load imbalance

### 7. **Thread Pool Configuration** (v2 - Core Utilization)
- **Added**: Explicit Rayon thread pool initialization in `main.rs`
- **Configuration**: Uses `num_cpus::get()` to detect all logical cores
- **Impact**: Ensures all CPU cores are utilized, not just physical cores
- **Location**: `src/main.rs` line 7-10

### 8. **Flattened Parallelism** (v2 - Reduced Contention)
- **Before**: Nested `par_iter()` causing thread pool contention
- **After**: Flatten data structures before parallel processing
- **Impact**: Better thread utilization, less contention
- **Location**: Rule generation - flatten itemsets before parallel processing

### 9. **Parallel Prune Step** (v2 - Additional Parallelization)
- **Added**: Parallel filtering for large candidate sets
- **Threshold**: Sequential for <100 items, parallel for larger sets
- **Impact**: Reduces another sequential bottleneck
- **Location**: `prune_step()` function

### 10. **Inline Hints for Hot Functions** (v2 - Compiler Optimization)
- **Added**: `#[inline]` attribute to frequently called functions
- **Functions**: `calculate_support()`, `calculate_support_with_min()`
- **Impact**: Helps compiler optimize critical path, reduces function call overhead

## Expected Performance Improvements

### Version 1 Results
1. ✅ **CPU Utilization**: Improved from 1 core to ~50% of cores
2. ✅ **Algorithmic Efficiency**: Sorting bitmaps by size reduces total intersection operations
3. ✅ **Early Termination**: Avoids wasted computation on itemsets that won't meet thresholds

### Version 2 Results (Current)
1. 🎯 **CPU Utilization**: Should now use ALL available CPU cores at high utilization
2. 🎯 **Load Balancing**: Adaptive chunking ensures even distribution of work across threads
3. 🎯 **Reduced Contention**: Flattened parallelism eliminates thread pool conflicts
4. 🎯 **Better Scaling**: Performance should scale nearly linearly with core count

## Benchmarking Recommendations

To measure the impact of these optimizations:

1. **Before/After Comparison**: Run the same dataset with a commit before and after these changes
2. **CPU Utilization**: Monitor with `htop` or Task Manager - should see multiple cores at high usage
3. **Time Metrics**: Add timing code around:
   - Support calculation loop (line 233)
   - Rule generation (line 174)
   - Overall mining time

## Dependencies Added

- **rayon = "1.10"**: Data parallelism library
- **num_cpus = "1.16"**: CPU core detection

## Example Timing Code

```rust
use std::time::Instant;

let start = Instant::now();
// ... code to time ...
let duration = start.elapsed();
println!("Operation took: {:?}", duration);
```

## Troubleshooting "Half Cores Running"

If you still see only half cores utilized:

1. **Check Hyper-Threading**: Your CPU might have 8 logical cores but only 4 physical cores
   - Windows: Task Manager → Performance → CPU → Check "Logical processors"
   - Linux: `lscpu | grep "^CPU(s):"` and `lscpu | grep "Core(s) per socket"`

2. **Memory Bandwidth Bottleneck**: If operations are memory-bound, not CPU-bound
   - Monitor memory bandwidth usage
   - Consider if the dataset fits in cache

3. **OS Thread Scheduling**: Ensure your OS isn't limiting threads
   - Windows: Check power plan (should be "High Performance" for benchmarks)
   - Linux: Check CPU governor: `cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor`

4. **Verify Thread Pool**: Check the output shows correct thread count
   ```
   Using N threads for parallel processing
   ```

5. **Build in Release Mode**: Always benchmark with `--release`
   ```bash
   cargo run --release
   cargo run --example benchmark --release
   ```

## Potential Further Optimizations

If performance is still not satisfactory:

1. **Support Caching**: Cache frequently calculated support values (trade-off: memory vs speed)
2. **Bitmap Sampling**: For very large bitmaps, sample to estimate support before full calculation
3. **Incremental Intersection**: Use RoaringBitmap's specialized intersection methods
4. **SIMD Operations**: Explore if RoaringBitmap can leverage SIMD for faster operations

## Testing

The existing unit tests in `src/apriori.rs` should all pass:
- `test_join_step`
- `test_prune_step`
- `test_calculate_support`
- `test_rule_confidence_calculation`
- `test_rule_edge_cases`

Run with: `cargo test`

