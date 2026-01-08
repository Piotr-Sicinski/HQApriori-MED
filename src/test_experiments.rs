use hqapriori_med::apriori::{Apriori, LevelConfig};
use std::collections::HashMap;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running quick test of experiments...\n");

    // Test with tiny dataset
    let dataset_path = "datasets/instacart_trainFIM-tiny.txt";
    let taxonomy_path = "datasets/instacart_taxonomy.txt";

    // Test 1: Basic mining
    println!("Test 1: Basic multi-level mining");
    let start = Instant::now();

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
            support: 0.005,
            confidence: 0.2,
            gen_rules: true,
        },
    );

    let mut apriori = Apriori::new(dataset_path, taxonomy_path, configs)?;
    let rules = apriori.mine();

    let time = start.elapsed().as_millis();
    let num_rules: usize = rules.values().map(|v| v.len()).sum();

    println!("  Time: {}ms", time);
    println!("  Rules found: {}", num_rules);
    println!("  Rules by level:");
    for (level, rules_vec) in rules.iter() {
        println!("    Level {}: {} rules", level, rules_vec.len());
    }

    // Test 2: Single level mining
    println!("\nTest 2: Single level mining");
    let start = Instant::now();

    let mut configs2 = HashMap::new();
    configs2.insert(
        1,
        LevelConfig {
            support: 0.5,
            confidence: 0.2,
            gen_rules: true,
        },
    );

    let mut apriori2 = Apriori::new(dataset_path, taxonomy_path, configs2)?;
    let rules2 = apriori2.mine();

    let time2 = start.elapsed().as_millis();
    let num_rules2: usize = rules2.values().map(|v| v.len()).sum();

    println!("  Time: {}ms", time2);
    println!("  Rules found: {}", num_rules2);

    println!("\nAll tests passed! Experiments infrastructure is working.");

    Ok(())
}
