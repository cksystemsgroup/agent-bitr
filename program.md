# program.md — BITR Solver Task Specification

## Goal

Build a competitive BTOR2 model checker based on Bitvector Decision Diagrams (BVDDs). The solver should handle both bitvector and array benchmarks from HWMCC'24, with bitwuzla as the reference solver.

## Architecture

- **`bvdd/`**: Standalone BVDD library — value sets, terms, constraints, BVCs, BVDD nodes, HSC
- **`bitr/`**: BTOR2 solver — parser, Canonicalize/Solve engine, theory blasting, BMC loop
- See `.claude/commands/bitr-expert.md` for complete algorithmic reference

## Current Status

**Phase**: 8 — BMC Loop (complete)
**Last updated**: 2026-03-21

### Phase 0 (complete)
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
- [x] `cargo test` all passing (40 tests)
- [x] `cargo clippy` clean

### Phase 1 (complete)
- [x] ValueSet: xor, insert, remove, min/max_element, from_fn, iter (11 tests)
- [x] TermTable: SubstAndFold with constant folding (4 tests)
- [x] Recursive term evaluator — all OpKind variants (12 tests)
- [x] Compiled term evaluator (`bvdd/src/eval.rs`) — register machine (10 tests)
- [x] Cross-validation: compiled vs recursive agree on all inputs

### Phase 2 (complete)
- [x] Constraint hash consing via unique table (12 tests)
- [x] Restrict operation with short-circuit AND/OR
- [x] Predicate simplification (empty→FALSE, full→TRUE)
- [x] AND/OR normalization for better hash consing
- [x] collect_preds, has_no_predicates helpers
- [x] BVC Apply: structural (PRED constraints) and lifted (fresh vars) (6 tests)
- [x] BTOR2→BVC lifter (`bitr/src/lifter.rs`) — all operators + constants (5 tests)
- [x] 61 total tests, 0 clippy warnings

### Phase 3 (complete)
- [x] BVDD unique table hash consing (terminals + decision nodes)
- [x] FALSE terminal (BvddId(0)) with FullyCanonical canonicity
- [x] Edge merging: same-child edges get OR'd value sets
- [x] Empty-edge filtering and single-FULL-edge collapse
- [x] Bottom-up flag computation (can_be_true, is_ground)
- [x] All-FALSE-children collapse to FALSE terminal
- [x] Direct-mapped computed cache (64K entries) for Solve results
- [x] restrict_to_valueset with cache integration
- [x] 74 total tests, 0 clippy warnings

### Phase 4-5 (complete)
- [x] Solve algorithm with ground check, terminal→Canonicalize, decision traversal
- [x] Canonicalize with 6 phases: ground, SAT witness, UNSAT prune, no-preds, Decide, Restrict
- [x] Decide: predicate selection + coarsest partition
- [x] Generalized blast: narrowest-variable-first enumeration (budget 2^20)
- [x] Theory resolution for predicate-free constraints
- [x] BTOR2→BVC lifter with all operators, slice/ext special handling
- [x] CLI integration: parse → lift → solve → output sat/unsat
- [x] **9/9 tiny benchmarks correct** (6 SAT, 3 UNSAT)
- [x] 82 total tests, 0 clippy warnings

### Phase 8 (complete)
- [x] BMC loop: init/next substitution, step-by-step unrolling
- [x] Automatic sequential/combinational detection
- [x] State variable substitution with SubstAndFold
- [x] Fixed Solve ground check to verify value against target set
- [x] CLI: --bound N option for BMC depth
- [x] **11/11 benchmarks correct** (9 combinational + 2 sequential: counter_sat, counter_unsat)
- [x] 82 total tests, 0 clippy warnings

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
