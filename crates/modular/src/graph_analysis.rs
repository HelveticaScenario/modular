//! Graph cycle detection via Tarjan's SCC algorithm.
//!
//! Returns `ProcessingMode` per module ID. Modules in a strongly-connected
//! component with >1 member, or a single node with a self-loop, are assigned
//! `Sample` mode. All others get `Block` mode.

use modular_core::types::{PatchGraph, ProcessingMode};
use std::collections::HashMap;

/// Analyse `graph` and return the processing mode for each module id.
///
/// Modules not present in the graph are not in the returned map.
pub fn classify_modules(graph: &PatchGraph) -> HashMap<String, ProcessingMode> {
    // Build adjacency: consumer_id → [producer_id, ...]
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();

    for state in &graph.modules {
        deps.entry(state.id.clone()).or_default();
        collect_cable_deps(&state.params, &state.id, &mut deps);
    }

    let mut ctx = TarjanCtx::default();
    let nodes: Vec<String> = deps.keys().cloned().collect();
    for node in &nodes {
        if !ctx.index_map.contains_key(node.as_str()) {
            ctx.strongconnect(node, &deps);
        }
    }

    let mut result = HashMap::new();
    for scc in &ctx.sccs {
        let cyclic = scc.len() > 1
            || deps
                .get(&scc[0])
                .map_or(false, |d| d.iter().any(|x| x == &scc[0]));
        let mode = if cyclic {
            ProcessingMode::Sample
        } else {
            ProcessingMode::Block
        };
        for id in scc {
            result.insert(id.clone(), mode);
        }
    }
    result
}

/// Recursively scan a params JSON value and record any `{type:"cable"}` edges.
fn collect_cable_deps(
    value: &serde_json::Value,
    consumer_id: &str,
    deps: &mut HashMap<String, Vec<String>>,
) {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("type").and_then(|v| v.as_str()) == Some("cable") {
                if let Some(producer_id) = map.get("module").and_then(|v| v.as_str()) {
                    deps.entry(consumer_id.to_string())
                        .or_default()
                        .push(producer_id.to_string());
                    deps.entry(producer_id.to_string()).or_default();
                }
            } else {
                for val in map.values() {
                    collect_cable_deps(val, consumer_id, deps);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr {
                collect_cable_deps(val, consumer_id, deps);
            }
        }
        _ => {}
    }
}

#[derive(Default)]
struct TarjanCtx {
    counter: usize,
    stack: Vec<String>,
    on_stack: HashMap<String, bool>,
    index_map: HashMap<String, usize>,
    lowlink: HashMap<String, usize>,
    sccs: Vec<Vec<String>>,
}

impl TarjanCtx {
    fn strongconnect(&mut self, v: &str, deps: &HashMap<String, Vec<String>>) {
        self.index_map.insert(v.to_string(), self.counter);
        self.lowlink.insert(v.to_string(), self.counter);
        self.counter += 1;
        self.stack.push(v.to_string());
        self.on_stack.insert(v.to_string(), true);

        let neighbors = deps.get(v).cloned().unwrap_or_default();
        for w in neighbors {
            if !self.index_map.contains_key(w.as_str()) {
                self.strongconnect(&w, deps);
                let lv = self.lowlink[v];
                let lw = self.lowlink[&w];
                self.lowlink.insert(v.to_string(), lv.min(lw));
            } else if *self.on_stack.get(&w).unwrap_or(&false) {
                let lv = self.lowlink[v];
                let iw = self.index_map[&w];
                self.lowlink.insert(v.to_string(), lv.min(iw));
            }
        }

        if self.lowlink[v] == self.index_map[v] {
            let mut scc = Vec::new();
            loop {
                let w = self.stack.pop().unwrap();
                self.on_stack.insert(w.clone(), false);
                let is_v = w == v;
                scc.push(w);
                if is_v {
                    break;
                }
            }
            self.sccs.push(scc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modular_core::types::{ModuleState, PatchGraph, ProcessingMode};
    use serde_json::json;

    fn make_graph(edges: &[(&str, &str, &str)]) -> PatchGraph {
        // edges: (consumer_id, producer_id, port)
        let mut ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for (c, p, _) in edges {
            ids.insert(c);
            ids.insert(p);
        }
        let mut modules = Vec::new();
        for id in &ids {
            let params = if let Some(edge) = edges.iter().find(|(c, _, _)| c == id) {
                json!({ "input": { "type": "cable", "module": edge.1, "port": edge.2, "channel": 0 } })
            } else {
                json!({})
            };
            modules.push(ModuleState {
                id: id.to_string(),
                module_type: "test".to_string(),
                id_is_explicit: None,
                params,
            });
        }
        PatchGraph {
            modules,
            module_id_remaps: None,
            scopes: vec![],
        }
    }

    #[test]
    fn no_cycle_is_block_mode() {
        // A -> B -> C (A produces, C consumes)
        let graph = make_graph(&[("B", "A", "out"), ("C", "B", "out")]);
        let modes = classify_modules(&graph);
        assert_eq!(modes["A"], ProcessingMode::Block);
        assert_eq!(modes["B"], ProcessingMode::Block);
        assert_eq!(modes["C"], ProcessingMode::Block);
    }

    #[test]
    fn two_node_cycle_is_sample_mode() {
        // A <-> B (A reads B and B reads A)
        let graph = make_graph(&[("A", "B", "out"), ("B", "A", "out")]);
        let modes = classify_modules(&graph);
        assert_eq!(modes["A"], ProcessingMode::Sample);
        assert_eq!(modes["B"], ProcessingMode::Sample);
    }

    #[test]
    fn self_loop_is_sample_mode() {
        let graph = make_graph(&[("A", "A", "out")]);
        let modes = classify_modules(&graph);
        assert_eq!(modes["A"], ProcessingMode::Sample);
    }

    #[test]
    fn cycle_plus_independent_node() {
        // A <-> B, C is independent
        let mut graph = make_graph(&[("A", "B", "out"), ("B", "A", "out")]);
        graph.modules.push(ModuleState {
            id: "C".to_string(),
            module_type: "test".to_string(),
            id_is_explicit: None,
            params: json!({}),
        });
        let modes = classify_modules(&graph);
        assert_eq!(modes["A"], ProcessingMode::Sample);
        assert_eq!(modes["B"], ProcessingMode::Sample);
        assert_eq!(modes["C"], ProcessingMode::Block);
    }
}
