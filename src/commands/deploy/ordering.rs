use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{Result, bail};

/// Static deployment order by item type.
///
/// Items of type earlier in this list are deployed before items of types later.
/// This ensures dependencies are satisfied (e.g., Lakehouses exist before Notebooks
/// that reference them, Notebooks exist before `DataPipelines` that invoke them).
///
/// Based on fabric-cicd's `SERIAL_ITEM_PUBLISH_ORDER` extended with additional
/// types supported by fabio.
pub const DEPLOY_ORDER: &[&str] = &[
    "VariableLibrary",
    "Warehouse",
    "MirroredDatabase",
    "Lakehouse",
    "SQLDatabase",
    "Environment",
    "UserDataFunction",
    "Eventhouse",
    "KQLDatabase",
    "SparkJobDefinition",
    "Notebook",
    "SemanticModel",
    "Report",
    "CopyJob",
    "KQLQueryset",
    "KQLDashboard",
    "Reflex",
    "Eventstream",
    "Dataflow",
    "DataPipeline",
    "GraphQLApi",
    "ApacheAirflowJob",
    "MountedDataFactory",
    "DataAgent",
    "MLExperiment",
    "MLModel",
    "Ontology",
    "Map",
    "Connection",
];

/// Returns the deploy priority for a given item type.
/// Lower number = deployed earlier. Unknown types get a high number (deployed last).
pub fn deploy_priority(item_type: &str) -> usize {
    DEPLOY_ORDER
        .iter()
        .position(|&t| t.eq_ignore_ascii_case(item_type))
        .unwrap_or(DEPLOY_ORDER.len())
}

/// Reverse deployment order for deletes.
/// Items that depend on others should be deleted first.
pub fn delete_priority(item_type: &str) -> usize {
    let pos = deploy_priority(item_type);
    DEPLOY_ORDER.len().saturating_sub(pos)
}

/// Topological sort for items that reference each other (e.g., sub-pipelines).
///
/// `items` is a list of (name, references) where references are names of other
/// items in the same list that must be deployed first.
///
/// Returns the sorted order, or an error if circular dependencies are detected.
pub fn topological_sort(items: &[(String, Vec<String>)]) -> Result<Vec<String>> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    // Build adjacency list and in-degree count
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    let names: HashSet<&str> = items.iter().map(|(n, _)| n.as_str()).collect();

    for (name, _) in items {
        in_degree.entry(name.as_str()).or_insert(0);
        dependents.entry(name.as_str()).or_default();
    }

    for (name, refs) in items {
        for dep in refs {
            // Only count references to items within our set
            if names.contains(dep.as_str()) {
                *in_degree.entry(name.as_str()).or_insert(0) += 1;
                dependents
                    .entry(dep.as_str())
                    .or_default()
                    .push(name.as_str());
            }
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(name, _)| *name)
        .collect();

    let mut sorted = Vec::with_capacity(items.len());

    while let Some(node) = queue.pop_front() {
        sorted.push(node.to_owned());

        if let Some(deps) = dependents.get(node) {
            for &dep in deps {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }
    }

    if sorted.len() != items.len() {
        let unsorted: Vec<&str> = names
            .iter()
            .filter(|n| !sorted.contains(&(**n).to_owned()))
            .copied()
            .collect();
        bail!(
            "Circular dependency detected among items: {}",
            unsorted.join(", ")
        );
    }

    Ok(sorted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_priority_known_types() {
        assert!(deploy_priority("VariableLibrary") < deploy_priority("Notebook"));
        assert!(deploy_priority("Notebook") < deploy_priority("DataPipeline"));
        assert!(deploy_priority("Lakehouse") < deploy_priority("Notebook"));
        assert!(deploy_priority("SemanticModel") < deploy_priority("Report"));
    }

    #[test]
    fn test_deploy_priority_unknown_type() {
        let unknown = deploy_priority("UnknownType");
        assert_eq!(unknown, DEPLOY_ORDER.len());
    }

    #[test]
    fn test_delete_priority_reverses_order() {
        assert!(delete_priority("DataPipeline") < delete_priority("Notebook"));
        assert!(delete_priority("Notebook") < delete_priority("Lakehouse"));
    }

    #[test]
    fn test_topological_sort_simple() {
        let items = vec![
            ("C".to_owned(), vec!["A".to_owned(), "B".to_owned()]),
            ("A".to_owned(), vec![]),
            ("B".to_owned(), vec!["A".to_owned()]),
        ];
        let sorted = topological_sort(&items).unwrap();
        let pos_a = sorted.iter().position(|n| n == "A").unwrap();
        let pos_b = sorted.iter().position(|n| n == "B").unwrap();
        let pos_c = sorted.iter().position(|n| n == "C").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_topological_sort_circular() {
        let items = vec![
            ("A".to_owned(), vec!["B".to_owned()]),
            ("B".to_owned(), vec!["A".to_owned()]),
        ];
        assert!(topological_sort(&items).is_err());
    }

    #[test]
    fn test_topological_sort_empty() {
        let items: Vec<(String, Vec<String>)> = vec![];
        let sorted = topological_sort(&items).unwrap();
        assert!(sorted.is_empty());
    }

    #[test]
    fn test_topological_sort_external_refs_ignored() {
        // References to items NOT in the set are ignored (not an error)
        let items = vec![
            ("A".to_owned(), vec!["External".to_owned()]),
            ("B".to_owned(), vec!["A".to_owned()]),
        ];
        let sorted = topological_sort(&items).unwrap();
        let pos_a = sorted.iter().position(|n| n == "A").unwrap();
        let pos_b = sorted.iter().position(|n| n == "B").unwrap();
        assert!(pos_a < pos_b);
    }
}
