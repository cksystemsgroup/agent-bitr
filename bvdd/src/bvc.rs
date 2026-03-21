use crate::types::{BvcId, TermId, ConstraintId, BvWidth};

/// A single BVC entry: (term, constraint)
#[derive(Debug, Clone)]
pub struct BvcEntry {
    pub term: TermId,
    pub constraint: ConstraintId,
}

/// A Bitvector Constraint: set of constrained terms
#[derive(Debug, Clone)]
pub struct Bvc {
    pub id: BvcId,
    pub width: BvWidth,
    pub entries: Vec<BvcEntry>,
}

/// BVC manager: arena for all BVCs
pub struct BvcManager {
    bvcs: Vec<Bvc>,
    // TODO: lifted variable definitions
    // TODO: term table reference
}

impl Default for BvcManager {
    fn default() -> Self { Self::new() }
}

impl BvcManager {
    pub fn new() -> Self {
        BvcManager {
            bvcs: Vec::new(),
        }
    }

    pub fn get(&self, id: BvcId) -> &Bvc {
        &self.bvcs[id.0 as usize]
    }

    pub fn alloc(&mut self, width: BvWidth, entries: Vec<BvcEntry>) -> BvcId {
        let id = BvcId(self.bvcs.len() as u32);
        self.bvcs.push(Bvc { id, width, entries });
        id
    }
}
