//! Statistics and profiling for BITR solver

use std::fmt;

/// BITR solver statistics
#[derive(Debug, Default)]
pub struct BitrStats {
    // Call counts
    pub canonicalize_calls: u64,
    pub solve_calls: u64,
    pub decide_calls: u64,
    pub restrict_calls: u64,

    // Structural events
    pub edges_pruned: u64,
    pub edges_merged: u64,
    pub sat_witnesses: u64,
    pub unsat_terminals: u64,
    pub nodes_created: u64,

    // Decomposition & oracle
    pub decompose_calls: u64,
    pub oracle_calls: u64,
    pub oracle_cache_hits: u64,
    pub decomp_memo_hits: u64,

    // Blast
    pub blast_calls: u64,
    pub blast_var_elims: u64,

    // Byte-split oracle
    pub bso_calls: u64,
    pub bso_resolved: u64,
    pub bso_bailouts: u64,

    // Depth tracking
    pub max_depth: u32,

    // Timing (seconds)
    pub total_time_s: f64,
}

impl fmt::Display for BitrStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== BITR Statistics ===")?;
        writeln!(f, "Canonicalize calls: {}", self.canonicalize_calls)?;
        writeln!(f, "Solve calls:        {}", self.solve_calls)?;
        writeln!(f, "Decide calls:       {}", self.decide_calls)?;
        writeln!(f, "Restrict calls:     {}", self.restrict_calls)?;
        writeln!(f, "Edges pruned:       {}", self.edges_pruned)?;
        writeln!(f, "Edges merged:       {}", self.edges_merged)?;
        writeln!(f, "SAT witnesses:      {}", self.sat_witnesses)?;
        writeln!(f, "UNSAT terminals:    {}", self.unsat_terminals)?;
        writeln!(f, "Nodes created:      {}", self.nodes_created)?;
        writeln!(f, "Blast calls:        {}", self.blast_calls)?;
        writeln!(f, "Oracle calls:       {}", self.oracle_calls)?;
        writeln!(f, "Max depth:          {}", self.max_depth)?;
        writeln!(f, "Total time:         {:.3}s", self.total_time_s)?;
        Ok(())
    }
}
