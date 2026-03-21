# program.md — BITR Solver Task Specification

## Goal

Build a competitive BTOR2 model checker based on Bitvector Decision Diagrams (BVDDs). The solver should handle both bitvector and array benchmarks from HWMCC'24, with bitwuzla as the reference solver.

## Architecture

- **`bvdd/`**: Standalone BVDD library — value sets, terms, constraints, BVCs, BVDD nodes, HSC
- **`bitr/`**: BTOR2 solver — parser, Canonicalize/Solve engine, theory blasting, BMC loop
- See `.claude/commands/bitr-expert.md` for complete algorithmic reference

## Current Status

**Phase**: 0 — Scaffolding
**Last updated**: 2026-03-21

- [x] Repository structure created
- [x] Cargo workspace configured
- [x] Core types defined (`bvdd/src/types.rs`)
- [x] ValueSet with tests (`bvdd/src/valueset.rs`)
- [x] Term table with hash consing (`bvdd/src/term.rs`)
- [x] Constraint table with simplification (`bvdd/src/constraint.rs`)
- [x] BVC manager stub (`bvdd/src/bvc.rs`)
- [x] BVDD node representation (`bvdd/src/bvdd.rs`)
- [x] HSC stub (`bvdd/src/hsc.rs`)
- [x] BTOR2 parser (`bitr/src/btor2.rs`)
- [x] CLI entry point (`bitr/src/main.rs`)
- [x] Tiny benchmarks committed
- [ ] `cargo test` all passing
- [ ] `cargo clippy` clean

## Implementation Phases

### Phase 1: Value Sets + Terms (bvdd/)
- Finalize ValueSet operations (tested)
- Complete TermTable with SubstAndFold
- Add term evaluation (recursive + compiled)
- Unit tests for all operations

### Phase 2: Constraints + BVCs (bvdd/)
- Full constraint table with hash consing
- BVC Apply operation (cross-product)
- BVC from BTOR2 operators
- Lifted variable definitions

### Phase 3: BVDD Core (bvdd/)
- Unique table for hash consing
- Apply operation
- Edge merging (same-child → bitmask OR)
- HSC cascade construction
- Computed cache (direct-mapped)
- C API header generation

### Phase 4: BTOR2 Lifting
- BTOR2 parser → BVC construction
- BVC → BVDD lifting (lazy mode)
- Handle all BTOR2 operators
- Validate on tiny benchmarks

### Phase 5: Canonicalize/Solve
- Implement 6-phase Canonicalize
- Implement Solve (decision node traversal)
- Decide (predicate selection + partition)
- Restrict (constraint restriction)
- Test on tiny sat/unsat benchmarks

### Phase 6: Theory Resolution
- Boolean decomposition (1-bit terms)
- Generalized blast (variable enumeration)
- Compiled evaluator
- Domain budget checks

### Phase 7: External Oracle
- Bitwuzla integration via C FFI
- Oracle caching
- Byte-blast oracle (recursive splitting)
- Budget management (depth, time)

### Phase 8: BMC Loop
- State unrolling
- Init/Next substitution
- Bad property checking per step
- Counterexample extraction

### Phase 9: Array Support
- WRITE → READ-over-WRITE expansion
- ITE chain construction for READ
- Array-track benchmark testing

### Phase 10+: Optimization
- Profile hot paths
- SIMD for ValueSet operations
- Tune domain budgets
- Maximize HWMCC'24 solved count

## Evaluation

- **Timeout**: 300s per benchmark
- **Memory**: 8GB limit
- **Correctness**: paramount — wrong answers are fatal
- **Primary metric**: number of benchmarks solved correctly
- **Secondary metric**: total solving time (PAR-2)

## Experiment Protocol

Before each benchmark run:
1. Note the change being evaluated
2. Record git commit hash
3. Run full benchmark suite with timeout
4. Record results in `results/` as CSV
5. Append summary to `experiments.log`
6. Compare against previous best and reference solver
