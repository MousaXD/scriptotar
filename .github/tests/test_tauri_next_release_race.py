from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/tauri-next-release.yml"
WINDOWS_WORKFLOW = ROOT / ".github/workflows/windows-tauri.yml"
LINUX_WORKFLOW = ROOT / ".github/workflows/linux-tauri.yml"
GUARD = ROOT / ".github/scripts/assert-tauri-next-current-main.sh"


def git(cwd: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", *args], cwd=cwd, check=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    return completed.stdout.strip()


class CurrentMainGuardTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        root = Path(self.tempdir.name)
        self.remote = root / "remote.git"
        self.work = root / "work"
        git(root, "init", "--bare", str(self.remote))
        git(root, "init", str(self.work))
        git(self.work, "config", "user.name", "Release Contract Test")
        git(self.work, "config", "user.email", "release-test@example.invalid")
        git(self.work, "remote", "add", "origin", str(self.remote))
        (self.work / "payload.txt").write_text("A\n", encoding="utf-8")
        git(self.work, "add", "payload.txt")
        git(self.work, "commit", "-m", "A")
        git(self.work, "branch", "-M", "main")
        git(self.work, "push", "-u", "origin", "main")
        self.sha_a = git(self.work, "rev-parse", "HEAD")

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def run_guard(self, source_sha: str) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["SOURCE_SHA"] = source_sha
        return subprocess.run(
            ["bash", str(GUARD)], cwd=self.work, env=env, text=True,
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        )

    def advance_main(self) -> str:
        (self.work / "payload.txt").write_text("B\n", encoding="utf-8")
        git(self.work, "add", "payload.txt")
        git(self.work, "commit", "-m", "B")
        git(self.work, "push", "origin", "main")
        return git(self.work, "rev-parse", "HEAD")

    def test_source_sha_equal_to_main_is_allowed(self) -> None:
        result = self.run_guard(self.sha_a)
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn(self.sha_a, result.stdout)

    def test_older_source_is_rejected_after_main_advances(self) -> None:
        sha_b = self.advance_main()
        result = self.run_guard(self.sha_a)
        self.assertEqual(result.returncode, 42, result.stdout)
        self.assertIn("Stale Scriptotar Next release source", result.stdout)
        self.assertIn(sha_b, result.stdout)

    def test_newer_source_is_allowed_after_main_advances(self) -> None:
        sha_b = self.advance_main()
        result = self.run_guard(sha_b)
        self.assertEqual(result.returncode, 0, result.stdout)


class WorkflowRaceContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")
        cls.packagers = {
            "windows": WINDOWS_WORKFLOW.read_text(encoding="utf-8"),
            "linux": LINUX_WORKFLOW.read_text(encoding="utf-8"),
        }

    def test_rolling_publisher_is_globally_serialized(self) -> None:
        self.assertIn("group: tauri-next-release-publisher", self.workflow)
        self.assertIn("cancel-in-progress: false", self.workflow)
        self.assertNotIn("group: tauri-next-release-${{", self.workflow)

    def test_1_4_1_release_needs_explicit_version_approval(self) -> None:
        publish_condition = next(line.strip() for line in self.workflow.splitlines()
                                 if "if: github.event_name == 'workflow_run'" in line)
        self.assertIn("vars.SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED == 'true'", publish_condition)
        self.assertIn("vars.SCRIPTOTAR_NEXT_1_4_1_DISTRIBUTION_APPROVED == 'true'", publish_condition)
        self.assertEqual(
            publish_condition,
            "if: github.event_name == 'workflow_run' && "
            "github.event.workflow_run.head_branch == 'main' && "
            "vars.SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED == 'true' && "
            "vars.SCRIPTOTAR_NEXT_1_4_1_DISTRIBUTION_APPROVED == 'true'",
        )
        self.assertNotIn("github.event.workflow_run.head_commit.message", publish_condition)
        self.assertNotIn("||", publish_condition)

    def test_commit_message_cannot_override_false_owner_approvals(self) -> None:
        # The previous workflow ORed an exact commit-message string with the
        # independent two-variable approval requirement. In particular,
        # (false && false) || special_commit_message was true.
        special_commit = "owner-approve-scriptotar-next-1.4.1-hotfix-publish-20261112"
        old_authorized = (False and False) or (
            special_commit == "owner-approve-scriptotar-next-1.4.1-hotfix-publish-20261112"
        )
        self.assertTrue(old_authorized, "Regression must reproduce the old shortcut")
        publish_condition = next(line.strip() for line in self.workflow.splitlines()
                                 if "if: github.event_name == 'workflow_run'" in line)
        self.assertNotIn(special_commit, publish_condition)
        # The only two owner-approval clauses are conjunctive, independent of
        # the commit text. Approval variables must be set by repository admins.
        required = (
            "SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED",
            "SCRIPTOTAR_NEXT_1_4_1_DISTRIBUTION_APPROVED",
        )
        import re
        clauses = re.findall(r"vars[.]([A-Z_0-9]+) == 'true'", publish_condition)
        self.assertEqual(clauses, list(required))
        for general in (False, True):
            for version in (False, True):
                repo_vars = dict(zip(required, (general, version)))
                for commit_message in (special_commit, "ordinary source change"):
                    # Commit text is deliberately irrelevant to the derived
                    # GitHub Actions owner-variable approval gate.
                    authorized = all(repo_vars[variable] for variable in clauses)
                    self.assertEqual(authorized, general and version,
                                     (general, version, commit_message))
                    if not (general and version):
                        self.assertFalse(authorized)

    def test_staging_identity_is_unique_for_duplicates_and_retries(self) -> None:
        self.assertIn("github.run_id", self.workflow)
        self.assertIn("github.run_attempt", self.workflow)
        self.assertIn("tauri-next-staging-", self.workflow)

    def test_stale_source_is_checked_after_validation_and_again_before_switch(self) -> None:
        stage_assets = self.workflow.index("Stage one cross-platform preview release")
        first_guard = self.workflow.index("Reject stale source before draft staging")
        stage_draft = self.workflow.index("Stage complete release as a draft")
        second_guard = self.workflow.index("Re-check current main before rolling switch")
        switch = self.workflow.index("Switch rolling tag and publish staged release")
        self.assertLess(stage_assets, first_guard)
        self.assertLess(first_guard, stage_draft)
        self.assertLess(stage_draft, second_guard)
        self.assertLess(second_guard, switch)
        self.assertGreaterEqual(self.workflow.count("assert-tauri-next-current-main.sh"), 2)

    def test_assets_are_complete_before_the_rolling_release_changes(self) -> None:
        create_draft = self.workflow.index('gh release create "$STAGING_TAG"')
        draft_flag = self.workflow.index("--draft", create_draft)
        delete_old = self.workflow.index('gh api --method DELETE "repos/$GITHUB_REPOSITORY/releases/$OLD_RELEASE_ID"')
        move_tag = self.workflow.index('git push origin "refs/tags/$RELEASE_TAG" --force')
        publish_draft = self.workflow.index('"repos/$GITHUB_REPOSITORY/releases/$STAGING_RELEASE_ID"')
        self.assertLess(create_draft, draft_flag)
        self.assertLess(draft_flag, delete_old)
        self.assertLess(delete_old, move_tag)
        self.assertLess(move_tag, publish_draft)

    def test_failure_cleanup_is_always_attempted(self) -> None:
        cleanup = self.workflow.index("Clean up unpublished staging release")
        always = self.workflow.index("if: always()", cleanup)
        self.assertGreater(always, cleanup)

    def test_packagers_cannot_race_the_authoritative_publisher(self) -> None:
        forbidden = (
            "gh release create", "gh release edit", "gh release upload",
            "git tag -f tauri-next-latest", "contents: write",
        )
        for platform, workflow in self.packagers.items():
            for token in forbidden:
                self.assertNotIn(token, workflow, f"{platform}: {token}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
