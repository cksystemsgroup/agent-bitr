#!/usr/bin/env python3
"""Compare bitr results against a reference solver."""

import csv
import sys
from pathlib import Path


def load_results(path):
    results = {}
    with open(path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            results[row["name"]] = {
                "status": row["status"],
                "time_s": float(row["time_s"]),
            }
    return results


def compare(bitr_path, ref_path):
    bitr = load_results(bitr_path)
    ref = load_results(ref_path)

    common = set(bitr.keys()) & set(ref.keys())

    bitr_solved = sum(1 for n in common if bitr[n]["status"] in ("sat", "unsat"))
    ref_solved = sum(1 for n in common if ref[n]["status"] in ("sat", "unsat"))

    # Wrong answers: bitr says sat but ref says unsat, or vice versa
    wrong = []
    for n in common:
        bs, rs = bitr[n]["status"], ref[n]["status"]
        if bs in ("sat", "unsat") and rs in ("sat", "unsat") and bs != rs:
            wrong.append(n)

    bitr_only = sum(
        1
        for n in common
        if bitr[n]["status"] in ("sat", "unsat")
        and ref[n]["status"] not in ("sat", "unsat")
    )
    ref_only = sum(
        1
        for n in common
        if ref[n]["status"] in ("sat", "unsat")
        and bitr[n]["status"] not in ("sat", "unsat")
    )

    bitr_time = sum(bitr[n]["time_s"] for n in common if bitr[n]["status"] in ("sat", "unsat"))
    ref_time = sum(ref[n]["time_s"] for n in common if ref[n]["status"] in ("sat", "unsat"))

    print(f"Benchmarks compared: {len(common)}")
    print(f"bitr solved:         {bitr_solved}")
    print(f"reference solved:    {ref_solved}")
    print(f"bitr-only:           {bitr_only}")
    print(f"reference-only:      {ref_only}")
    print(f"WRONG ANSWERS:       {len(wrong)}")
    if wrong:
        for w in wrong[:10]:
            print(f"  {w}: bitr={bitr[w]['status']} ref={ref[w]['status']}")
    print(f"bitr total time:     {bitr_time:.1f}s")
    print(f"reference total time: {ref_time:.1f}s")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <bitr.csv> <reference.csv>")
        sys.exit(1)
    compare(sys.argv[1], sys.argv[2])
