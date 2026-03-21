use crate::types::{BvddId, BvcId, Canonicity};
use crate::valueset::ValueSet;

/// An edge in a BVDD decision node
#[derive(Debug, Clone)]
pub struct BvddEdge {
    pub label: ValueSet,
    pub child: BvddId,
}

/// A BVDD node: either terminal (holds BVC) or decision (edges + label)
#[derive(Debug, Clone)]
pub enum BvddNodeKind {
    Terminal {
        bvc: BvcId,
    },
    Decision {
        /// Label BVDD (the variable being decided on)
        label: BvddId,
        edges: Vec<BvddEdge>,
    },
}

#[derive(Debug, Clone)]
pub struct BvddNode {
    pub id: BvddId,
    pub kind: BvddNodeKind,
    /// O(1) flag: can any reachable terminal evaluate to 1?
    pub can_be_true: bool,
    /// O(1) flag: are all terminals variable-free?
    pub is_ground: bool,
    pub canonicity: Canonicity,
}

/// BVDD manager: arena + unique table + computed cache
pub struct BvddManager {
    nodes: Vec<BvddNode>,
    // TODO: unique table (HashMap)
    // TODO: computed cache (direct-mapped)
    // TODO: BVDD ordering
}

impl Default for BvddManager {
    fn default() -> Self { Self::new() }
}

impl BvddManager {
    pub fn new() -> Self {
        BvddManager {
            nodes: Vec::new(),
        }
    }

    pub fn get(&self, id: BvddId) -> &BvddNode {
        &self.nodes[id.0 as usize]
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Allocate a terminal node
    pub fn make_terminal(&mut self, bvc: BvcId, can_be_true: bool, is_ground: bool) -> BvddId {
        let id = BvddId(self.nodes.len() as u32);
        self.nodes.push(BvddNode {
            id,
            kind: BvddNodeKind::Terminal { bvc },
            can_be_true,
            is_ground,
            canonicity: Canonicity::ModuloBvc,
        });
        id
    }

    /// Allocate a decision node
    pub fn make_decision(
        &mut self,
        label: BvddId,
        edges: Vec<BvddEdge>,
        can_be_true: bool,
        is_ground: bool,
    ) -> BvddId {
        // TODO: hash-cons, edge merging
        let id = BvddId(self.nodes.len() as u32);
        self.nodes.push(BvddNode {
            id,
            kind: BvddNodeKind::Decision { label, edges },
            can_be_true,
            is_ground,
            canonicity: Canonicity::ModuloBvc,
        });
        id
    }
}
