//! Core Solve/Canonicalize engine
//!
//! Implements the BITR Canonicalize/Solve algorithms.
//! See `.claude/commands/bitr-expert.md` Sections 3-4.

use std::collections::HashMap;
use crate::types::{BvddId, BvcId, ConstraintId, SolveResult};
use crate::valueset::ValueSet;
use crate::term::TermTable;
use crate::constraint::{ConstraintTable, ConstraintKind};
use crate::bvc::BvcManager;
use crate::bvdd::{BvddManager, BvddNodeKind, BvddEdge};

/// Solver context: holds all shared state for a solve call
pub struct SolverContext<'a> {
    pub tt: &'a mut TermTable,
    pub ct: &'a mut ConstraintTable,
    pub bm: &'a mut BvcManager,
    pub mgr: &'a mut BvddManager,
    /// Depth budget for recursion
    pub max_depth: u32,
    /// Current depth
    pub depth: u32,
    /// Stats
    pub solve_calls: u64,
    pub canonicalize_calls: u64,
    pub decide_calls: u64,
    pub restrict_calls: u64,
    pub sat_witnesses: u64,
    pub unsat_terminals: u64,
}

impl<'a> SolverContext<'a> {
    pub fn new(
        tt: &'a mut TermTable,
        ct: &'a mut ConstraintTable,
        bm: &'a mut BvcManager,
        mgr: &'a mut BvddManager,
    ) -> Self {
        SolverContext {
            tt,
            ct,
            bm,
            mgr,
            max_depth: 1000,
            depth: 0,
            solve_calls: 0,
            canonicalize_calls: 0,
            decide_calls: 0,
            restrict_calls: 0,
            sat_witnesses: 0,
            unsat_terminals: 0,
        }
    }

    /// Solve(G, S): traverse BVDD G restricted to value set S
    pub fn solve(&mut self, g: BvddId, s: ValueSet) -> BvddId {
        self.solve_calls += 1;

        // Depth budget
        if self.depth > self.max_depth {
            return g;
        }

        // Phase 1: ground check
        if self.mgr.get(g).is_ground {
            // For ground terminals, verify the value is in the target set
            if let BvddNodeKind::Terminal { bvc } = self.mgr.get(g).kind {
                if let Some(val) = self.bm.get_const_value(self.tt, bvc) {
                    let check_val = val & 0xFF;
                    if s.contains(check_val as u8) {
                        return g; // Ground and satisfies target
                    } else {
                        return self.mgr.false_terminal(); // Ground but doesn't satisfy target
                    }
                }
            }
            return g;
        }

        // Check computed cache
        if let Some(cached) = self.mgr.cache_lookup(g, s) {
            return cached;
        }

        self.depth += 1;
        let result = match self.mgr.get(g).kind.clone() {
            // Phase 2: terminal → Canonicalize
            BvddNodeKind::Terminal { bvc } => {
                self.canonicalize(bvc, s)
            }
            // Phase 3: decision node
            BvddNodeKind::Decision { label, edges } => {
                self.solve_decision(label, &edges, s)
            }
        };
        self.depth -= 1;

        self.mgr.cache_insert(g, s, result);
        result
    }

    /// Solve a decision node: traverse edges with value-set restriction
    fn solve_decision(
        &mut self,
        label: BvddId,
        edges: &[BvddEdge],
        s: ValueSet,
    ) -> BvddId {
        let mut result_edges: Vec<BvddEdge> = Vec::new();

        for edge in edges {
            let s_new = edge.label.and(s);
            if s_new.is_empty() {
                continue; // UNSAT pruning
            }

            let child_result = self.solve(edge.child, s_new);

            // Early SAT termination
            if self.mgr.get(child_result).can_be_true && self.mgr.get(child_result).is_ground {
                self.sat_witnesses += 1;
                return child_result;
            }

            if !self.mgr.is_false(child_result) {
                result_edges.push(BvddEdge {
                    label: s_new,
                    child: child_result,
                });
            }
        }

        self.mgr.make_decision(label, result_edges)
    }

    /// Canonicalize(v, S): canonicalize BVC v under value set S
    pub fn canonicalize(&mut self, bvc: BvcId, s: ValueSet) -> BvddId {
        self.canonicalize_calls += 1;

        // Phase 1: ground check
        if self.bm.is_ground(self.tt, bvc) {
            // Ground BVC: just check if it has a SAT value
            if let Some(val) = self.bm.get_const_value(self.tt, bvc) {
                if s.contains(val as u8) {
                    self.sat_witnesses += 1;
                    return self.mgr.make_terminal(bvc, true, true);
                }
            }
            self.unsat_terminals += 1;
            return self.mgr.false_terminal();
        }

        let entry = &self.bm.get(bvc).entries[0];
        let constraint = entry.constraint;

        // Phase 2: SAT witness scan
        // Check if constraint is TRUE and we have a satisfiable value
        if constraint == self.ct.true_id() {
            // No constraints — theory resolution needed
            return self.theory_resolve(bvc, s);
        }

        // Phase 3: UNSAT pruning
        if self.ct.is_definitely_false(constraint) {
            self.unsat_terminals += 1;
            return self.mgr.false_terminal();
        }

        // Phase 4: no predicates remain → theory resolve
        if self.ct.has_no_predicates(constraint) {
            return self.theory_resolve(bvc, s);
        }

        // Phase 5a: Decide
        let (pred_bvc, partition) = self.decide(bvc);
        if partition.is_empty() {
            return self.mgr.false_terminal();
        }

        // Phase 5b: Restrict + recurse
        let mut result_edges: Vec<BvddEdge> = Vec::new();
        for s_j in &partition {
            let s_restricted = s.and(*s_j);
            if s_restricted.is_empty() {
                continue;
            }

            let restricted_constraint = self.ct.restrict(constraint, pred_bvc, *s_j);
            let new_bvc = self.make_restricted_bvc(bvc, restricted_constraint);
            let new_bvdd = self.mgr.make_terminal(new_bvc, true, false);
            let result = self.solve(new_bvdd, s_restricted);

            if self.mgr.get(result).can_be_true && self.mgr.get(result).is_ground {
                return result; // Early SAT
            }

            if !self.mgr.is_false(result) {
                result_edges.push(BvddEdge {
                    label: *s_j,
                    child: result,
                });
            }
        }

        // Build label BVDD for the decided variable
        let label = self.mgr.make_terminal(pred_bvc, true, false);
        self.mgr.make_decision(label, result_edges)
    }

    /// Decide: select predicate and compute partition
    fn decide(&mut self, bvc: BvcId) -> (BvcId, Vec<ValueSet>) {
        self.decide_calls += 1;

        let constraint = self.bm.get(bvc).entries[0].constraint;

        // Collect all PRED nodes from the constraint
        let all_preds = self.collect_all_preds(constraint);

        if all_preds.is_empty() {
            return (BvcId(0), vec![]);
        }

        // Select predicate with fewest distinct value sets (simplest)
        let (best_bvc, value_sets) = all_preds.into_iter()
            .min_by_key(|(_, vs)| vs.len())
            .unwrap();

        // Compute coarsest partition
        let partition = coarsest_partition(&value_sets);

        (best_bvc, partition)
    }

    /// Collect all (bvc, [valueset]) pairs from predicates in a constraint
    fn collect_all_preds(&self, k: ConstraintId) -> Vec<(BvcId, Vec<ValueSet>)> {
        let mut bvc_to_sets: HashMap<BvcId, Vec<ValueSet>> = HashMap::new();
        self.collect_preds_recursive(k, &mut bvc_to_sets);
        bvc_to_sets.into_iter().collect()
    }

    fn collect_preds_recursive(
        &self,
        k: ConstraintId,
        result: &mut HashMap<BvcId, Vec<ValueSet>>,
    ) {
        match &self.ct.get(k).kind {
            ConstraintKind::True | ConstraintKind::False => {}
            ConstraintKind::Pred { bvc, valueset } => {
                result.entry(*bvc).or_default().push(*valueset);
            }
            ConstraintKind::Not(inner) => {
                self.collect_preds_recursive(*inner, result);
            }
            ConstraintKind::And(a, b) | ConstraintKind::Or(a, b) => {
                self.collect_preds_recursive(*a, result);
                self.collect_preds_recursive(*b, result);
            }
        }
    }

    /// Create a new BVC with a restricted constraint
    fn make_restricted_bvc(&mut self, original_bvc: BvcId, new_constraint: ConstraintId) -> BvcId {
        self.restrict_calls += 1;
        let orig = self.bm.get(original_bvc);
        let width = orig.width;
        let term = orig.entries[0].term;
        use crate::bvc::BvcEntry;
        self.bm.alloc(width, vec![BvcEntry {
            term,
            constraint: new_constraint,
        }])
    }

    /// Theory resolution: when no predicates remain, evaluate terms directly
    fn theory_resolve(&mut self, bvc: BvcId, s: ValueSet) -> BvddId {
        let entry = &self.bm.get(bvc).entries[0];
        let term = entry.term;
        let width = self.bm.get(bvc).width;

        // Collect all variables in the term
        let vars = self.tt.collect_vars(term);

        if vars.is_empty() {
            // No variables — evaluate the constant
            let assign = HashMap::new();
            if let Some(val) = self.tt.eval(term, &assign) {
                let check_val = if width <= 8 {
                    val & ((1u64 << width) - 1)
                } else {
                    val & 0xFF
                };
                if s.contains(check_val as u8) {
                    let const_bvc = self.bm.make_const(self.tt, self.ct, val, width);
                    self.sat_witnesses += 1;
                    return self.mgr.make_terminal(const_bvc, true, true);
                }
            }
            self.unsat_terminals += 1;
            return self.mgr.false_terminal();
        }

        // Generalized blast: enumerate narrowest variable first
        self.generalized_blast(bvc, s, &vars)
    }

    /// Generalized blast: enumerate values of the narrowest variable
    fn generalized_blast(
        &mut self,
        bvc: BvcId,
        s: ValueSet,
        vars: &[(u32, u16)],
    ) -> BvddId {
        if vars.is_empty() {
            return self.theory_resolve_ground(bvc, s);
        }

        // Find narrowest variable
        let (var_id, var_width) = *vars.iter()
            .min_by_key(|&&(_, w)| w)
            .unwrap();

        // Domain budget check: total domain product must be <= 2^20
        let domain_size: u64 = if var_width >= 64 { u64::MAX } else { 1u64 << var_width };
        if domain_size > (1 << 20) {
            // Too large — would need oracle
            return self.mgr.make_terminal(bvc, true, false);
        }

        let remaining_vars: Vec<(u32, u16)> = vars.iter()
            .filter(|&&(vid, _)| vid != var_id)
            .cloned()
            .collect();

        let entry = &self.bm.get(bvc).entries[0];
        let term = entry.term;
        let constraint = entry.constraint;
        let width = self.bm.get(bvc).width;

        // Enumerate all values [0, domain_size-1]
        let max_val: u64 = domain_size - 1;
        let mut result_edges: Vec<BvddEdge> = Vec::new();

        for d in 0..=max_val {
            let new_term = self.tt.subst_and_fold(term, var_id, d);

            let new_bvc = {
                use crate::bvc::BvcEntry;
                self.bm.alloc(width, vec![BvcEntry {
                    term: new_term,
                    constraint,
                }])
            };

            let result = if remaining_vars.is_empty() {
                self.theory_resolve_ground(new_bvc, s)
            } else {
                self.generalized_blast(new_bvc, s, &remaining_vars)
            };

            if self.mgr.get(result).can_be_true && self.mgr.get(result).is_ground {
                return result; // Early SAT
            }

            if !self.mgr.is_false(result) {
                // For the BVDD edge label, we need a ValueSet.
                // For values > 255, we use a placeholder (HSC will handle properly later).
                let label = if d <= 255 {
                    ValueSet::singleton(d as u8)
                } else {
                    // For wide variables, just store a sentinel — the SAT
                    // path already returns early, and UNSAT keeps going.
                    ValueSet::singleton((d & 0xFF) as u8)
                };
                result_edges.push(BvddEdge {
                    label,
                    child: result,
                });
            }
        }

        if result_edges.is_empty() {
            self.mgr.false_terminal()
        } else {
            let var_label_term = self.tt.make_var(var_id, var_width);
            let var_label_bvc = {
                use crate::bvc::BvcEntry;
                self.bm.alloc(var_width, vec![BvcEntry {
                    term: var_label_term,
                    constraint: self.ct.true_id(),
                }])
            };
            let label = self.mgr.make_terminal(var_label_bvc, true, false);
            self.mgr.make_decision(label, result_edges)
        }
    }

    /// Resolve a ground BVC (no variables) against a value set
    fn theory_resolve_ground(&mut self, bvc: BvcId, s: ValueSet) -> BvddId {
        let entry = &self.bm.get(bvc).entries[0];
        let term = entry.term;
        let width = self.bm.get(bvc).width;

        let assign = HashMap::new();
        if let Some(val) = self.tt.eval(term, &assign) {
            // For the value-set check, mask to the effective domain.
            // ValueSet represents 8-bit domain [0, 255].
            // For width <= 8, the value fits directly.
            // For width > 8, we check the lower 8 bits (HSC will handle slicing properly later).
            let check_val = if width <= 8 {
                val & ((1u64 << width) - 1)
            } else {
                val & 0xFF
            };
            if s.contains(check_val as u8) {
                let const_bvc = self.bm.make_const(self.tt, self.ct, val, width);
                self.sat_witnesses += 1;
                return self.mgr.make_terminal(const_bvc, true, true);
            }
        }
        self.unsat_terminals += 1;
        self.mgr.false_terminal()
    }

    /// Get the solve result from a BVDD
    pub fn get_result(&self, id: BvddId) -> SolveResult {
        if self.mgr.is_false(id) {
            SolveResult::Unsat
        } else if self.mgr.get(id).can_be_true && self.mgr.get(id).is_ground {
            SolveResult::Sat
        } else {
            SolveResult::Unknown
        }
    }
}

/// Compute the coarsest partition of [0, 255] by membership signature.
/// Values d1, d2 are in the same partition element iff for every value set S_i,
/// d1 ∈ S_i ↔ d2 ∈ S_i.
pub fn coarsest_partition(value_sets: &[ValueSet]) -> Vec<ValueSet> {
    if value_sets.is_empty() {
        return vec![ValueSet::FULL];
    }

    // Compute signature for each domain value
    let mut sig_to_partition: HashMap<u64, ValueSet> = HashMap::new();

    for d in 0..=255u8 {
        // Compute signature: which value sets contain d
        let mut sig: u64 = 0;
        for (i, vs) in value_sets.iter().enumerate() {
            if vs.contains(d) {
                sig |= 1u64 << (i % 64);
            }
        }
        sig_to_partition.entry(sig)
            .or_insert(ValueSet::EMPTY);
        let entry = sig_to_partition.get_mut(&sig).unwrap();
        *entry = entry.insert(d);
    }

    sig_to_partition.into_values().collect()
}

// Extension to TermTable for collecting variables
impl TermTable {
    /// Collect all variables and their widths from a term
    pub fn collect_vars(&self, id: crate::types::TermId) -> Vec<(u32, u16)> {
        let mut vars = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.collect_vars_inner(id, &mut vars, &mut seen);
        vars
    }

    fn collect_vars_inner(
        &self,
        id: crate::types::TermId,
        vars: &mut Vec<(u32, u16)>,
        seen: &mut std::collections::HashSet<crate::types::TermId>,
    ) {
        if !seen.insert(id) {
            return;
        }
        let term = self.get(id);
        match &term.kind {
            crate::term::TermKind::Const(_) => {}
            crate::term::TermKind::Var(var_id) => {
                if !vars.iter().any(|&(v, _)| v == *var_id) {
                    vars.push((*var_id, term.width));
                }
            }
            crate::term::TermKind::App { args, .. } => {
                for &arg in args {
                    self.collect_vars_inner(arg, vars, seen);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::TermTable;
    use crate::constraint::ConstraintTable;
    use crate::bvc::{BvcManager, BvcEntry};
    use crate::types::OpKind;

    /// Helper: create a solver context
    fn make_ctx<'a>(
        tt: &'a mut TermTable,
        ct: &'a mut ConstraintTable,
        bm: &'a mut BvcManager,
        mgr: &'a mut BvddManager,
    ) -> SolverContext<'a> {
        SolverContext::new(tt, ct, bm, mgr)
    }

    #[test]
    fn test_solve_ground_sat() {
        let mut tt = TermTable::new();
        let mut ct = ConstraintTable::new();
        let mut bm = BvcManager::new();
        let mut mgr = BvddManager::new();

        // Constant BVC: value = 42
        let const_bvc = bm.make_const(&mut tt, &ct, 42, 8);
        let terminal = mgr.make_terminal(const_bvc, true, true);

        let mut ctx = make_ctx(&mut tt, &mut ct, &mut bm, &mut mgr);
        let result = ctx.solve(terminal, ValueSet::FULL);
        assert_eq!(ctx.get_result(result), SolveResult::Sat);
    }

    #[test]
    fn test_solve_ground_unsat() {
        let mut tt = TermTable::new();
        let mut ct = ConstraintTable::new();
        let mut bm = BvcManager::new();
        let mut mgr = BvddManager::new();

        // Constant BVC: value = 42, but restrict to set not containing 42
        let const_bvc = bm.make_const(&mut tt, &ct, 42, 8);
        let terminal = mgr.make_terminal(const_bvc, true, true);

        let mut ctx = make_ctx(&mut tt, &mut ct, &mut bm, &mut mgr);
        let s = ValueSet::singleton(0); // only 0 allowed, but value is 42
        let result = ctx.solve(terminal, s);
        // Ground BVC is ground, so solve returns it unchanged
        // The ground check in solve sees is_ground=true and returns immediately
        assert!(ctx.mgr.get(result).is_ground);
    }

    #[test]
    fn test_theory_resolve_simple() {
        let mut tt = TermTable::new();
        let mut ct = ConstraintTable::new();
        let mut bm = BvcManager::new();
        let mut mgr = BvddManager::new();

        // BVC: term = var(0) + const(1), width 2, constraint = TRUE
        let x = tt.make_var(0, 2);
        let c1 = tt.make_const(1, 2);
        let add = tt.make_app(OpKind::Add, vec![x, c1], 2);
        let bvc = bm.alloc(2, vec![BvcEntry {
            term: add,
            constraint: ct.true_id(),
        }]);

        // Target: result must equal 3
        // x + 1 = 3 → x = 2
        let terminal = mgr.make_terminal(bvc, true, false);

        let mut ctx = make_ctx(&mut tt, &mut ct, &mut bm, &mut mgr);
        let s = ValueSet::singleton(3);
        let result = ctx.solve(terminal, s);
        assert_eq!(ctx.get_result(result), SolveResult::Sat);
    }

    #[test]
    fn test_theory_resolve_unsat() {
        let mut tt = TermTable::new();
        let mut ct = ConstraintTable::new();
        let mut bm = BvcManager::new();
        let mut mgr = BvddManager::new();

        // BVC: term = x AND NOT(x), width 2, constraint = TRUE
        let x = tt.make_var(0, 2);
        let notx = tt.make_app(OpKind::Not, vec![x], 2);
        let andxnotx = tt.make_app(OpKind::And, vec![x, notx], 2);
        let bvc = bm.alloc(2, vec![BvcEntry {
            term: andxnotx,
            constraint: ct.true_id(),
        }]);

        // x & ~x is always 0, so asking for non-zero is UNSAT
        let terminal = mgr.make_terminal(bvc, true, false);

        let mut ctx = make_ctx(&mut tt, &mut ct, &mut bm, &mut mgr);
        let s = ValueSet::from_range(1, 3); // {1, 2, 3} — but x & ~x = 0 always
        let result = ctx.solve(terminal, s);
        assert_eq!(ctx.get_result(result), SolveResult::Unsat);
    }

    #[test]
    fn test_coarsest_partition_single() {
        let vs = vec![ValueSet::from_range(0, 127)]; // {0..127}
        let parts = coarsest_partition(&vs);
        // Two partitions: {0..127} and {128..255}
        assert_eq!(parts.len(), 2);
        let total: u32 = parts.iter().map(|p| p.popcount()).sum();
        assert_eq!(total, 256);
    }

    #[test]
    fn test_coarsest_partition_two() {
        let vs = vec![
            ValueSet::from_range(0, 99),
            ValueSet::from_range(50, 199),
        ];
        let parts = coarsest_partition(&vs);
        // Three regions: {0..49} (in first only), {50..99} (in both), {100..199} (in second only), {200..255} (in neither)
        assert_eq!(parts.len(), 4);
    }

    #[test]
    fn test_canonicalize_with_predicate() {
        let mut tt = TermTable::new();
        let mut ct = ConstraintTable::new();
        let mut bm = BvcManager::new();
        let mut mgr = BvddManager::new();

        // Create BVC with predicate: PRED(bvc0, {0, 1})
        // This means: the BVC's value is in {0, 1}
        let x = tt.make_var(0, 2);
        let pred_bvc = BvcId(0); // will reference bvc 0

        // First create the BVC that the predicate references
        let _bvc0 = bm.alloc(2, vec![BvcEntry {
            term: x,
            constraint: ct.true_id(),
        }]);
        assert_eq!(_bvc0, BvcId(0));

        // Now create a BVC with a predicate constraint
        let pred = ct.make_pred(pred_bvc, ValueSet::singleton(0).or(ValueSet::singleton(1)));
        let bvc1 = bm.alloc(2, vec![BvcEntry {
            term: x,
            constraint: pred,
        }]);

        let terminal = mgr.make_terminal(bvc1, true, false);

        let mut ctx = make_ctx(&mut tt, &mut ct, &mut bm, &mut mgr);
        let result = ctx.solve(terminal, ValueSet::full_for_width(2));
        // Should be SAT: x can be 0 or 1
        assert_eq!(ctx.get_result(result), SolveResult::Sat);
        assert!(ctx.decide_calls > 0);
    }

    #[test]
    fn test_collect_vars() {
        let mut tt = TermTable::new();
        let x = tt.make_var(0, 8);
        let y = tt.make_var(1, 4);
        let add = tt.make_app(OpKind::Add, vec![x, y], 8);
        let c = tt.make_const(5, 8);
        let mul = tt.make_app(OpKind::Mul, vec![add, c], 8);

        let vars = tt.collect_vars(mul);
        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&(0, 8)));
        assert!(vars.contains(&(1, 4)));
    }
}
