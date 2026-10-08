"""Validate frozen alternating runs and retain each pair and phase record."""

import argparse
import hashlib
import json
import statistics
from collections import defaultdict
from pathlib import Path


HARNESS_SHA = "203949B25783EA044256268BF4D48CBD6967386A774D78A8515477A64970B923"
BINARY_SHA = {
    "before": "46A19EC66B300DF79D7345724DB5CA2440160AC29A23202655965A41088AD36F",
    "after": "41293FD91D399D680488B43AB1CCE547204776864FDDE309C32F1C07D76A3411",
}
FIXTURE = {"files": 13016, "directories": 206, "bytes": 13327429}
OPERATIONS = {
    "component_durable_stage", "component_delete_manifest_fingerprint",
    "component_delete_stage", "component_delete_finalize", "component_delete_finalize_repeat",
}


def validate_run(run):
    version = run["version"]
    if run["exit_code"] != 0 or run["harness_sha256"] != HARNESS_SHA:
        raise ValueError("Run failed or harness identity changed")
    if run["binary_sha256"] != BINARY_SHA[version]:
        raise ValueError("Frozen binary identity changed")
    if (run["profile"], run["mode"], run["files"], run["samples"]) != (
        "release", "stage", 13016, 1
    ):
        raise ValueError("Run configuration changed")
    log = Path(run["raw_log"])
    raw = log.read_bytes()
    if hashlib.sha256(raw).hexdigest().upper() != run["raw_log_sha256"]:
        raise ValueError(f"Raw log identity changed: {log}")
    sidecar = json.loads(log.with_suffix(".identity.json").read_text(encoding="utf-8-sig"))
    if sidecar != run:
        raise ValueError(f"Run sidecar changed: {log}")
    if Path(run["stderr_log"]).read_bytes():
        raise ValueError(f"Unexpected stderr: {log}")
    lines = raw.decode("utf-8-sig").splitlines()
    if not any("test result: ok. 1 passed; 0 failed" in line for line in lines):
        raise ValueError(f"Missing complete test PASS: {log}")
    metadata = next(json.loads(line.split("BENCH_META ", 1)[1]) for line in lines if "BENCH_META " in line)
    if (metadata["profile"], metadata["mode"], metadata["files"], metadata["samples"]) != (
        "release", "stage", [13016], 1
    ):
        raise ValueError(f"Runtime configuration changed: {log}")
    rows = [json.loads(line[6:]) for line in lines if line.startswith("BENCH ")]
    if len(rows) != 5 or {row["operation"] for row in rows} != OPERATIONS or any(row["fixture"] != FIXTURE or row["sample"] != 0 for row in rows):
        raise ValueError(f"Fixture or phase records changed: {log}")
    cpu_start, cpu_end = run["benchmark_cpu_start"], run["benchmark_cpu_end"]
    cpu_delta = None
    if cpu_start["status"] == cpu_end["status"] == "MEASURED":
        if cpu_start["pid"] != cpu_end["pid"] or cpu_end["cpu_seconds"] < cpu_start["cpu_seconds"]:
            raise ValueError(f"Invalid CPU endpoints: {log}")
        cpu_delta = cpu_end["cpu_seconds"] - cpu_start["cpu_seconds"]
    return {"identity": run, "metadata": metadata, "test_status": "PASS", "raw_rows": rows,
            "benchmark_cpu_delta_seconds": cpu_delta}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    runs = json.loads(args.runs.read_text(encoding="utf-8-sig"))
    if len(runs) != 14:
        raise ValueError("Expected seven complete pairs")
    verified = [validate_run(run) for run in runs]
    if any(first["ended_utc"] > second["started_utc"] for first, second in zip(runs, runs[1:])):
        raise ValueError("Alternating runs overlapped or chronological order changed")
    pairs = []
    operations = defaultdict(list)
    for pair in range(7):
        pair_runs = verified[pair * 2:pair * 2 + 2]
        expected = ["before", "after"] if pair % 2 == 0 else ["after", "before"]
        if [item["identity"]["version"] for item in pair_runs] != expected:
            raise ValueError(f"Pair order changed: {pair}")
        if any(item["identity"]["pair_index"] != pair or item["identity"]["pair_order"] != "-".join(expected) for item in pair_runs):
            raise ValueError(f"Pair identity changed: {pair}")
        if pair_runs[0]["identity"]["ended_utc"] > pair_runs[1]["identity"]["started_utc"]:
            raise ValueError(f"Pair runs overlapped: {pair}")
        versions = {item["identity"]["version"]: item for item in pair_runs}
        old_rows = {row["operation"]: row for row in versions["before"]["raw_rows"]}
        new_rows = {row["operation"]: row for row in versions["after"]["raw_rows"]}
        if old_rows.keys() != new_rows.keys():
            raise ValueError(f"Operations changed: {pair}")
        comparisons = []
        for name in sorted(old_rows):
            old, new = old_rows[name]["elapsed_ms"], new_rows[name]["elapsed_ms"]
            row = {"pair_index": pair, "operation": name, "before_ms": old, "after_ms": new,
                   "delta_ms": new - old, "delta_percent": (new / old - 1) * 100}
            operations[name].append(row)
            comparisons.append(row)
        pairs.append({"pair_index": pair, "order": "-".join(expected), "runs": pair_runs,
                      "comparisons": comparisons})
    summary = []
    for name, rows in sorted(operations.items()):
        old, new = [row["before_ms"] for row in rows], [row["after_ms"] for row in rows]
        old_median, new_median = statistics.median(old), statistics.median(new)
        delta = (new_median / old_median - 1) * 100
        summary.append({"operation": name, "pairs": 7, "raw_pairs": rows,
                        "before_median_ms": old_median, "after_median_ms": new_median,
                        "median_delta_percent": delta,
                        "median_paired_delta_ms": statistics.median(row["delta_ms"] for row in rows),
                        "median_paired_delta_percent": statistics.median(row["delta_percent"] for row in rows),
                        "before_range_ms": [min(old), max(old)], "after_range_ms": [min(new), max(new)],
                        "improved_pairs": sum(row["delta_ms"] < 0 for row in rows),
                        "pairs_meeting_ten_percent_reduction": sum(row["delta_percent"] <= -10 for row in rows),
                        "numeric_stage_budget": ("PASS" if delta <= -10 else "NOT_MET") if name == "component_durable_stage" else None})
    result = {"runs_receipt": str(args.runs), "runs_sha256": hashlib.sha256(args.runs.read_bytes()).hexdigest(),
              "verified_runs": 14, "fixture": FIXTURE, "pairs": pairs, "operations": summary,
              "limits": "Alternating seven pairs remain separate from the first formal seven and single controls. All samples are retained. Cumulative benchmark CPU includes fixture setup and validation; scanner CPU is UNMEASURED. No stable-gain or scanner-cause inference follows from overlapping ranges."}
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    for item in summary:
        print(f"{item['operation']}: {item['before_median_ms']:.4f} -> {item['after_median_ms']:.4f} ms; {item['median_delta_percent']:.2f}%; paired median {item['median_paired_delta_percent']:.2f}%; stage_budget={item['numeric_stage_budget']}")
    print(f"Validated 14 PASS logs and seven alternating pairs: {args.output}")


if __name__ == "__main__":
    main()
