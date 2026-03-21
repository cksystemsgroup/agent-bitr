# expert.md — Living Knowledge Base

This file captures non-obvious insights discovered during development.
Updated by agents as they learn from implementation and experiments.

## Key Insights from C Prototype

1. **Arena pattern is essential**: BVDDs use `Vec<Node>` + `NodeId(u32)` — no reference counting, no garbage collection. The C prototype uses arrays + integer indices throughout.

2. **Lifting defers explosion**: Non-structural operators (arithmetic, bitwise at width > 1) are lifted to fresh variables. This keeps BVCs small during parsing. Cross-product happens lazily during canonicalization.

3. **Compiled evaluator is 10x faster**: For inner-loop term evaluation (generalized blast), a flat instruction sequence is ~10x faster than recursive `term_eval()`.

4. **Comparison-guided variable ordering in BSO**: The byte-blast oracle prefers variables appearing in comparison predicates over the widest variable. Comparisons impose structural constraints that collapse many branches.

5. **Adaptive bailout at 25%**: In byte-blast oracle, if after 15 of 256 byte values fewer than 25% are resolved by blast alone, bail out early to avoid wasted work.

## Open Questions

- What is the optimal domain budget for generalized blast? C prototype uses 2^20.
- How does edge merging interact with HSC cascades for wide variables?
- Can SIMD accelerate ValueSet operations meaningfully on M-series Apple Silicon?
