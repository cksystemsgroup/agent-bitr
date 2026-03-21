use crate::types::{ConstraintId, BvcId};
use crate::valueset::ValueSet;

/// A constraint is a Boolean formula over predicates phi restricted to S
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstraintKind {
    True,
    False,
    /// Predicate: BVC phi has value in set S
    Pred { bvc: BvcId, valueset: ValueSet },
    Not(ConstraintId),
    And(ConstraintId, ConstraintId),
    Or(ConstraintId, ConstraintId),
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub kind: ConstraintKind,
}

/// Constraint table with hash-consing and simplification
pub struct ConstraintTable {
    constraints: Vec<Constraint>,
    // TODO: unique table for hash consing
}

impl Default for ConstraintTable {
    fn default() -> Self { Self::new() }
}

impl ConstraintTable {
    pub fn new() -> Self {
        let mut ct = ConstraintTable {
            constraints: Vec::new(),
        };
        // Pre-allocate TRUE and FALSE at known indices
        let _true_id = ct.alloc(ConstraintKind::True);   // index 0
        let _false_id = ct.alloc(ConstraintKind::False);  // index 1
        ct
    }

    pub fn true_id(&self) -> ConstraintId { ConstraintId(0) }
    pub fn false_id(&self) -> ConstraintId { ConstraintId(1) }

    pub fn get(&self, id: ConstraintId) -> &Constraint {
        &self.constraints[id.0 as usize]
    }

    fn alloc(&mut self, kind: ConstraintKind) -> ConstraintId {
        let id = ConstraintId(self.constraints.len() as u32);
        self.constraints.push(Constraint { kind });
        id
    }

    /// Create a predicate constraint
    pub fn make_pred(&mut self, bvc: BvcId, valueset: ValueSet) -> ConstraintId {
        self.alloc(ConstraintKind::Pred { bvc, valueset })
    }

    /// Boolean AND with simplification
    pub fn make_and(&mut self, a: ConstraintId, b: ConstraintId) -> ConstraintId {
        if a == self.true_id() { return b; }
        if b == self.true_id() { return a; }
        if a == self.false_id() || b == self.false_id() { return self.false_id(); }
        if a == b { return a; }
        self.alloc(ConstraintKind::And(a, b))
    }

    /// Boolean OR with simplification
    pub fn make_or(&mut self, a: ConstraintId, b: ConstraintId) -> ConstraintId {
        if a == self.false_id() { return b; }
        if b == self.false_id() { return a; }
        if a == self.true_id() || b == self.true_id() { return self.true_id(); }
        if a == b { return a; }
        self.alloc(ConstraintKind::Or(a, b))
    }

    /// Boolean NOT with simplification
    pub fn make_not(&mut self, a: ConstraintId) -> ConstraintId {
        if a == self.true_id() { return self.false_id(); }
        if a == self.false_id() { return self.true_id(); }
        // Double negation
        if let ConstraintKind::Not(inner) = self.get(a).kind {
            return inner;
        }
        self.alloc(ConstraintKind::Not(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplification() {
        let mut ct = ConstraintTable::new();
        let t = ct.true_id();
        let f = ct.false_id();

        assert_eq!(ct.make_and(t, f), f);
        assert_eq!(ct.make_and(t, t), t);
        assert_eq!(ct.make_or(f, t), t);
        assert_eq!(ct.make_not(t), f);
        assert_eq!(ct.make_not(f), t);
    }
}
