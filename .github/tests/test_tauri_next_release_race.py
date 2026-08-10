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
        ["git", *args],
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
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
            ["bash", str(GUARD)],
            cwd=self.work,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
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
        self.assertGreaterEqual(
            self.workflow.count("assert-tauri-next-current-main.sh"), 2
        )

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
            "gh release create",
            "gh release edit",
            "gh release upload",
            "git tag -f tauri-next-latest",
            "contents: write",
        )
        for platform, workflow in self.packagers.items():
            for token in forbidden:
                self.assertNotIn(token, workflow, f"{platform}: {token}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
