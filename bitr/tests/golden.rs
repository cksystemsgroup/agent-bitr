//! Golden integration tests: end-to-end pipeline on tiny + curated benchmarks.
//!
//! Phase A4 of the correctness plan. These tests lock in the current correct
//! answers and serve as a non-regression guardrail. Any change that breaks a
//! golden test must be explained before landing.
//!
//! The test binary spawns the compiled `bitr` executable and asserts the output
//! matches the expected answer from the filename convention (`*_sat.btor2` →
//! "sat", `*_unsat.btor2` → "unsat").

use std::path::{Path, PathBuf};
use std::process::Command;

fn bitr_bin() -> PathBuf {
    // Cargo injects CARGO_BIN_EXE_<bin> at test build time.
    PathBuf::from(env!("CARGO_BIN_EXE_bitr"))
}

fn repo_root() -> PathBuf {
    // bitr/tests/golden.rs -> bitr/tests -> bitr -> <repo_root>
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("repo root").to_path_buf()
}

fn run_bitr(args: &[&str]) -> (String, String, bool) {
    let output = Command::new(bitr_bin())
        .args(args)
        .output()
        .expect("failed to spawn bitr");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

fn last_line(s: &str) -> &str {
    s.lines().last().unwrap_or("").trim()
}

/// Run bitr on a benchmark path and assert the last stdout line matches expected.
fn expect_answer(path: &Path, expected: &str) {
    assert!(path.exists(), "benchmark missing: {}", path.display());
    let path_str = path.to_str().expect("utf-8 path");
    let (stdout, stderr, _) = run_bitr(&[path_str]);
    let got = last_line(&stdout);
    assert_eq!(
        got, expected,
        "benchmark {}: expected `{}`, got `{}`\nstderr:\n{}",
        path.display(), expected, got, stderr,
    );
}

#[test]
fn tiny_sat_benchmarks_return_sat() {
    let root = repo_root();
    let tiny = root.join("benchmarks/tiny");
    // Each filename ending in _sat.btor2 must return "sat".
    let sat_cases = [
        "arith_sat.btor2",
        "array_row_sat.btor2",
        "concat_slice_sat.btor2",
        "counter_sat.btor2",
        "ite_sat.btor2",
        "multi_var_sat.btor2",
        "shift_reg_sat.btor2",
        "signed_sat.btor2",
        "simple_sat.btor2",
        "wide_sat.btor2",
    ];
    for case in &sat_cases {
        expect_answer(&tiny.join(case), "sat");
    }
}

#[test]
fn tiny_unsat_benchmarks_return_unsat() {
    let root = repo_root();
    let tiny = root.join("benchmarks/tiny");
    // Each filename ending in _unsat.btor2 must return "unsat".
    let unsat_cases = [
        "array_row_unsat.btor2",
        "bool_unsat.btor2",
        "counter_unsat.btor2",
        "ite_unsat.btor2",
        "multi_var_unsat.btor2",
        "simple_unsat.btor2",
    ];
    for case in &unsat_cases {
        expect_answer(&tiny.join(case), "unsat");
    }
}

/// With --verify, every SAT/UNSAT answer is cross-checked against the external
/// oracle. This test runs tiny benchmarks in verify mode and asserts none
/// trigger a verify-mismatch panic.
///
/// Skipped if no external SMT solver is installed on the test host.
#[test]
fn tiny_benchmarks_pass_verify_mode() {
    // Skip if no oracle available — verify mode is a no-op without one.
    let have_oracle = ["bitwuzla", "boolector", "z3"]
        .iter()
        .any(|name| which(name).is_some());
    if !have_oracle {
        eprintln!("skipping: no oracle solver on PATH");
        return;
    }

    let root = repo_root();
    let tiny = root.join("benchmarks/tiny");
    let all_cases: Vec<&str> = vec![
        "arith_sat.btor2", "array_row_sat.btor2", "array_row_unsat.btor2",
        "bool_unsat.btor2", "concat_slice_sat.btor2", "counter_sat.btor2",
        "counter_unsat.btor2", "ite_sat.btor2", "ite_unsat.btor2",
        "multi_var_sat.btor2", "multi_var_unsat.btor2", "shift_reg_sat.btor2",
        "signed_sat.btor2", "simple_sat.btor2", "simple_unsat.btor2",
        "wide_sat.btor2",
    ];

    for case in &all_cases {
        let path = tiny.join(case);
        let path_str = path.to_str().unwrap();
        let (stdout, stderr, success) = run_bitr(&["--verify", path_str]);
        // A panic from verify-mismatch will show in stderr and make the process exit abnormally.
        assert!(
            success || last_line(&stdout) == "unknown",
            "{}: bitr failed in --verify mode\nstdout:{}\nstderr:{}",
            case, stdout, stderr,
        );
        assert!(
            !stderr.contains("VERIFY MISMATCH"),
            "{}: oracle disagreement in --verify mode\nstderr:\n{}",
            case, stderr,
        );
    }
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let full = dir.join(bin);
        if full.exists() {
            return Some(full);
        }
    }
    None
}

// NOTE: The HWMCC BV golden subset was intentionally removed after Phase A5.
// Several SAT verdicts in the pre-A5 baseline (e.g., 93.c) depended on the
// unsound reset-signal heuristic that zeroed clock/reset inputs. With the A5
// fence they no longer qualify as resets, BMC has to unroll more steps, and
// the existing --timeout does not always bail out cleanly — a latent issue
// tracked separately. Once Phase B incremental BMC lands and timeout honoring
// is fixed, the HWMCC subset can be reintroduced with confirmed-correct
// expected results.
