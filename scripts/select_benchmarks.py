#!/usr/bin/env python3
"""Create benchmark subsets by difficulty (based on reference solver times)."""

import csv
import sys
from pathlib import Path


def select(ref_path, output_dir):
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    fast = []     # < 1s
    medium = []   # 1-60s
    hard = []     # 60-300s
    unsolved = [] # timeout

    with open(ref_path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            name = row["name"]
            status = row["status"]
            time_s = float(row["time_s"])

            if status in ("sat", "unsat"):
                if time_s < 1:
                    fast.append(name)
                elif time_s < 60:
                    medium.append(name)
                else:
                    hard.append(name)
            else:
                unsolved.append(name)

    for subset_name, subset in [("fast", fast), ("medium", medium), ("hard", hard), ("unsolved", unsolved)]:
        path = output_dir / f"{subset_name}.txt"
        with open(path, "w") as f:
            for name in sorted(subset):
                f.write(name + "\n")
        print(f"{subset_name}: {len(subset)} benchmarks")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <reference.csv> <output_dir>")
        sys.exit(1)
    select(sys.argv[1], sys.argv[2])
