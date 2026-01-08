pub mod apriori;
pub mod repository;

use apriori::{sort_rules, Apriori, LevelConfig, SortBy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("HQApriori - Multi-level Association Rule Mining\n");

    // Example configuration
    use std::collections::HashMap;
    let mut configs = HashMap::new();
    // configs.insert(
    //     1,
    //     LevelConfig {
    //         support: 0.3,
    //         confidence: 0.6,
    //         enabled: true,
    //     },
    // );
    // configs.insert(
    //     2,
    //     LevelConfig {
    //         support: 0.2,
    //         confidence: 0.5,
    //     },
    // );

    configs.insert(
        3,
        LevelConfig {
            support: 0.005,
            confidence: 0.2,
            gen_rules: true,
        },
    );

    // // Run on the fruithut dataset
    // let mut apriori = Apriori::new(
    //     "datasets/fruithut_original.txt",
    //     "datasets/Fruithut_taxonomy_data.txt",
    //     configs,
    // )?;

    let mut apriori = Apriori::new(
        "datasets/instacart_trainFIM.txt",
        "datasets/instacart_taxonomy.txt",
        configs,
    )?;

    println!("Repository loaded:");
    println!("  Transactions: {}", apriori.repository.transaction_count);
    println!();

    apriori.repository.plot_tree();

    let mut rules_by_level = apriori.mine();

    println!("\n=== Results ===");
    let mut levels: Vec<usize> = rules_by_level.keys().cloned().collect();
    levels.sort();
    for level in levels {
        if let Some(rules) = rules_by_level.get_mut(&level) {
            // Sort rules by confidence first, then support
            sort_rules(rules, SortBy::ConfidenceSupport);

            println!("\nLevel {}: {} rules", level, rules.len());
            for rule in rules.iter().take(10) {
                rule.print(&apriori.repository);
            }
            if rules.len() > 10 {
                println!("  ... and {} more rules", rules.len() - 10);
            }
        }
    }

    Ok(())
}
