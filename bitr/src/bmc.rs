//! Bounded Model Checking (BMC) loop
//!
//! Unrolls transition relation for k steps, checking bad properties
//! at each step.

use bvdd::types::SolveResult;

/// BMC configuration
pub struct BmcConfig {
    pub max_bound: u32,
    pub timeout_s: f64,
}

impl Default for BmcConfig {
    fn default() -> Self {
        BmcConfig {
            max_bound: 100,
            timeout_s: 300.0,
        }
    }
}

/// Run bounded model checking
pub fn bmc_check(_config: &BmcConfig) -> SolveResult {
    // TODO: implement BMC loop
    // For each step k:
    //   1. Create fresh state variables
    //   2. Substitute next-state functions
    //   3. Check bad properties via BITR solver
    //   4. If SAT -> counterexample found
    //   5. If all UNSAT -> continue to k+1
    SolveResult::Unknown
}
