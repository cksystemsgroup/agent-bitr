//! BTOR2 parser
//!
//! Parses the BTOR2 format (https://github.com/Boolector/btor2tools)
//! into BVCs and BVDDs for the BITR solver.
//!
//! BTOR2 format:
//! - Lines: <nid> <op> <sort> <args...> [<symbol>]
//! - Sorts: `sort bitvec <width>`, `sort array <index-width> <element-width>`
//! - Special nodes: input, state, init, next, bad, constraint, output

use bvdd::types::BvWidth;

/// Parsed BTOR2 sort
#[derive(Debug, Clone)]
pub enum Btor2Sort {
    Bitvec(BvWidth),
    Array { index_width: BvWidth, element_width: BvWidth },
}

/// Parsed BTOR2 node
#[derive(Debug, Clone)]
pub struct Btor2Node {
    pub nid: u32,
    pub op: String,
    pub sort_id: u32,
    pub args: Vec<i64>, // signed: negative = negated
    pub symbol: Option<String>,
}

/// Parsed BTOR2 model
#[derive(Debug)]
pub struct Btor2Model {
    pub sorts: Vec<(u32, Btor2Sort)>,
    pub nodes: Vec<Btor2Node>,
    pub bad_properties: Vec<u32>, // nids of bad nodes
}

/// Parse a BTOR2 file
pub fn parse_btor2(input: &str) -> Result<Btor2Model, String> {
    let mut sorts = Vec::new();
    let mut nodes = Vec::new();
    let mut bad_properties = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let nid: u32 = parts[0].parse().map_err(|e| format!("bad nid: {}", e))?;

        match parts[1] {
            "sort" => {
                if parts.len() >= 4 && parts[2] == "bitvec" {
                    let width: u16 = parts[3].parse().map_err(|e| format!("bad width: {}", e))?;
                    sorts.push((nid, Btor2Sort::Bitvec(width)));
                } else if parts.len() >= 5 && parts[2] == "array" {
                    let iw: u16 = parts[3].parse().map_err(|e| format!("bad index width: {}", e))?;
                    let ew: u16 = parts[4].parse().map_err(|e| format!("bad element width: {}", e))?;
                    sorts.push((nid, Btor2Sort::Array { index_width: iw, element_width: ew }));
                }
            }
            // Binary constant: value is a binary string
            "const" => {
                let sort_id: u32 = parts[2].parse().unwrap_or(0);
                let bin_str = if parts.len() > 3 { parts[3] } else { "0" };
                let val = u64::from_str_radix(bin_str, 2).unwrap_or(0) as i64;
                nodes.push(Btor2Node {
                    nid, op: "constd".to_string(), sort_id,
                    args: vec![val],
                    symbol: parts.get(4).map(|s| s.to_string()),
                });
            }
            // Hex constant: value is a hex string
            "consth" => {
                let sort_id: u32 = parts[2].parse().unwrap_or(0);
                let hex_str = if parts.len() > 3 { parts[3] } else { "0" };
                let val = u64::from_str_radix(hex_str, 16).unwrap_or(0) as i64;
                nodes.push(Btor2Node {
                    nid, op: "constd".to_string(), sort_id,
                    args: vec![val],
                    symbol: parts.get(4).map(|s| s.to_string()),
                });
            }
            // Decimal constant: value may be large unsigned or negative
            "constd" => {
                let sort_id: u32 = parts[2].parse().unwrap_or(0);
                if parts.len() > 3 {
                    let val_str = parts[3];
                    // Handle negative values and large unsigned values
                    let val = if val_str.starts_with('-') {
                        val_str.parse::<i64>().unwrap_or(0)
                    } else {
                        val_str.parse::<i64>()
                            .unwrap_or_else(|_| val_str.parse::<u64>().unwrap_or(0) as i64)
                    };
                    nodes.push(Btor2Node {
                        nid, op: "constd".to_string(), sort_id,
                        args: vec![val],
                        symbol: parts.get(4).map(|s| s.to_string()),
                    });
                } else {
                    nodes.push(Btor2Node {
                        nid, op: "constd".to_string(), sort_id,
                        args: vec![0],
                        symbol: None,
                    });
                }
            }
            "bad" => {
                let arg: u32 = parts[2].parse().map_err(|e| format!("bad arg: {}", e))?;
                bad_properties.push(arg);
                nodes.push(Btor2Node {
                    nid,
                    op: "bad".to_string(),
                    sort_id: 0,
                    args: vec![arg as i64],
                    symbol: parts.get(3).map(|s| s.to_string()),
                });
            }
            "constraint" => {
                // constraint <arg> — same format as bad (no sort)
                let arg: i64 = parts[2].parse().unwrap_or(0);
                nodes.push(Btor2Node {
                    nid,
                    op: "constraint".to_string(),
                    sort_id: 0,
                    args: vec![arg],
                    symbol: parts.get(3).map(|s| s.to_string()),
                });
            }
            "output" => {
                // output <arg> — same format
                let arg: i64 = if parts.len() > 2 { parts[2].parse().unwrap_or(0) } else { 0 };
                nodes.push(Btor2Node {
                    nid,
                    op: "output".to_string(),
                    sort_id: 0,
                    args: vec![arg],
                    symbol: parts.get(3).map(|s| s.to_string()),
                });
            }
            op => {
                let sort_id: u32 = parts[2].parse().unwrap_or(0);
                let mut args = Vec::new();
                for p in &parts[3..] {
                    if let Ok(v) = p.parse::<i64>() {
                        args.push(v);
                    } else {
                        // It's a symbol, stop parsing args
                        break;
                    }
                }
                // Check for symbol (last non-numeric part)
                let symbol = parts.last().and_then(|s| {
                    if s.parse::<i64>().is_err() && *s != op {
                        Some(s.to_string())
                    } else {
                        None
                    }
                });
                nodes.push(Btor2Node { nid, op: op.to_string(), sort_id, args, symbol });
            }
        }
    }

    Ok(Btor2Model { sorts, nodes, bad_properties })
}

/// Cone-of-Influence reduction: remove nodes/states not reachable from
/// bad properties or constraints. This can dramatically reduce state space
/// for models where only a small fraction of states affect the property.
pub fn cone_of_influence(model: &mut Btor2Model) -> usize {
    use std::collections::{HashMap, HashSet};

    let original_nodes = model.nodes.len();

    // Build nid → node index lookup
    let nid_to_idx: HashMap<u32, usize> = model.nodes.iter()
        .enumerate()
        .map(|(i, n)| (n.nid, i))
        .collect();

    // Collect all init/next relationships: state_nid → (init_nid, next_nid)
    let mut state_next: HashMap<u32, u32> = HashMap::new();
    let mut state_init: HashMap<u32, u32> = HashMap::new();
    for node in &model.nodes {
        if node.op == "next" && node.args.len() >= 2 {
            let state_nid = node.args[0].unsigned_abs() as u32;
            let next_nid = node.args[1].unsigned_abs() as u32;
            state_next.insert(state_nid, next_nid);
        }
        if node.op == "init" && node.args.len() >= 2 {
            let state_nid = node.args[0].unsigned_abs() as u32;
            let init_nid = node.args[1].unsigned_abs() as u32;
            state_init.insert(state_nid, init_nid);
        }
    }

    // BFS/DFS backward from bad properties and constraints
    let mut in_cone: HashSet<u32> = HashSet::new();
    let mut worklist: Vec<u32> = Vec::new();

    // Seed: bad property arguments
    for &bad_nid in &model.bad_properties {
        worklist.push(bad_nid);
    }

    // Seed: constraint arguments
    for node in &model.nodes {
        if node.op == "constraint" {
            for &arg in &node.args {
                worklist.push(arg.unsigned_abs() as u32);
            }
        }
    }

    // Walk backward through dependencies
    while let Some(nid) = worklist.pop() {
        if !in_cone.insert(nid) {
            continue; // Already visited
        }

        if let Some(&idx) = nid_to_idx.get(&nid) {
            let node = &model.nodes[idx];

            // For most ops, all args are dependencies
            if node.op != "constd" {
                for &arg in &node.args {
                    let arg_nid = arg.unsigned_abs() as u32;
                    worklist.push(arg_nid);
                }
            }

            // If this is a state in the cone, also pull in its next-state and init
            if node.op == "state" {
                if let Some(&next_nid) = state_next.get(&nid) {
                    worklist.push(next_nid);
                }
                if let Some(&init_nid) = state_init.get(&nid) {
                    worklist.push(init_nid);
                }
            }
        }
    }

    // Also include all init/next/bad/constraint nodes whose state/arg is in cone
    let mut keep: HashSet<u32> = in_cone.clone();
    for node in &model.nodes {
        match node.op.as_str() {
            "init" | "next" => {
                if !node.args.is_empty() {
                    let state_nid = node.args[0].unsigned_abs() as u32;
                    if in_cone.contains(&state_nid) {
                        keep.insert(node.nid);
                    }
                }
            }
            "bad" | "constraint" => {
                keep.insert(node.nid);
            }
            _ => {}
        }
    }

    // Filter nodes to only those in the cone
    model.nodes.retain(|n| keep.contains(&n.nid));

    original_nodes - model.nodes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = "\
; Simple test
1 sort bitvec 2
2 sort bitvec 1
3 input 1 x
4 input 1 y
5 add 1 3 4
6 constd 1 3
7 eq 2 5 6
8 bad 7
";
        let model = parse_btor2(input).unwrap();
        assert_eq!(model.sorts.len(), 2);
        assert_eq!(model.bad_properties, vec![7]);
    }
}
