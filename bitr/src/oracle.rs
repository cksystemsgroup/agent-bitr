//! External theory oracle integration
//!
//! Provides a trait for theory oracle backends (e.g., Bitwuzla)
//! and a caching layer.

use bvdd::types::{TermId, BvWidth, SolveResult};
use bvdd::valueset::ValueSet;

/// Theory oracle trait: check if a term can produce a value in the target set
pub trait TheoryOracle {
    /// Returns Sat, Unsat, or Unknown
    fn check(&mut self, term: TermId, width: BvWidth, target: ValueSet) -> SolveResult;
}

/// Cached oracle wrapper
pub struct CachedOracle<T: TheoryOracle> {
    inner: T,
    // TODO: HashMap<TermId, SolveResult> cache
}

impl<T: TheoryOracle> CachedOracle<T> {
    pub fn new(inner: T) -> Self {
        CachedOracle { inner }
    }

    pub fn check(&mut self, term: TermId, width: BvWidth, target: ValueSet) -> SolveResult {
        // TODO: check cache first
        self.inner.check(term, width, target)
    }
}
