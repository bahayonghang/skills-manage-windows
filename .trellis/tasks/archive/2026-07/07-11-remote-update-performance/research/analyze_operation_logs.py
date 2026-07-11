from __future__ import annotations

import json
import sqlite3
import statistics
from collections import defaultdict
from pathlib import Path


DB_PATH = Path.home() / ".skillsmanage" / "db.sqlite"


def percentile(values: list[float], ratio: float) -> float:
    ordered = sorted(values)
    index = int((len(ordered) - 1) * ratio)
    return ordered[index]


def linear_fit(samples: list[tuple[float, float]]) -> tuple[float, float] | None:
    if len(samples) < 2:
        return None
    xs = [sample[0] for sample in samples]
    ys = [sample[1] for sample in samples]
    x_mean = statistics.fmean(xs)
    y_mean = statistics.fmean(ys)
    denominator = sum((value - x_mean) ** 2 for value in xs)
    if denominator == 0:
        return None
    slope = sum(
        (x_value - x_mean) * (y_value - y_mean)
        for x_value, y_value in samples
    ) / denominator
    return y_mean - slope * x_mean, slope


def main() -> None:
    connection = sqlite3.connect(DB_PATH)
    rows = connection.execute(
        """
        SELECT action, target_kind, duration_ms, details_json
        FROM operation_logs
        WHERE action IN (?, ?) AND status = ? AND duration_ms IS NOT NULL
        """,
        ("central.delete_repository", "skill.batch_uninstall", "succeeded"),
    ).fetchall()

    samples_by_action_target: dict[tuple[str, str], list[tuple[float, float]]] = (
        defaultdict(list)
    )
    for action, target_kind, duration_ms, details_json in rows:
        details = json.loads(details_json or "{}")
        if action == "central.delete_repository":
            unit_count = float(details.get("requestCount", 0))
        else:
            unit_count = float(len(details.get("succeeded", [])) + len(details.get("failed", [])))
        if unit_count > 0:
            samples_by_action_target[(action, target_kind)].append(
                (unit_count, float(duration_ms))
            )

    result: dict[str, dict[str, object]] = defaultdict(dict)
    for (action, target_kind), samples in sorted(samples_by_action_target.items()):
        durations = [duration for _, duration in samples]
        per_unit = [duration / count for count, duration in samples]
        fit = linear_fit(samples)
        result[action][target_kind] = {
            "samples": len(samples),
            "unitCountRange": [
                int(min(count for count, _ in samples)),
                int(max(count for count, _ in samples)),
            ],
            "durationP50Ms": round(percentile(durations, 0.50), 2),
            "durationP95Ms": round(percentile(durations, 0.95), 2),
            "perUnitP50Ms": round(percentile(per_unit, 0.50), 2),
            "linearInterceptMs": round(fit[0], 2) if fit else None,
            "linearSlopeMsPerUnit": round(fit[1], 2) if fit else None,
        }

    print(json.dumps(result, indent=2, ensure_ascii=True))


if __name__ == "__main__":
    main()
