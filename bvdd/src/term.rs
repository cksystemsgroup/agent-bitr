use std::collections::HashMap;
use crate::types::{TermId, OpKind, BvWidth};

/// Maximum number of arguments per operator application
pub const TERM_MAX_ARGS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TermKind {
    /// Domain constant value
    Const(u64),
    /// Variable (var_id from BTOR2)
    Var(u32),
    /// Operator application
    App {
        op: OpKind,
        args: Vec<TermId>,
        /// For OP_SLICE: upper and lower bit indices
        slice_upper: u16,
        slice_lower: u16,
    },
}

#[derive(Debug, Clone)]
pub struct Term {
    pub kind: TermKind,
    pub width: BvWidth,
}

/// Hash-consed term table: all terms stored uniquely
pub struct TermTable {
    terms: Vec<Term>,
    /// Unique table: (kind, width) -> TermId
    unique: HashMap<(TermKind, BvWidth), TermId>,
}

impl Default for TermTable {
    fn default() -> Self { Self::new() }
}

impl TermTable {
    pub fn new() -> Self {
        TermTable {
            terms: Vec::new(),
            unique: HashMap::new(),
        }
    }

    pub fn get(&self, id: TermId) -> &Term {
        &self.terms[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Intern a term, returning its unique ID
    pub fn intern(&mut self, kind: TermKind, width: BvWidth) -> TermId {
        let key = (kind.clone(), width);
        if let Some(&id) = self.unique.get(&key) {
            return id;
        }
        let id = TermId(self.terms.len() as u32);
        self.terms.push(Term { kind: kind.clone(), width });
        self.unique.insert(key, id);
        id
    }

    /// Create a constant term
    pub fn make_const(&mut self, val: u64, width: BvWidth) -> TermId {
        self.intern(TermKind::Const(val), width)
    }

    /// Create a variable term
    pub fn make_var(&mut self, var_id: u32, width: BvWidth) -> TermId {
        self.intern(TermKind::Var(var_id), width)
    }

    /// Create an operator application term
    pub fn make_app(&mut self, op: OpKind, args: Vec<TermId>, width: BvWidth) -> TermId {
        self.intern(TermKind::App { op, args, slice_upper: 0, slice_lower: 0 }, width)
    }

    /// Create a slice term
    pub fn make_slice(&mut self, arg: TermId, upper: u16, lower: u16) -> TermId {
        let width = upper - lower + 1;
        self.intern(
            TermKind::App {
                op: OpKind::Slice,
                args: vec![arg],
                slice_upper: upper,
                slice_lower: lower,
            },
            width,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consing() {
        let mut tt = TermTable::new();
        let x1 = tt.make_var(0, 8);
        let x2 = tt.make_var(0, 8);
        assert_eq!(x1, x2); // same variable -> same ID
        let y = tt.make_var(1, 8);
        assert_ne!(x1, y);
    }

    #[test]
    fn test_const() {
        let mut tt = TermTable::new();
        let c1 = tt.make_const(42, 8);
        let c2 = tt.make_const(42, 8);
        assert_eq!(c1, c2);
        let c3 = tt.make_const(43, 8);
        assert_ne!(c1, c3);
    }
}
