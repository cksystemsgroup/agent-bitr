# CLAUDE.md

## Build & Test

```bash
cargo build              # debug build (both crates)
cargo build --release    # optimized build
cargo test               # all tests
cargo clippy             # lint — must pass with no warnings
```

## Development Workflow

1. Read `program.md` for current status and next task
2. Read `.claude/commands/bitr-expert.md` for algorithmic reference
3. Implement — write code + tests
4. `cargo test` — never break existing tests
5. `cargo clippy` — fix all warnings
6. Benchmark if relevant: `cargo run --release -- --stats benchmarks/tiny/*.btor2`
7. Update `expert.md` with any discoveries or insights
8. Update `program.md` status section
9. Commit with descriptive message

## Critical Rules

- **Soundness over speed**: never return SAT when answer is UNSAT, or vice versa
- **Never break tests**: all existing tests must pass before committing
- **Measure before/after**: when optimizing, record benchmark numbers
- **Append to experiments.log**: every benchmark run gets a dated entry
- **No unsafe Rust** unless absolutely necessary and documented
- **Arena pattern**: BVDDs use `Vec<Node>` + `NodeId(u32)` indices, not Rc/RefCell
