"""Summarize BENCH records with Python's standard library. Preserve raw logs."""

import argparse
import hashlib
import json
import statistics
from collections import defaultdict
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("logs", nargs="+", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    groups = defaultdict(list)
    log_metadata = {}
    for path in args.logs:
        raw = path.read_bytes()
        lines = raw.decode("utf-8-sig").splitlines()
        metadata = next((json.loads(line.split("BENCH_META ", 1)[1]) for line in lines if "BENCH_META " in line), {})
        identity_path = path.with_suffix(".identity.json")
        log_metadata[str(path)] = {
            "sha256": hashlib.sha256(raw).hexdigest(),
            "metadata": metadata,
            "test_status": "PASS" if any("test result: ok." in line for line in lines) else "INCOMPLETE_OR_FAILED",
            "run_identity": json.loads(identity_path.read_text(encoding="utf-8-sig")) if identity_path.exists() else None,
        }
        for line in lines:
            if line.startswith("BENCH "):
                row = json.loads(line[6:])
                groups[(str(path), row["fixture"]["files"], row["operation"])].append(row)
    summary = []
    for (log, files, operation), rows in sorted(groups.items()):
        elapsed = [row["elapsed_ms"] for row in rows]
        phases = defaultdict(list)
        for row in rows:
            for name, phase in row["phases"].items():
                phases[name].append(phase["await_ms"])
        summary.append({
            "log": log, "files": files, "operation": operation,
            "observations": len(rows), "samples": len({row["sample"] for row in rows}),
            "fixture": rows[0]["fixture"], "raw_ms": elapsed,
            "raw_rows": rows,
            "median_ms": statistics.median(elapsed),
            "min_ms": min(elapsed), "max_ms": max(elapsed),
            "phase_median_ms": {name: statistics.median(values) for name, values in sorted(phases.items())},
            "phase_note": "inclusive wrapper await wall time; nested spans overlap",
            "sample_0_ms": [row["elapsed_ms"] for row in rows if row["sample"] == 0],
            "samples_1_to_6_ms": [row["elapsed_ms"] for row in rows if 1 <= row["sample"] <= 6],
        })
    args.output.write_text(json.dumps({"logs": log_metadata, "groups": summary}, indent=2) + "\n", encoding="utf-8")
    print(f"Saved {len(summary)} groups to {args.output}")


if __name__ == "__main__":
    main()
