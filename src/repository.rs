use indextree::{Arena, NodeId};
use roaring::RoaringBitmap;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Clone)]
pub struct NodeData {
    pub id: u32,
    pub name: String,
    pub bitmap: RoaringBitmap,
}

pub struct Repository {
    pub arena: Arena<NodeData>,
    pub root: NodeId,
    pub node_by_id: HashMap<u32, NodeId>,
    pub transaction_count: u32,
}

impl Repository {
    pub fn new(
        transactions_path: &str,
        taxonomy_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut arena = Arena::new();
        let mut node_by_name = HashMap::new();
        let mut node_by_id = HashMap::new();
        let mut next_id = 0u32;

        // Parse taxonomy file and build tree directly
        let file = File::open(taxonomy_path)?;
        let reader = BufReader::new(file);

        // Helper function to get or create a node
        let get_or_create_node = |name: &String,
                                  arena: &mut Arena<NodeData>,
                                  node_by_name: &mut HashMap<String, NodeId>,
                                  node_by_id: &mut HashMap<u32, NodeId>,
                                  next_id: &mut u32|
         -> NodeId {
            if let Some(&node_id) = node_by_name.get(name) {
                node_id
            } else {
                let node_data = NodeData {
                    id: *next_id,
                    name: name.clone(),
                    bitmap: RoaringBitmap::new(),
                };
                let node_id = arena.new_node(node_data.clone());
                node_by_name.insert(name.clone(), node_id);
                node_by_id.insert(*next_id, node_id);
                *next_id += 1;
                node_id
            }
        };

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 2 {
                panic!("Invalid taxonomy file format");
            }

            let child_name = parts[0].to_string();
            let parent_name = parts[1].to_string();

            // Get or create parent node
            let parent_id = get_or_create_node(
                &parent_name,
                &mut arena,
                &mut node_by_name,
                &mut node_by_id,
                &mut next_id,
            );

            // Get or create child node
            let child_id = get_or_create_node(
                &child_name,
                &mut arena,
                &mut node_by_name,
                &mut node_by_id,
                &mut next_id,
            );

            // Link parent to child
            parent_id.append(child_id, &mut arena);
        }

        // Find roots (nodes with no parent)
        let mut roots = Vec::new();
        for &node_id in node_by_id.values() {
            if arena[node_id].parent().is_none() {
                roots.push(node_id);
            }
        }

        // Create artificial root if needed
        let root = if roots.len() == 1 {
            roots[0]
        } else {
            let root_data = NodeData {
                id: next_id,
                name: "ROOT".to_string(),
                bitmap: RoaringBitmap::new(),
            };
            let root_id = arena.new_node(root_data);
            node_by_id.insert(next_id, root_id);
            for r in roots {
                root_id.append(r, &mut arena);
            }
            // next_id += 1; // No need to increment next_id for the root node anymore
            root_id
        };

        // Load transactions
        let transaction_count =
            Self::load_transactions(&mut arena, &node_by_name, transactions_path)?;

        // Propagate bitmaps from leaves to root
        Self::propagate_bitmaps(&mut arena, root);

        Ok(Repository {
            arena,
            root,
            node_by_id,
            transaction_count,
        })
    }

    fn load_transactions(
        arena: &mut Arena<NodeData>,
        node_by_name: &HashMap<String, NodeId>,
        transactions_path: &str,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let file = File::open(transactions_path)?;
        let reader = BufReader::new(file);
        let mut transaction_id = 0u32;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || line.starts_with('@') {
                continue;
            }

            // Split by ':' to ignore additional data
            let main_part = line.split(':').next().unwrap_or(line);

            // Try different separators
            let items: Vec<&str> = main_part.split_whitespace().collect();

            for item in items {
                // Skip if item is not found in taxonomy
                if let Some(&node_id) = node_by_name.get(item) {
                    arena[node_id].get_mut().bitmap.insert(transaction_id);
                }
            }

            transaction_id += 1;
        }

        Ok(transaction_id)
    }

    fn propagate_bitmaps(arena: &mut Arena<NodeData>, node_id: NodeId) {
        let children: Vec<NodeId> = node_id.children(arena).collect();

        if children.is_empty() {
            // Leaf node, bitmap already set
            return;
        }

        // Recursively propagate for children
        for child_id in &children {
            Self::propagate_bitmaps(arena, *child_id);
        }

        // Union bitmaps of children
        let mut combined = RoaringBitmap::new();
        for child_id in &children {
            combined |= &arena[*child_id].get().bitmap;
        }
        arena[node_id].get_mut().bitmap = combined;
    }

    pub fn get_children(&self, parent_nodes: &HashSet<NodeId>) -> HashSet<NodeId> {
        let mut children = HashSet::new();
        for &parent_id in parent_nodes {
            for child_id in parent_id.children(&self.arena) {
                if !child_id.is_removed(&self.arena) {
                    children.insert(child_id);
                }
            }
        }
        children
    }

    /// Plots the taxonomy tree excluding the last level (leaf nodes)
    pub fn plot_tree(&self) {
        println!("\n=== Taxonomy Tree (excluding leaf level) ===\n");
        Self::plot_tree_recursive(&self.arena, self.root, 0, true, &Vec::new());
        println!();
    }

    fn plot_tree_recursive(
        arena: &Arena<NodeData>,
        node_id: NodeId,
        depth: usize,
        is_last: bool,
        prefix: &[bool], // Track which ancestors were last children
    ) {
        let node = arena[node_id].get();
        let children: Vec<NodeId> = node_id.children(arena).collect();

        // Skip leaf nodes (nodes with no children) - don't print them
        if children.is_empty() {
            return;
        }

        // Print indentation based on prefix
        for &was_last in prefix {
            if was_last {
                print!("    "); // Use spaces for last child columns
            } else {
                print!("│   "); // Use vertical line for non-last child columns
            }
        }

        // Print connector for current node
        if depth > 0 {
            if is_last {
                print!("└── ");
            } else {
                print!("├── ");
            }
        }

        // Count child nodes (non-leaf) and child leaves (leaf)
        let mut child_nodes = 0;
        let mut child_leafs = 0;
        for &child_id in &children {
            let child_children: Vec<NodeId> = child_id.children(arena).collect();
            if child_children.is_empty() {
                child_leafs += 1;
            } else {
                child_nodes += 1;
            }
        }

        // Print node name with counts
        println!(
            "{} (child_nodes: {}, child_leafs: {})",
            node.name, child_nodes, child_leafs
        );

        // Build new prefix for children
        let mut new_prefix = prefix.to_vec();
        new_prefix.push(is_last);

        // Recursively print children (only non-leaf children will be printed)
        let num_children = children.len();
        for (idx, &child_id) in children.iter().enumerate() {
            let is_last_child = idx == num_children - 1;
            Self::plot_tree_recursive(arena, child_id, depth + 1, is_last_child, &new_prefix);
        }
    }
}
