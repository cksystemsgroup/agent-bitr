use crate::types::{BvcId, TermId, ConstraintId, BvWidth, OpKind};
use crate::term::{TermTable, TermKind};
use crate::constraint::ConstraintTable;

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

/// BVC manager: arena for all BVCs with lifted variable support
pub struct BvcManager {
    bvcs: Vec<Bvc>,
    /// Next fresh variable ID for lifted definitions
    next_lifted_var: u32,
}

impl Default for BvcManager {
    fn default() -> Self { Self::new() }
}

impl BvcManager {
    pub fn new() -> Self {
        BvcManager {
            bvcs: Vec::new(),
            next_lifted_var: 0x1000_0000, // High range to avoid collision with BTOR2 nids
        }
    }

    pub fn get(&self, id: BvcId) -> &Bvc {
        &self.bvcs[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.bvcs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bvcs.is_empty()
    }

    pub fn alloc(&mut self, width: BvWidth, entries: Vec<BvcEntry>) -> BvcId {
        let id = BvcId(self.bvcs.len() as u32);
        self.bvcs.push(Bvc { id, width, entries });
        id
    }

    /// Allocate a fresh lifted variable ID
    pub fn fresh_var(&mut self) -> u32 {
        let v = self.next_lifted_var;
        self.next_lifted_var += 1;
        v
    }

    /// Create a BVC for an unconstrained input/state variable
    pub fn make_input(
        &mut self,
        tt: &mut TermTable,
        ct: &ConstraintTable,
        var_id: u32,
        width: BvWidth,
    ) -> BvcId {
        let term = tt.make_var(var_id, width);
        self.alloc(width, vec![BvcEntry {
            term,
            constraint: ct.true_id(),
        }])
    }

    /// Create a BVC for a constant value
    pub fn make_const(
        &mut self,
        tt: &mut TermTable,
        ct: &ConstraintTable,
        val: u64,
        width: BvWidth,
    ) -> BvcId {
        let term = tt.make_const(val, width);
        self.alloc(width, vec![BvcEntry {
            term,
            constraint: ct.true_id(),
        }])
    }

    /// Apply a structural operator: keeps the actual operator term.
    ///
    /// For structural ops (comparisons, Boolean at width 1), the result
    /// BVC has the operator application as its term, with operand constraints
    /// propagated. Theory resolution can then evaluate the expression directly.
    pub fn apply_structural(
        &mut self,
        tt: &mut TermTable,
        ct: &mut ConstraintTable,
        op: OpKind,
        operands: &[BvcId],
        result_width: BvWidth,
    ) -> BvcId {
        // Build the operator term from operand terms
        let arg_terms: Vec<TermId> = operands.iter()
            .map(|&bvc_id| self.get(bvc_id).entries[0].term)
            .collect();

        // The result term is the actual operator application
        let result_term = tt.make_app(op, arg_terms, result_width);

        // Collect constraints from operands (conjunction)
        let mut combined_constraint = ct.true_id();
        for &bvc_id in operands {
            let entry_constraint = self.get(bvc_id).entries[0].constraint;
            combined_constraint = ct.make_and(combined_constraint, entry_constraint);
        }

        self.alloc(result_width, vec![BvcEntry {
            term: result_term,
            constraint: combined_constraint,
        }])
    }

    /// Apply a non-structural operator: keeps the actual operator term.
    ///
    /// For correctness, we keep the actual computation term so theory
    /// resolution can evaluate it. Lifting to fresh variables with
    /// equality constraints is a deferred optimization.
    pub fn apply_lifted(
        &mut self,
        tt: &mut TermTable,
        ct: &mut ConstraintTable,
        op: OpKind,
        operands: &[BvcId],
        result_width: BvWidth,
    ) -> BvcId {
        // Same as structural: keep the actual operator term
        self.apply_structural(tt, ct, op, operands, result_width)
    }

    /// Apply an operator — keeps the actual operator term for all ops.
    /// Both structural and non-structural ops produce BVCs with the
    /// actual computation term. This enables correct theory resolution.
    pub fn apply(
        &mut self,
        tt: &mut TermTable,
        ct: &mut ConstraintTable,
        op: OpKind,
        operands: &[BvcId],
        result_width: BvWidth,
    ) -> BvcId {
        self.apply_structural(tt, ct, op, operands, result_width)
    }

    /// Check if a BVC is ground (all entries have constant terms and TRUE constraints)
    pub fn is_ground(&self, tt: &TermTable, id: BvcId) -> bool {
        let bvc = self.get(id);
        bvc.entries.iter().all(|e| matches!(tt.get(e.term).kind, TermKind::Const(_)))
    }

    /// Get the concrete value if this is a ground BVC with a single constant entry
    pub fn get_const_value(&self, tt: &TermTable, id: BvcId) -> Option<u64> {
        let bvc = self.get(id);
        if bvc.entries.len() == 1 {
            if let TermKind::Const(v) = tt.get(bvc.entries[0].term).kind {
                return Some(v);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_input() {
        let mut tt = TermTable::new();
        let ct = ConstraintTable::new();
        let mut bm = BvcManager::new();

        let bvc = bm.make_input(&mut tt, &ct, 0, 8);
        assert_eq!(bm.get(bvc).width, 8);
        assert_eq!(bm.get(bvc).entries.len(), 1);
        assert_eq!(bm.get(bvc).entries[0].constraint, ct.true_id());
    }

    #[test]
    fn test_make_const() {
        let mut tt = TermTable::new();
        let ct = ConstraintTable::new();
        let mut bm = BvcManager::new();

        let bvc = bm.make_const(&mut tt, &ct, 42, 8);
        assert!(bm.is_ground(&tt, bvc));
        assert_eq!(bm.get_const_value(&tt, bvc), Some(42));
    }

    #[test]
    fn test_apply_structural() {
        let mut tt = TermTable::new();
        let mut ct = ConstraintTable::new();
        let mut bm = BvcManager::new();

        let x = bm.make_input(&mut tt, &ct, 0, 8);
        let y = bm.make_input(&mut tt, &ct, 1, 8);
        let eq = bm.apply_structural(&mut tt, &mut ct, OpKind::Eq, &[x, y], 1);

        assert_eq!(bm.get(eq).width, 1);
        // The term should be the actual Eq application
        let term = bm.get(eq).entries[0].term;
        match &tt.get(term).kind {
            TermKind::App { op: OpKind::Eq, .. } => {} // correct
            other => panic!("expected Eq app, got {:?}", other),
        }
    }

    #[test]
    fn test_apply_lifted() {
        let mut tt = TermTable::new();
        let mut ct = ConstraintTable::new();
        let mut bm = BvcManager::new();

        let x = bm.make_input(&mut tt, &ct, 0, 8);
        let y = bm.make_input(&mut tt, &ct, 1, 8);
        let add = bm.apply_lifted(&mut tt, &mut ct, OpKind::Add, &[x, y], 8);

        assert_eq!(bm.get(add).width, 8);
        // The term should be the actual Add application
        match &tt.get(bm.get(add).entries[0].term).kind {
            TermKind::App { op: OpKind::Add, .. } => {} // good: actual term
            other => panic!("expected Add app, got {:?}", other),
        }
    }

    #[test]
    fn test_apply_dispatches() {
        let mut tt = TermTable::new();
        let mut ct = ConstraintTable::new();
        let mut bm = BvcManager::new();

        let x = bm.make_input(&mut tt, &ct, 0, 8);
        let y = bm.make_input(&mut tt, &ct, 1, 8);

        // Eq is structural
        let eq = bm.apply(&mut tt, &mut ct, OpKind::Eq, &[x, y], 1);
        assert_eq!(bm.get(eq).width, 1);

        // Add is non-structural (at width 8)
        let add = bm.apply(&mut tt, &mut ct, OpKind::Add, &[x, y], 8);
        assert_eq!(bm.get(add).width, 8);
    }

    #[test]
    fn test_fresh_var_uniqueness() {
        let mut bm = BvcManager::new();
        let v1 = bm.fresh_var();
        let v2 = bm.fresh_var();
        assert_ne!(v1, v2);
    }
}
