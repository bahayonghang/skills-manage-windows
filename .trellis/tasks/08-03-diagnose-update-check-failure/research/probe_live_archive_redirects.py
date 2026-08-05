"""Probe live GitHub archive redirects without following codeload or using a PAT."""

from __future__ import annotations

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
import json
from pathlib import Path
import re
import sqlite3
from typing import Any
import urllib.error
import urllib.parse
import urllib.request


API_HOST = "api.github.com"
CODELOAD_HOST = "codeload.github.com"
OWNER_PATTERN = re.compile(r"^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?$")
REPO_PATTERN = re.compile(r"^[A-Za-z0-9_.-]+$")


@dataclass(frozen=True)
class RepositoryRef:
    owner: str
    repo: str
    branch: str


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):  # type: ignore[no-untyped-def]
        return None


def raw_segment_is_dot(segment: str) -> bool:
    remainder = segment.lower()
    dots = 0
    while remainder:
        if remainder.startswith("."):
            remainder = remainder[1:]
        elif remainder.startswith("%2e"):
            remainder = remainder[3:]
        else:
            return False
        dots += 1
    return dots in (1, 2)


def decoded_path_segments(parsed: urllib.parse.SplitResult) -> list[str]:
    raw_segments = parsed.path.strip("/").split("/")
    if any(raw_segment_is_dot(segment) for segment in raw_segments):
        raise ValueError("dot_segment")
    if any("%2f" in segment.lower() or "%5c" in segment.lower() for segment in raw_segments):
        raise ValueError("encoded_separator")
    return [urllib.parse.unquote(segment) for segment in raw_segments]


def validate_https_origin(
    parsed: urllib.parse.SplitResult,
    expected_host: str,
) -> None:
    if (
        parsed.scheme != "https"
        or parsed.hostname != expected_host
        or parsed.port not in (None, 443)
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
    ):
        raise ValueError("invalid_origin")


def is_full_commit_sha(value: str) -> bool:
    return len(value) == 40 and all(character in "0123456789abcdefABCDEF" for character in value)


def validate_api_canonical_location(location: str, branch: str) -> None:
    parsed = urllib.parse.urlsplit(location)
    validate_https_origin(parsed, API_HOST)
    segments = decoded_path_segments(parsed)
    if len(segments) != 4 or segments[0] != "repositories" or segments[2] != "tarball":
        raise ValueError("invalid_numeric_path")
    repository_id = segments[1]
    if not repository_id.isascii() or not repository_id.isdigit() or int(repository_id) <= 0:
        raise ValueError("invalid_repository_id")
    if segments[3] != branch:
        raise ValueError("changed_ref")


def validate_codeload_location(
    location: str,
    repository: RepositoryRef,
    *,
    allow_renamed_identity: bool,
) -> str:
    parsed = urllib.parse.urlsplit(location)
    validate_https_origin(parsed, CODELOAD_HOST)
    segments = decoded_path_segments(parsed)
    if is_full_commit_sha(repository.branch):
        valid_shape = len(segments) == 4 and segments[2:] == [
            "legacy.tar.gz",
            repository.branch,
        ]
    else:
        valid_shape = len(segments) == 6 and segments[2:] == [
            "legacy.tar.gz",
            "refs",
            "heads",
            repository.branch,
        ]
    if not valid_shape:
        raise ValueError("invalid_codeload_path")
    owner, repo = segments[:2]
    if not OWNER_PATTERN.fullmatch(owner) or not REPO_PATTERN.fullmatch(repo):
        raise ValueError("invalid_canonical_identity")
    if repo in (".", ".."):
        raise ValueError("invalid_canonical_identity")
    if not allow_renamed_identity and (
        owner.lower() != repository.owner.lower()
        or repo.lower() != repository.repo.lower()
    ):
        raise ValueError("changed_identity_without_numeric_proof")
    if owner == repository.owner and repo == repository.repo:
        return "exact_302"
    if owner.lower() == repository.owner.lower() and repo.lower() == repository.repo.lower():
        return "case_only_302"
    return "numeric_canonicalization"


def request_without_redirect(url: str) -> tuple[int, str | None]:
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "SkillPort-archive-redirect-probe",
        },
        method="GET",
    )
    opener = urllib.request.build_opener(NoRedirect())
    try:
        response = opener.open(request, timeout=20)
        status = response.status
        location = response.headers.get("Location")
        response.close()
        return status, location
    except urllib.error.HTTPError as error:
        status = error.code
        location = error.headers.get("Location")
        error.close()
        return status, location


def archive_api_url(repository: RepositoryRef) -> str:
    segments = [
        urllib.parse.quote(value, safe="")
        for value in (repository.owner, repository.repo, "tarball", repository.branch)
    ]
    return f"https://{API_HOST}/repos/" + "/".join(segments)


def probe_repository(repository: RepositoryRef) -> str:
    status, location = request_without_redirect(archive_api_url(repository))
    if location is None:
        raise ValueError("missing_location")
    if status == 302:
        return validate_codeload_location(
            location,
            repository,
            allow_renamed_identity=False,
        )
    if status != 301:
        raise ValueError(f"unexpected_initial_status_{status}")
    validate_api_canonical_location(location, repository.branch)
    canonical_status, codeload_location = request_without_redirect(location)
    if canonical_status != 302:
        raise ValueError(f"unexpected_numeric_status_{canonical_status}")
    if codeload_location is None:
        raise ValueError("missing_codeload_location")
    return validate_codeload_location(
        codeload_location,
        repository,
        allow_renamed_identity=True,
    )


def load_repositories(database: Path) -> list[RepositoryRef]:
    connection = sqlite3.connect(f"{database.resolve().as_uri()}?mode=ro", uri=True)
    try:
        connection.execute("PRAGMA query_only=ON")
        rows = connection.execute(
            "SELECT DISTINCT r.owner, r.repo, r.branch "
            "FROM skill_repositories r "
            "JOIN skill_repository_members m ON m.repository_id = r.id "
            "WHERE r.source_type = 'github' AND r.is_unknown = 0 "
            "ORDER BY r.id"
        ).fetchall()
    finally:
        connection.close()
    return [RepositoryRef(*row) for row in rows]


def build_report(repositories: list[RepositoryRef], workers: int) -> dict[str, Any]:
    outcomes: Counter[str] = Counter()
    failures: Counter[str] = Counter()
    with ThreadPoolExecutor(max_workers=workers) as pool:
        future_map = {
            pool.submit(probe_repository, repository): repository
            for repository in repositories
        }
        for future in as_completed(future_map):
            try:
                outcomes[future.result()] += 1
            except Exception as error:
                failures[str(error) or type(error).__name__] += 1
    return {
        "repositories": len(repositories),
        "outcomes": dict(sorted(outcomes.items())),
        "failureReasons": dict(sorted(failures.items())),
        "policyCompatible": not failures and sum(outcomes.values()) == len(repositories),
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Probe live GitHub archive redirect metadata without a PAT."
    )
    parser.add_argument("--database", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=6)
    args = parser.parse_args()
    repositories = load_repositories(args.database)
    report = build_report(repositories, max(1, min(args.workers, 12)))
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["policyCompatible"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
