/// Bitvector width (up to 65535 bits)
pub type BvWidth = u16;

/// Maximum width for single-level BVDD (HSC extends beyond this)
pub const BV_WIDTH_MAX: BvWidth = 8;

/// ID types — arena indices into Vec-based tables
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstraintId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BvcId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BvddId(pub u32);

/// Sentinel for missing IDs
pub const ID_NONE: u32 = u32::MAX;

impl TermId {
    pub const NONE: TermId = TermId(ID_NONE);
}
impl ConstraintId {
    pub const NONE: ConstraintId = ConstraintId(ID_NONE);
}
impl BvcId {
    pub const NONE: BvcId = BvcId(ID_NONE);
}
impl BvddId {
    pub const NONE: BvddId = BvddId(ID_NONE);
}

/// Solver result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveResult {
    Sat,
    Unsat,
    Unknown,
}

/// BTOR2 operator kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpKind {
    // Boolean-result operators
    Eq, Neq,
    Ult, Slt, Ulte, Slte,
    Redand, Redor, Redxor,
    // Arithmetic
    Add, Sub, Mul,
    Udiv, Urem, Sdiv, Srem, Smod,
    // Bitwise
    And, Or, Xor, Not, Neg,
    // Shifts
    Sll, Srl, Sra,
    // Data movement
    Slice, Uext, Sext, Concat,
    // Memory
    Read, Write,
    // Mixed
    Ite,
    // Overflow detection
    Uaddo, Umulo,
}

impl OpKind {
    /// Structural operators produce predicate/constraint structure for Decide.
    /// Non-structural operators are lifted to fresh variables in lazy mode.
    pub fn is_structural(self, result_width: BvWidth) -> bool {
        match self {
            // Comparisons: always structural
            OpKind::Eq | OpKind::Neq |
            OpKind::Ult | OpKind::Slt | OpKind::Ulte | OpKind::Slte |
            OpKind::Uaddo | OpKind::Umulo => true,
            // Reductions: always structural
            OpKind::Redand | OpKind::Redor | OpKind::Redxor => true,
            // ITE: always structural
            OpKind::Ite => true,
            // Boolean/Bitwise: structural only at width 1
            OpKind::And | OpKind::Or | OpKind::Xor | OpKind::Not => result_width == 1,
            // Everything else: arithmetic, shifts, data movement
            _ => false,
        }
    }

    pub fn arity(self) -> usize {
        match self {
            OpKind::Not | OpKind::Neg |
            OpKind::Redand | OpKind::Redor | OpKind::Redxor |
            OpKind::Slice => 1,
            OpKind::Ite | OpKind::Write => 3,
            _ => 2,
        }
    }
}

/// BVDD canonicity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Canonicity {
    /// Only structural invariants (hash consing, edge merging)
    ModuloBvc,
    /// All constraints are top, but terms still symbolic
    ModuloBitvector,
    /// All terminals are value terminals {(d, top)}
    FullyCanonical,
}
