"""Compare the selected before/after matrix without merging smoke records."""

import argparse
import json
from pathlib import Path


def read_groups(path):
    document = json.loads(path.read_text(encoding="utf-8-sig"))
    groups = {}
    for group in document["groups"]:
        log = document["logs"][group["log"]]
        metadata = log["metadata"]
        if log["test_status"] != "PASS":
            raise ValueError(f"Incomplete benchmark log: {group['log']}")
        key = (metadata.get("mode", "service"), group["files"], group["operation"])
        if key in groups:
            raise ValueError(f"Multiple logs for one comparison key: {key}")
        groups[key] = (group, metadata)
    return groups


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--before", required=True, type=Path)
    parser.add_argument("--after", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    before = read_groups(args.before)
    after = read_groups(args.after)
    rows = []
    for key in sorted(before.keys() & after.keys()):
        old, old_metadata = before[key]
        new, new_metadata = after[key]
        if old["fixture"] != new["fixture"] or old_metadata["profile"] != new_metadata["profile"]:
            raise ValueError(f"Fixture or profile mismatch: {key}")
        if old["samples"] != new["samples"]:
            raise ValueError(f"Sample count mismatch: {key}")
        delta = (new["median_ms"] / old["median_ms"] - 1) * 100
        budget = None
        if old["samples"] == 7:
            if key == ("stage", 13016, "component_durable_stage"):
                budget = "PASS" if delta <= -10 else "NOT_MET"
            elif key[0] == "service" and key[1] in (32, 128) and not key[2].startswith("component_") and key[2] != "snapshot_digest":
                budget = "PASS" if delta <= 10 else "REGRESSION_REQUIRES_REVIEW"
        rows.append({
            "mode": key[0], "files": key[1], "operation": key[2],
            "samples": old["samples"], "before_log": old["log"], "after_log": new["log"],
            "before_median_ms": old["median_ms"], "after_median_ms": new["median_ms"],
            "median_delta_percent": delta,
            "before_range_ms": [old["min_ms"], old["max_ms"]],
            "after_range_ms": [new["min_ms"], new["max_ms"]],
            "sample_ranges_overlap": max(old["min_ms"], new["min_ms"]) <= min(old["max_ms"], new["max_ms"]),
            "numeric_budget": budget,
            "evidence": "seven_samples" if old["samples"] == 7 else "single_boundary_sample",
        })
    result = {
        "before_summary": str(args.before), "after_summary": str(args.after),
        "before_only": [list(key) for key in sorted(before.keys() - after.keys())],
        "after_only": [list(key) for key in sorted(after.keys() - before.keys())],
        "rows": rows,
        "limits": "Numeric budgets do not establish statistical significance or native WebView performance. Check frozen build/harness identity receipts separately.",
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"Saved {len(rows)} comparisons to {args.output}")


if __name__ == "__main__":
    main()
