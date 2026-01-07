# Quick Start - Optimized Multi-Core Apriori

## TL;DR - Run Optimized Version

```bash
# Build and run with full optimizations
cargo run --release

# Or run the benchmark
cargo run --example benchmark --release
```

**Important**: Always use `--release` mode for real performance!

## What to Expect

### Before Optimizations
- 1 CPU core at 100%
- Other cores idle or low usage
- Slow mining on large datasets

### After v2 Optimizations
- **ALL** CPU cores at 80-100% during mining
- 4-8x speedup (depending on core count)
- Efficient parallel processing

## Verify It's Working

Open Task Manager (Windows) or htop (Linux) **BEFORE** running:

```bash
# Then run in another terminal:
cargo run --release
```

**Look for**: All CPU cores showing high activity (80-100%), not just one or half.

## Key Files Modified

| File | What Changed |
|------|--------------|
| `src/apriori.rs` | Core optimization - parallel support calculation, chunking, inlining |
| `src/main.rs` | Thread pool configuration to use all cores |
| `Cargo.toml` | Added `rayon` and `num_cpus` dependencies |
| `examples/benchmark.rs` | Performance measurement tool |

## Optimization Features

✅ **Parallel support calculation** - Main bottleneck now multi-threaded
✅ **Parallel rule generation** - Rules generated across all cores
✅ **Parallel pruning** - Large candidate sets pruned in parallel
✅ **Adaptive chunking** - Work distributed evenly across threads
✅ **Bitmap sorting** - Smaller bitmaps first for faster intersection
✅ **Early exit** - Stops calculation when below threshold
✅ **Thread pool tuning** - All logical cores utilized

## Troubleshooting

### Only Half Cores Running?

**Likely cause**: Your CPU has hyper-threading. 8 "logical" cores = 4 "physical" cores.

- With memory-bound operations, only physical cores may show 100%
- This is actually normal and optimal!
- Check: `examples/benchmark.rs` will show physical vs logical core count

### Still Slow?

1. **Are you using --release?**
   ```bash
   cargo run --release  # ← Important!
   ```

2. **Is your dataset very small?**
   - Small datasets don't have enough work to saturate all cores
   - Try with larger datasets or lower min_support

3. **Check if memory-bound**
   - Open Task Manager → Performance → Memory
   - If memory bandwidth maxed out, you're memory-bound not CPU-bound

### No Improvement?

The program might be spending time in sequential sections:
- File I/O (loading data)
- Tree traversal (single-threaded by design)
- Very small candidate sets (not worth parallelizing)

## Performance Comparison

Run the benchmark to see actual numbers:

```bash
cargo run --example benchmark --release
```

Expected output:
```
=== Apriori Performance Benchmark ===

CPU Info:
  Logical cores: 8
  Physical cores: 4
  Hyper-Threading: Enabled (2 threads per core)

Using 8 threads for parallel processing

Repository loaded in: 2.34s
  Transactions: 1000000

Starting mining with parallelization...
💡 Monitor CPU usage to verify all cores are utilized
...
Total mining time: 45.67s
Rules per second: 1234
```

## Documentation

- `OPTIMIZATIONS.md` - Technical details of all optimizations
- `PERFORMANCE_TUNING_v2.md` - Deep dive into v2 improvements
- `README.md` - General project information

## Questions?

Common issues:

**Q: Why do I see 8 cores but only 4 at 100%?**
A: You have 4 physical cores with hyper-threading. This is normal for memory-intensive operations.

**Q: Does it work on Mac/Linux?**
A: Yes! The optimizations are cross-platform.

**Q: Can I tune the parallelization?**
A: Yes, see the chunk size multiplier in `src/apriori.rs` line ~239 and ~486.

**Q: What if I have 32+ cores?**
A: The code scales! It will use all available cores automatically.

