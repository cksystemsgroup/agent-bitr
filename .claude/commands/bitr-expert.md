# BITR: Bitvector Theory Resolution via Theory Decision Diagrams

Reference skill encoding the BITR paper's algorithms for agent-consumable use.

---

## 1. Conceptual Foundation

BITR solves bitvector satisfiability by treating **canonicalization as solving**.

- **TCs (Theory-Constrained terms)**: symbolic bitvector expressions paired with
  constraints on which domain values they may take.
- **TDDs (Theory Decision Diagrams)**: BDD-like DAGs where nodes are labeled by
  bitvector variables and edges carry **value sets** (subsets of the domain).
  A path from root to terminal encodes a partial assignment.
- **BVDDs (Bitvector Decision Diagrams)**: the concrete TDD instantiation for
  8-bit bitvector slices. A BVDD encodes the complete solver state: the decision
  diagram IS the formula, the assignment trail, AND the learned clauses combined.
- **Canonicalization = solving**: reducing a BVDD to canonical form decides
  satisfiability. A fully-canonical BVDD with a non-FALSE terminal witnesses SAT;
  a BVDD that reduces to the FALSE terminal witnesses UNSAT.

---

## 2. Data Structures

### Value Sets (256-bit bitmasks)

One bit per domain value for 8-bit bitvectors (values 0..255).

```
Operations:
  AND(S1, S2)  -> intersection (propagation)
  OR(S1, S2)   -> union (resolution / edge merging)
  NOT(S)       -> complement
  popcount(S)  -> cardinality
  singleton(S) -> true iff popcount(S) == 1
  is_empty(S)  -> true iff popcount(S) == 0
  is_full(S)   -> true iff S == 0xFFFF...FF (all 256 bits set)
```

Stored as `[u64; 4]`. All ops are branchless bitwise.

### Terms (hash-consed DAG)

Arena-allocated in `Vec<Term>` with `TermId(u32)` indices.

```
Term ::= TERM_CONST(d)           -- concrete domain value (0..255)
       | TERM_VAR(x)             -- symbolic variable
       | TERM_APP(op, args)      -- function application
```

Hash-consed: structurally identical terms share the same `TermId`.

### Constraints (Boolean formulas over predicates)

```
Constraint ::= TRUE
             | FALSE
             | PRED(phi, S)       -- "BVC phi has value in set S"
             | NOT(K)
             | AND(K1, K2)
             | OR(K1, K2)
```

`PRED(phi, S)` asserts that the bitvector expression `phi` evaluates to a value
within the value set `S`.

### BVCs (Bitvector Constraints)

A set of (term, constraint) entries: `{(t1, K1), ..., (tn, Kn)}`.

Each entry says "the symbolic term `ti` is feasible under constraint `Ki`."
Represents a constrained symbolic value -- the term tells you what expression,
the constraint tells you what is known about it.

### BVDDs (Bitvector Decision Diagrams)

DAG with two node kinds:

- **Terminal nodes**: hold a BVC (the constrained value at this leaf).
- **Decision nodes**: labeled with a BVDD variable; outgoing edges carry
  value-set labels. Each edge points to a child BVDD.

Hash-consed via unique table. Edges with identical children are merged
(value sets OR'd together).

### HSC (Hierarchical Slice Cascade)

Extends BVDDs beyond 8-bit to arbitrary width by cascading 8-bit slices
from MSB to LSB. Each level of the cascade is an 8-bit BVDD whose terminals
contain the next-lower-byte BVDDs. A 32-bit variable uses 4 cascaded levels.

---

## 3. Core Algorithms

### Solve(G, S)

Traverse BVDD `G` restricted to value set `S`.

```
Solve(G, S):
  Phase 1 -- O(1) ground check:
    if G.is_ground: return G  (already fully resolved)

  Phase 2 -- terminal:
    if G.is_terminal:
      return Canonicalize(G.bvc, S)

  Phase 3 -- decision node:
    for each edge (S_i, child_i) in G.edges:
      S_new = AND(S_i, S)
      if is_empty(S_new): continue          -- UNSAT pruning
      result_i = Solve(child_i, S_new)
      if result_i.is_sat: return result_i   -- early SAT termination
    combine results, merge edges, return new BVDD
```

### Canonicalize(v, S)

Canonicalize a BVC `v` under value set `S`.

```
Canonicalize(v, S):
  Phase 1 -- ground check:
    if v.is_ground: return v

  Phase 2 -- SAT witness scan:
    if v.has_sat_witness(S): return witness

  Phase 3 -- UNSAT pruning:
    if v.is_unsat(S): return FALSE

  Phase 4 -- no predicates remain:
    if v.has_no_predicates:
      return TheoryResolve(v, S)

  Phase 5a -- Decide:
    (phi, partition) = Decide(v)
    -- select highest-priority predicate phi
    -- compute coarsest partition of domain by membership signature

  Phase 5b -- Restrict + recurse:
    for each S_j in partition:
      v_j = Restrict(v, phi, S_j)
      result_j = Solve(MakeBVDD(v_j), S_j)
    combine results into new BVDD
```

### Decide

```
Decide(v):
  preds = collect all PRED(phi, S) nodes from constraints in v
  phi = select_highest_priority(preds)
  -- priority: fewest distinct value sets > narrowest term > leftmost

  partition = coarsest_partition(domain, phi)
  -- group domain values d by their membership signature:
  --   sig(d) = { S : PRED(phi, S) in preds and d in S }
  -- values with identical signatures go in the same partition element

  return (phi, partition)
```

### Restrict(K, phi, S_j)

Syntactic substitution -- specialize constraint `K` given that predicate
`phi` takes values in `S_j`.

```
Restrict(K, phi, S_j):
  match K:
    TRUE | FALSE       -> K
    PRED(phi', S'):
      if phi' != phi   -> K  (unrelated predicate, unchanged)
      if S_j subset S' -> TRUE
      if S_j inter S' == empty -> FALSE
      else             -> PRED(phi, S_j inter S')
    NOT(K1)            -> NOT(Restrict(K1, phi, S_j))
    AND(K1, K2)        -> short-circuit AND of Restrict(K1), Restrict(K2)
    OR(K1, K2)         -> short-circuit OR of Restrict(K1), Restrict(K2)
```

Short-circuit: `AND(FALSE, _) = FALSE`, `OR(TRUE, _) = TRUE`, etc.

---

## 4. Theory Resolution Cascade

When no predicates remain in a BVC (all constraints are TRUE/FALSE, but terms
are still symbolic), the theory resolution cascade resolves the remaining
symbolic structure.

### Stage 1: Boolean Decomposition (1-bit terms only)

Find comparison subterms (EQ, ULT, etc.) within 1-bit expressions.
Branch on true/false for each comparison, creating two sub-problems.

### Stage 2: Generalized Blast

Eliminate the **narrowest** variable first. Enumerate its full domain,
call `SubstAndFold` for each value, recurse.

```
GeneralizedBlast(v, S):
  x = narrowest_variable(v)
  for each d in domain(x):
    v_d = SubstAndFold(v, x, d)
    result_d = Solve(v_d, S)
    if result_d.is_sat: return result_d
  return UNSAT
```

Budget: 2^20 total domain product across all variables at this stage.
With compiled evaluator, budget extends to 2^28.

### Stage 3: Byte-Blast Oracle

Split the **widest** comparison-relevant variable's MSB byte. Blast that
byte (enumerate 256 values), recurse on the remainder. If recursion stalls,
invoke the theory oracle on the residual.

```
Limits:
  max_depth = 4     (at most 4 bytes blasted)
  oracle_budget = 5s per oracle call
  wall_budget = 10s total
  adaptive_bailout = 25% (bail if >25% of branches need oracle)
```

### Stage 4: Direct Theory Oracle

Call an external solver (e.g., bitwuzla) on the residual formula.
Last resort. Results are cached in the oracle cache.

---

## 5. Three Blasting Strategies

### Interval-Blasting (Phases 5a/5b of Canonicalize)

The Decide/Restrict loop partitions the domain by predicate boundaries.
Each partition element groups domain values that satisfy exactly the same
set of predicates. This is **interval blasting** -- the partition elements
are often contiguous intervals when predicates are comparisons.

### Byte-Blasting (Generalized Blast, Stage 2)

Variable-at-a-time elimination, narrowest variable first. For variables
wider than 8 bits, split into MSB byte + remainder via CONCAT, then
blast the MSB byte first. This decomposes a wide enumeration into a
cascade of 256-way branches.

### Bit-Blasting (Stage 1 / terminal case)

When only 1-bit variables remain, direct enumeration is bit-blasting.
Each variable has domain {0, 1}, so enumeration is a binary branch.

---

## 6. Canonicity Levels

```
MODULO_BVC:
  Structural invariants only.
  - All terms and BVDD nodes are hash-consed.
  - Edges to the same child are merged (value sets OR'd).
  - No semantic simplification has been performed.

MODULO_BITVECTOR:
  All constraints are TOP (no predicates remain).
  Terms are still symbolic -- theory resolution has not yet
  eliminated all variables. The BVDD structure is canonical
  with respect to the predicate-free constraint language.

FULLY_CANONICAL:
  All terminals are value terminals: {(d, TOP)} for concrete d.
  Every label BVDD has exactly one variable.
  The diagram is a canonical representation of the solution set.
  SAT iff any non-FALSE terminal is reachable.
```

---

## 7. BTOR2 Format and Operators

### Sort System

```
sort bitvec W          -- bitvector of width W
sort array W_idx W_val -- array indexed by W_idx-bit keys, W_val-bit values
```

### Node Format

```
<nid> <op> <sort> <args...>
```

`nid` is a positive integer. Negative `-nid` means the NOT/negation of node `nid`.

### Structural Operators

These produce predicate/constraint structure consumed by Decide.
They create PRED nodes in the constraint language.

```
Width-1 results (Boolean):
  EQ, NEQ, ULT, SLT, ULTE, SLTE    -- comparisons
  UADDO, UMULO                       -- overflow detection
  REDAND, REDOR, REDXOR              -- reduction ops
  AND, OR, XOR, NOT                  -- Boolean (at width 1)
  ITE                                 -- if-then-else (width 1 condition)
```

### Non-Structural Operators

Lifted to fresh variables in lazy mode (defer cross-product until needed).

```
Arithmetic:  ADD, SUB, MUL, UDIV, UREM, SDIV, SREM, SMOD
Shift:       SLL, SRL, SRA
Bitwise:     NOT, NEG (wide, i.e., width > 1)
Slice/Ext:   SLICE, UEXT, SEXT, CONCAT
```

### Special Nodes

```
input <sort>                -- primary input (unconstrained)
state <sort>                -- state variable (for sequential)
init <sort> <state> <val>   -- initial value of state
next <sort> <state> <expr>  -- next-state function
bad <expr>                  -- property to check (bad iff expr=1)
constraint <expr>           -- assumption (expr=1 always)
output <expr>               -- observable output
```

---

## 8. Array Support

Arrays are handled by ROW (Read-Over-Write) expansion:

- **WRITE** nodes: expanded to READ-over-WRITE chains.
  `WRITE(a, addr, val)` is not stored as a monolithic array; instead,
  reads from the written array become ITE chains.
- **READ** nodes: `READ(WRITE(a, w_addr, w_val), r_addr)` expands to
  `ITE(EQ(r_addr, w_addr), w_val, READ(a, r_addr))`.
  Nested writes produce nested ITE chains checking each write address.

This eliminates the array theory, reducing everything to bitvector operations.

---

## 9. BMC Integration

Bounded model checking via unrolling:

```
BMC(model, max_k):
  for k = 0, 1, 2, ..., max_k:
    -- create fresh state variables for step k
    -- step 0: apply init constraints
    -- step k>0: substitute next-state functions from step k-1
    -- check each bad property:
    --   Solve(bad_k) == SAT  -->  counterexample of length k
    --   Solve(bad_k) == UNSAT --> property holds at depth k
  fixpoint: all bad properties UNSAT at depth k --> SAFE
```

Each unrolling step substitutes the `next` expressions to create a
combinational formula for step `k`, then calls `Solve`.

---

## 10. Performance Guidelines

### Hash Consing

All terms and BVDD nodes are arena-allocated and hash-consed.
`Vec<Term>` arena with `TermId(u32)` indices. Unique table maps
structural content to existing IDs. O(1) equality checks via ID comparison.

### Maximal Edge Merging

When multiple edges from a decision node point to the same child BVDD,
merge them into a single edge whose value set is the OR of the originals.
Reduces branching factor.

### Compiled Evaluator

Inner-loop term evaluation compiles the term DAG into a flat instruction
sequence (register-machine style). Approximately 10x faster than recursive
`eval()`. Used in generalized blast when enumerating large domains.

### Domain Budget Checks

Before enumerating, compute the product of domain sizes:
- Standard blast budget: 2^20 total domain product
- Compiled-eval blast budget: 2^28 total domain product
Abort and fall through to next cascade stage if budget exceeded.

### Lifting Non-Structural Ops

Non-structural operators (ADD, MUL, etc.) introduce fresh variables
with equality constraints rather than eagerly computing cross-products.
Defers explosion until Decide/Restrict actually needs the values.

### O(1) Flags on BVDD Nodes

Each BVDD node caches:
- `is_ground`: all terminals are concrete values
- `can_be_true`: at least one non-FALSE terminal is reachable
Computed bottom-up at construction time. Enables Phase 1 short-circuits.

### Memoization

- **Oracle cache**: maps residual formulas to oracle results
- **Decomposition memo**: caches Boolean decomposition results
- **Computed table**: caches Solve(G, S) results for (node_id, valueset) pairs

---

## 11. Correspondence: Bitmask Ops to Solving Steps

```
Bitmask Operation       | Solving Interpretation
------------------------|----------------------------------------------
AND(S1, S2)             | Propagation -- intersect feasible value sets
OR(S1, S2)              | Resolution -- merge edges to same child
NOT(S)                  | Complement -- negate a constraint
popcount(S) == 0        | Conflict -- UNSAT (empty domain)
popcount(S) == 1        | Unit -- forced assignment (single value)
S == full (all bits)    | Unconstrained -- tautology (any value works)
S1 subset S2            | Entailment -- S1 implies S2 (Restrict -> TRUE)
S1 inter S2 == empty    | Contradiction -- mutual exclusion (Restrict -> FALSE)
```

The key insight: every DPLL(T) operation (propagation, conflict, learning,
backtracking) has a direct counterpart as a bitmask operation on value sets
within the BVDD. The diagram structure makes these operations implicit and
cache-friendly rather than requiring explicit clause databases.
