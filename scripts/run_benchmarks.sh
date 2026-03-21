#!/bin/bash
# Run bitr on all benchmarks with timeout
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BITR="${ROOT_DIR}/target/release/bitr"
TIMEOUT=${TIMEOUT:-300}
RESULTS_DIR="${ROOT_DIR}/results"

mkdir -p "$RESULTS_DIR"

if [ ! -f "$BITR" ]; then
    echo "Building bitr (release)..."
    cargo build --release --manifest-path "${ROOT_DIR}/Cargo.toml"
fi

run_suite() {
    local suite_name="$1"
    local suite_dir="$2"
    local output="${RESULTS_DIR}/${suite_name}.csv"

    echo "name,status,time_s" > "$output"

    local count=0
    local solved=0

    for f in "$suite_dir"/*.btor2; do
        [ -f "$f" ] || continue
        local name=$(basename "$f" .btor2)
        count=$((count + 1))

        local start=$(date +%s.%N)
        local result
        result=$(timeout "${TIMEOUT}s" "$BITR" --stats "$f" 2>&1 | tail -1) || result="TIMEOUT"
        local end=$(date +%s.%N)
        local elapsed=$(echo "$end - $start" | bc)

        local status="unknown"
        case "$result" in
            *"sat"*) status="sat"; solved=$((solved + 1)) ;;
            *"unsat"*) status="unsat"; solved=$((solved + 1)) ;;
            *"TIMEOUT"*) status="timeout" ;;
        esac

        echo "${name},${status},${elapsed}" >> "$output"
        printf "\r[%d] %s: %s (%.1fs)" "$count" "$name" "$status" "$elapsed"
    done

    echo ""
    echo "${suite_name}: solved ${solved}/${count} (results in ${output})"
}

# Run on tiny benchmarks first
if [ -d "${ROOT_DIR}/benchmarks/tiny" ]; then
    run_suite "tiny" "${ROOT_DIR}/benchmarks/tiny"
fi

# Run on HWMCC'24 if downloaded
if [ -d "${ROOT_DIR}/benchmarks/bv" ]; then
    run_suite "bv" "${ROOT_DIR}/benchmarks/bv"
fi

if [ -d "${ROOT_DIR}/benchmarks/array" ]; then
    run_suite "array" "${ROOT_DIR}/benchmarks/array"
fi
