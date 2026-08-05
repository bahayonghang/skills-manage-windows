import unittest

from probe_live_archive_redirects import (
    RepositoryRef,
    validate_api_canonical_location,
    validate_codeload_location,
)


class ProbeLiveArchiveRedirectTests(unittest.TestCase):
    def test_case_only_codeload_identity_is_equivalent(self) -> None:
        repository = RepositoryRef("owner", "repo", "main")
        outcome = validate_codeload_location(
            "https://codeload.github.com/Owner/Repo/legacy.tar.gz/refs/heads/main",
            repository,
            allow_renamed_identity=False,
        )
        self.assertEqual(outcome, "case_only_302")

    def test_numeric_location_requires_positive_id_and_same_ref(self) -> None:
        validate_api_canonical_location(
            "https://api.github.com/repositories/123/tarball/main",
            "main",
        )
        for rejected in (
            "https://api.github.com/repositories/0/tarball/main",
            "https://api.github.com/repositories/not-a-number/tarball/main",
            "https://api.github.com/repositories/123/tarball/dev",
            "https://api.github.com/repositories/123/tarball/main?token=secret",
        ):
            with self.assertRaises(ValueError, msg=rejected):
                validate_api_canonical_location(rejected, "main")

    def test_renamed_identity_needs_numeric_proof(self) -> None:
        repository = RepositoryRef("old-owner", "old-repo", "main")
        location = (
            "https://codeload.github.com/new-owner/new-repo/"
            "legacy.tar.gz/refs/heads/main"
        )
        with self.assertRaisesRegex(ValueError, "changed_identity"):
            validate_codeload_location(
                location,
                repository,
                allow_renamed_identity=False,
            )
        self.assertEqual(
            validate_codeload_location(
                location,
                repository,
                allow_renamed_identity=True,
            ),
            "numeric_canonicalization",
        )

    def test_codeload_keeps_ref_and_authority_strict(self) -> None:
        repository = RepositoryRef("owner", "repo", "main")
        for rejected in (
            "http://codeload.github.com/owner/repo/legacy.tar.gz/refs/heads/main",
            "https://codeload.github.com/owner/repo/legacy.tar.gz/refs/heads/dev",
            "https://codeload.github.com/owner/repo/legacy.tar.gz/refs/heads/main/extra",
            "https://codeload.github.com/owner/repo/legacy.tar.gz/refs/heads/main#fragment",
        ):
            with self.assertRaises(ValueError, msg=rejected):
                validate_codeload_location(
                    rejected,
                    repository,
                    allow_renamed_identity=False,
                )


if __name__ == "__main__":
    unittest.main()
