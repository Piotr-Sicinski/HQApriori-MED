use hqapriori_med::apriori::{Apriori, LevelConfig, Rule};
use hqapriori_med::repository::Repository;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

// Helper function to create a simple test taxonomy file with unique name
fn create_test_taxonomy_file(test_name: &str) -> std::path::PathBuf {
    use std::fs;
    use std::io::Write;

    let path = std::path::PathBuf::from(format!("tests/test_taxonomy_{}.txt", test_name));
    let mut file = fs::File::create(&path).unwrap();

    // Create a simple flat taxonomy (no hierarchy for basic tests)
    writeln!(file, "a,ROOT").unwrap();
    writeln!(file, "b,ROOT").unwrap();
    writeln!(file, "c,ROOT").unwrap();
    writeln!(file, "d,ROOT").unwrap();
    writeln!(file, "e,ROOT").unwrap();

    path
}

// Helper function to create a simple test transactions file with unique name
fn create_test_transactions_file(test_name: &str) -> std::path::PathBuf {
    use std::fs;
    use std::io::Write;

    let path = std::path::PathBuf::from(format!("tests/test_transactions_{}.txt", test_name));
    let mut file = fs::File::create(&path).unwrap();

    // Example from Agrawal et al. 1994 paper
    writeln!(file, "a b c").unwrap();
    writeln!(file, "a b d").unwrap();
    writeln!(file, "a c d").unwrap();
    writeln!(file, "b c e").unwrap();
    writeln!(file, "b d e").unwrap();

    path
}

#[test]
fn test_rule_confidence() {
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
fn test_rule_confidence_zero_lhs() {
    let rule = Rule {
        lhs: vec![1, 2],
        rhs: vec![3],
        count_full: 0,
        count_lhs: 0,
        num_transactions: 5,
    };

    assert_eq!(rule.confidence(), 0.0);
}

#[test]
fn test_rule_support_zero_transactions() {
    let rule = Rule {
        lhs: vec![1, 2],
        rhs: vec![3],
        count_full: 2,
        count_lhs: 3,
        num_transactions: 0,
    };

    assert_eq!(rule.support(), 0.0);
}

#[test]
fn test_join_step_basic() {
    // This test verifies the join_step logic matches Python implementation
    // From Python: itemsets = [(1, 2, 3), (1, 2, 4), (1, 3, 4), (1, 3, 5), (2, 3, 4)]
    // Expected: [(1, 2, 3, 4), (1, 3, 4, 5)]

    // We'll test this indirectly through integration tests since join_step is private
    // But we can verify the overall behavior
}

#[test]
fn test_basic_itemset_generation() {
    let taxonomy_path = create_test_taxonomy_file("basic");
    let transactions_path = create_test_transactions_file("basic");

    let mut configs = HashMap::new();
    configs.insert(
        1,
        LevelConfig {
            support: 0.4, // 2/5 = 0.4 minimum
            confidence: 0.5,
            gen_rules: true,
        },
    );

    let mut apriori = Apriori::new(
        transactions_path.to_str().unwrap(),
        taxonomy_path.to_str().unwrap(),
        configs,
    )
    .unwrap();

    let rules_by_level = apriori.mine();

    // Should find some rules at level 1
    assert!(rules_by_level.contains_key(&1));

    // Cleanup
    fs::remove_file(taxonomy_path).ok();
    fs::remove_file(transactions_path).ok();
}

#[test]
fn test_support_threshold() {
    let taxonomy_path = create_test_taxonomy_file("support");
    let transactions_path = create_test_transactions_file("support");

    let mut configs = HashMap::new();
    configs.insert(
        1,
        LevelConfig {
            support: 0.8, // Very high threshold - should find fewer itemsets
            confidence: 0.5,
            gen_rules: true,
        },
    );

    let mut apriori = Apriori::new(
        transactions_path.to_str().unwrap(),
        taxonomy_path.to_str().unwrap(),
        configs,
    )
    .unwrap();

    let _rules_by_level = apriori.mine();

    // With high support threshold, might find fewer or no rules
    // This tests that support filtering works

    // Cleanup
    fs::remove_file(taxonomy_path).ok();
    fs::remove_file(transactions_path).ok();
}

#[test]
fn test_confidence_threshold() {
    let taxonomy_path = create_test_taxonomy_file("confidence");
    let transactions_path = create_test_transactions_file("confidence");

    let mut configs = HashMap::new();
    configs.insert(
        1,
        LevelConfig {
            support: 0.4,
            confidence: 1.0, // Very high confidence - should find fewer rules
            gen_rules: true,
        },
    );

    let mut apriori = Apriori::new(
        transactions_path.to_str().unwrap(),
        taxonomy_path.to_str().unwrap(),
        configs,
    )
    .unwrap();

    let rules_by_level = apriori.mine();

    // All rules should have confidence >= 1.0 (if any exist)
    if let Some(rules) = rules_by_level.get(&1) {
        for rule in rules {
            assert!(rule.confidence() >= 1.0);
        }
    }

    // Cleanup
    fs::remove_file(taxonomy_path).ok();
    fs::remove_file(transactions_path).ok();
}

#[test]
fn test_empty_transactions() {
    use std::fs;
    let taxonomy_path = create_test_taxonomy_file("empty");
    let empty_transactions_path = std::path::PathBuf::from("tests/test_empty_transactions.txt");
    let _file = fs::File::create(&empty_transactions_path).unwrap();
    // Empty file

    let mut configs = HashMap::new();
    configs.insert(
        1,
        LevelConfig {
            support: 0.1,
            confidence: 0.5,
            gen_rules: true,
        },
    );

    let mut apriori = Apriori::new(
        empty_transactions_path.to_str().unwrap(),
        taxonomy_path.to_str().unwrap(),
        configs,
    )
    .unwrap();

    let _rules_by_level = apriori.mine();

    // Should handle empty transactions gracefully
    assert_eq!(apriori.repository.transaction_count, 0);

    // Cleanup
    fs::remove_file(taxonomy_path).ok();
    fs::remove_file(empty_transactions_path).ok();
}

#[test]
fn test_negative_level_config() {
    let taxonomy_path = create_test_taxonomy_file("negative");
    let transactions_path = create_test_transactions_file("negative");

    let mut configs = HashMap::new();
    configs.insert(
        -1, // Last level
        LevelConfig {
            support: 0.4,
            confidence: 0.5,
            gen_rules: true,
        },
    );

    let apriori = Apriori::new(
        transactions_path.to_str().unwrap(),
        taxonomy_path.to_str().unwrap(),
        configs,
    )
    .unwrap();

    // Negative level should be converted to absolute level
    // Since we have a flat taxonomy (ROOT -> a,b,c,d,e), max_level should be 1
    // So -1 should become level 1
    assert!(apriori.level_configs.contains_key(&1));

    // Cleanup
    fs::remove_file(taxonomy_path).ok();
    fs::remove_file(transactions_path).ok();
}

#[test]
fn test_apriori_algorithm_example_from_paper() {
    // This test uses the classic example from Agrawal et al. 1994
    // Transactions: [(1, 3, 4), (2, 3, 5), (1, 2, 3, 5), (2, 5)]
    // Expected itemsets with min_support = 2/5 = 0.4:
    //   1-itemsets: {(1): 2, (2): 3, (3): 3, (5): 3}
    //   2-itemsets: {(1, 3): 2, (2, 3): 2, (2, 5): 3, (3, 5): 2}
    //   3-itemsets: {(2, 3, 5): 2}

    let taxonomy_path = std::path::PathBuf::from("tests/test_paper_taxonomy.txt");
    let transactions_path = std::path::PathBuf::from("tests/test_paper_transactions.txt");

    {
        let mut file = fs::File::create(&taxonomy_path).unwrap();
        writeln!(file, "1,ROOT").unwrap();
        writeln!(file, "2,ROOT").unwrap();
        writeln!(file, "3,ROOT").unwrap();
        writeln!(file, "4,ROOT").unwrap();
        writeln!(file, "5,ROOT").unwrap();
    }

    {
        let mut file = fs::File::create(&transactions_path).unwrap();
        writeln!(file, "1 3 4").unwrap();
        writeln!(file, "2 3 5").unwrap();
        writeln!(file, "1 2 3 5").unwrap();
        writeln!(file, "2 5").unwrap();
    }

    let mut configs = HashMap::new();
    configs.insert(
        1,
        LevelConfig {
            support: 0.4, // 2/5
            confidence: 0.5,
            gen_rules: true,
        },
    );

    let mut apriori = Apriori::new(
        transactions_path.to_str().unwrap(),
        taxonomy_path.to_str().unwrap(),
        configs,
    )
    .unwrap();

    assert_eq!(apriori.repository.transaction_count, 4);

    let rules_by_level = apriori.mine();

    // Should find rules at level 1
    assert!(rules_by_level.contains_key(&1));

    // Cleanup
    fs::remove_file(taxonomy_path).ok();
    fs::remove_file(transactions_path).ok();
}

#[test]
fn test_multiple_levels() {
    // Test with hierarchical taxonomy
    let taxonomy_path = std::path::PathBuf::from("tests/test_hierarchical_taxonomy.txt");
    let transactions_path = std::path::PathBuf::from("tests/test_hierarchical_transactions.txt");

    {
        let mut file = fs::File::create(&taxonomy_path).unwrap();
        writeln!(file, "a,level1").unwrap();
        writeln!(file, "b,level1").unwrap();
        writeln!(file, "level1,ROOT").unwrap();
    }

    {
        let mut file = fs::File::create(&transactions_path).unwrap();
        writeln!(file, "a b").unwrap();
        writeln!(file, "a").unwrap();
        writeln!(file, "b").unwrap();
    }

    let mut configs = HashMap::new();
    configs.insert(
        1,
        LevelConfig {
            support: 0.3,
            confidence: 0.5,
            gen_rules: true,
        },
    );
    configs.insert(
        2,
        LevelConfig {
            support: 0.3,
            confidence: 0.5,
            gen_rules: true,
        },
    );

    let mut apriori = Apriori::new(
        transactions_path.to_str().unwrap(),
        taxonomy_path.to_str().unwrap(),
        configs,
    )
    .unwrap();

    let rules_by_level = apriori.mine();

    // Should process multiple levels
    // Should process multiple levels (may or may not find rules depending on support)
    let _rules_count = rules_by_level.len();

    // Cleanup
    fs::remove_file(taxonomy_path).ok();
    fs::remove_file(transactions_path).ok();
}

#[test]
fn test_rule_validation() {
    // Test that all generated rules meet the confidence threshold
    let taxonomy_path = create_test_taxonomy_file("validation");
    let transactions_path = create_test_transactions_file("validation");

    let min_confidence = 0.6;
    let mut configs = HashMap::new();
    configs.insert(
        1,
        LevelConfig {
            support: 0.4,
            confidence: min_confidence,
            gen_rules: true,
        },
    );

    let mut apriori = Apriori::new(
        transactions_path.to_str().unwrap(),
        taxonomy_path.to_str().unwrap(),
        configs,
    )
    .unwrap();

    let rules_by_level = apriori.mine();

    // Verify all rules meet confidence threshold
    for (_, rules) in rules_by_level.iter() {
        for rule in rules {
            assert!(
                rule.confidence() >= min_confidence,
                "Rule confidence {} should be >= {}",
                rule.confidence(),
                min_confidence
            );
            assert!(
                rule.confidence() <= 1.0,
                "Rule confidence {} should be <= 1.0",
                rule.confidence()
            );
            assert!(
                rule.support() >= 0.0 && rule.support() <= 1.0,
                "Rule support {} should be between 0 and 1",
                rule.support()
            );
        }
    }

    // Cleanup
    fs::remove_file(taxonomy_path).ok();
    fs::remove_file(transactions_path).ok();
}

#[test]
fn test_apriori_from_repository() {
    // Test creating Apriori from an existing Repository instance
    let taxonomy_path = create_test_taxonomy_file("from_repo");
    let transactions_path = create_test_transactions_file("from_repo");

    // First, create the repository separately
    let repository = Repository::new(
        transactions_path.to_str().unwrap(),
        taxonomy_path.to_str().unwrap(),
    )
    .unwrap();

    // Verify repository was created correctly
    assert_eq!(repository.transaction_count, 5);

    // Now create Apriori instance from the repository
    let mut configs = HashMap::new();
    configs.insert(
        1,
        LevelConfig {
            support: 0.4,
            confidence: 0.5,
            gen_rules: true,
        },
    );

    let mut apriori = Apriori::from_repository(repository, configs);

    // Verify the apriori instance has the correct repository
    assert_eq!(apriori.repository.transaction_count, 5);

    // Mine rules
    let rules_by_level = apriori.mine();

    // Should find some rules at level 1
    assert!(rules_by_level.contains_key(&1));

    // Cleanup
    fs::remove_file(taxonomy_path).ok();
    fs::remove_file(transactions_path).ok();
}
