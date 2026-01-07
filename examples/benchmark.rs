// Simple benchmark to measure performance improvements
// Run with: cargo run --example benchmark --release

use hqapriori::apriori::{Apriori, LevelConfig};
use std::collections::HashMap;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Apriori Performance Benchmark ===\n");
    
    // Detect CPU info
    let num_cpus = num_cpus::get();
    let num_physical = num_cpus::get_physical();
    println!("CPU Info:");
    println!("  Logical cores: {}", num_cpus);
    println!("  Physical cores: {}", num_physical);
    if num_cpus > num_physical {
        println!("  Hyper-Threading: Enabled ({} threads per core)", num_cpus / num_physical);
    }
    println!();
    
    // Configuration
    let mut configs = HashMap::new();
    configs.insert(
        -1,
        LevelConfig {
            support: 0.001,
            confidence: 0.2,
        },
    );
    
    println!("Loading repository...");
    let load_start = Instant::now();
    
    let mut apriori = Apriori::new(
        "datasets/instacart_priorFIM.txt",
        "datasets/instacart_taxonomy.txt",
        configs,
    )?;
    
    let load_duration = load_start.elapsed();
    println!("Repository loaded in: {:.2?}", load_duration);
    println!("  Transactions: {}", apriori.repository.transaction_count);
    println!();
    
    println!("Starting mining with parallelization...");
    println!("💡 Monitor CPU usage to verify all cores are utilized:");
    println!("   - Windows: Task Manager → Performance → CPU (should see all logical processors active)");
    println!("   - Linux: htop (press F2 → Display options → enable detailed CPU view)");
    println!("   - Expect: ALL {} cores at 80-100% utilization\n", num_cpus);
    let mine_start = Instant::now();
    
    let rules_by_level = apriori.mine();
    
    let mine_duration = mine_start.elapsed();
    
    println!("\n=== Benchmark Results ===");
    println!("Total mining time: {:.2?}", mine_duration);
    println!("Loading time: {:.2?}", load_duration);
    println!("Total time: {:.2?}", load_duration + mine_duration);
    println!();
    
    // Results summary
    let mut levels: Vec<usize> = rules_by_level.keys().cloned().collect();
    levels.sort();
    
    let total_rules: usize = rules_by_level.values().map(|r| r.len()).sum();
    println!("Total rules found: {}", total_rules);
    println!("Levels with rules: {:?}", levels);
    
    for level in &levels {
        if let Some(rules) = rules_by_level.get(level) {
            println!("  Level {}: {} rules", level, rules.len());
        }
    }
    
    println!("\n=== Performance Metrics ===");
    if total_rules > 0 {
        let rules_per_sec = total_rules as f64 / mine_duration.as_secs_f64();
        println!("Rules per second: {:.0}", rules_per_sec);
    }
    if !levels.is_empty() {
        println!("Average time per level: {:.2?}", mine_duration / levels.len() as u32);
    }
    
    println!("\n=== Optimization Status ===");
    println!("✓ Bitmap sorting by size (smallest first)");
    println!("✓ Early exit on empty intersections");
    println!("✓ Parallel support calculation (multi-core)");
    println!("✓ Parallel rule generation (multi-core)");
    println!("✓ Optimized support calculation with min threshold");
    
    Ok(())
}

