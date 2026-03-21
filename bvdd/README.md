# bvdd — Bitvector Decision Diagrams

A standalone library for constructing and manipulating Bitvector Decision Diagrams (BVDDs).

## Overview

BVDDs are canonical decision diagrams where:
- **Edges** carry 256-bit bitmask labels (value sets)
- **Terminal nodes** hold bitvector constraints (BVCs)
- **Decision nodes** partition the domain by predicate boundaries
- **Hash consing** ensures structural sharing

## Usage (Rust)

```rust
use bvdd::valueset::ValueSet;
use bvdd::term::TermTable;

// Create a value set for {0, 1, 2, 3}
let vs = ValueSet::full_for_width(2);
assert_eq!(vs.popcount(), 4);

// Intersection = propagation
let a = ValueSet::from_range(0, 5);
let b = ValueSet::from_range(3, 10);
let propagated = a.and(b); // {3, 4, 5}

// Hash-consed terms
let mut terms = TermTable::new();
let x = terms.make_var(0, 8);
let y = terms.make_var(1, 8);
```

## License

MIT
