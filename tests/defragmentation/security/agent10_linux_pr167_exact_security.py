#!/usr/bin/env python3
"""Independent Agent10 exact-SHA Linux standalone combined PR168 trust-boundary regression.

Source inspection and disposable committed Git negative controls ONLY.
Never executes code from the candidate, starts a paid effect, publishes releases,
modifies original checked-out bytes or approves any native WebKit/WebView result.
"""
from __future__ import annotations

import argparse
import re
import subprocess
import tempfile
from pathlib import Path

BASE = "0f8e33f0e2abef003440ef27ef76b7cb448d2b32"
CANDIDATE = "a521b706feb278f101fe77fd9fae4445fddd9e0b"
GUARD_COMMIT = "55ce3ac389b7e1fcab99b3a5acc84b526630af72"
GUARD_BLOB = "5c746abd3caf414ba2c80be825d66983312236c6"
WORKFLOW = ".github/workflows/linux-tauri.yml"
ALLOWED = (
    ".github/workflows/windows-tauri.yml",
    ".github/workflows/installed-webview-acceptance.yml",
    ".github/workflows/linux-tauri.yml",
)
STEP = re.compile(r"(?m)^ {6}- (?:name|uses): .+$")
STEP_NAMES = (
    None,
    "Confirm exact unmerged source checkout",
    None,
    "Checkout immutable reviewed Agent11 pre-Vite gate",
    "Validate pinned guard bytes and audited source before any PR-controlled code",
    "Verify independently reviewed Linux package executable source scope",
)
NEGATIVE_FILES = (
    ("Rust build script", "apps/desktop/src-tauri/build.rs"),
    ("npm hook", "apps/desktop-ui/package.json"),
    ("npm dependency lock", "apps/desktop-ui/package-lock.json"),
    ("Cargo dependency lock", "Cargo.lock"),
    ("Python runtime builder", "sidecars/transcription/build_runtime.py"),
    ("Python requirements", "sidecars/transcription/requirements-bundle.txt"),
    ("unreviewed preview publisher", ".github/workflows/tauri-next-release.yml"),
    ("extra arbitrary workflow", ".github/workflows/new-privileged.yml"),
    ("native QA harness", "tests/defragmentation/native-e2e/windows_webview2.ps1"),
    ("bundled resource", "apps/desktop/src-tauri/resources/injected.json"),
)


def steps_of(source: str) -> list[str]:
    head = re.search(r"(?m)^  build-linux-deb:\s*$", source)
    if not head:
        return []
    tail = source[head.end():]
    next_job = re.search(r"(?m)^  [\w-]+:\s*$", tail)
    body = tail[:next_job.start()] if next_job else tail
    matches = list(STEP.finditer(body))
    return [body[m.start():matches[i + 1].start() if i + 1 < len(matches) else len(body)]
            for i, m in enumerate(matches)]


def contains(step: str, literal: str) -> bool:
    return literal in step


def inspect(source: str) -> list[str]:
    findings: list[str] = []
    s = steps_of(source)
    if len(s) < 8:
        return ["Linux canonical build job or its minimum steps missing"]
    if not re.search(r"(?m)^permissions:\s*\n  contents:\s*read\s*$", source):
        findings.append("Missing read-only workflow token permissions")

    for i, required in enumerate(STEP_NAMES):
        if required and not re.search(rf"(?m)^      - name: {re.escape(required)}\s*$", s[i]):
            findings.append(f"Mandatory step {i + 1} changed/reordered: {required}")

    first = s[0]
    if not contains(first, "uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262"):
        findings.append("First source checkout must use exact immutable action")
    if not re.search(r"(?m)^\s+persist-credentials:\s*false\s*$", first):
        findings.append("First checkout leaks persisted credentials")

    assertion = s[1]
    if not contains(assertion, "Confirm exact unmerged source checkout"):
        findings.append("Missing first SHA-only source assertion")
    if not re.search(r'(?m)^\s+run:\s*test "\$\(git rev-parse HEAD\)" = "\$EXPECTED_SOURCE_SHA"\s*$', assertion):
        findings.append("Source SHA assertion changed/extended")

    setup = s[2]
    if not contains(setup, "uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020"):
        findings.append("Pre-guard setup-node action not exact pinned version")
    if re.search(r"(?m)^\s+cache:\s*\S+", setup):
        findings.append("Pre-guard untrusted npm cache enabled")
    if re.search(r"(?m)^\s+run:\s*", setup):
        findings.append("Executable shell introduced in setup-node metadata")

    trusted = s[3]
    if not contains(trusted, "uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262"):
        findings.append("Trusted checkout action not pinned")
    if not re.search(rf"(?m)^\s+ref:\s*{GUARD_COMMIT}\s*$", trusted):
        findings.append("Trusted guard checkout commit not immutable")
    if not re.search(r"(?m)^\s+persist-credentials:\s*false\s*$", trusted):
        findings.append("Trusted guard checkout persists credentials")
    if not contains(trusted, "apps/desktop-ui/scripts/guard-vite-build-patterns.mjs"):
        findings.append("Reviewed scanner path missing from trusted checkout")

    verified = s[4]
    required_checks = (
        ("set -euo pipefail", "guard validation not fail-closed"),
        ('rev-parse HEAD', "immutable trusted HEAD not authenticated"),
        (GUARD_COMMIT, "trusted guard identity missing"),
        ('rev-parse "HEAD:$guard"', "expected committed guard file not checked"),
        (GUARD_BLOB, "reviewed guard committed blob not pinned"),
        ('hash-object --path="$guard" "$guard"', "actual executing guard bytes not hashed"),
        ('diff --quiet -- "$guard"', "guard worktree cleanliness not asserted"),
        ("--self-test", "trusted guard self-test not executed"),
        ("--workspace apps/desktop-ui", "real Vite source guard not executed"),
    )
    for expected, label in required_checks:
        if expected not in verified:
            findings.append(label)
    ordered = [
        verified.find('rev-parse HEAD'),
        verified.find('rev-parse "HEAD:$guard"'),
        verified.find('hash-object --path="$guard" "$guard"'),
        verified.find('diff --quiet -- "$guard"'),
        verified.find("--self-test"),
        verified.find("--workspace apps/desktop-ui"),
    ]
    if min(ordered) >= 0 and ordered != sorted(ordered):
        findings.append("Guard worktree authentication occurs after guard execution")

    scope = s[5]
    required_scope = (
        ("set -euo pipefail", "source scope check not fail-closed"),
        ("reviewed=" + BASE, "independently reviewed source baseline changed"),
        ('git fetch --no-tags --depth=1 origin "$reviewed"', "immutable baseline not fetched"),
        ('git cat-file -t "$reviewed"', "reviewed baseline commit type not checked"),
        ('git diff --quiet "$reviewed" HEAD -- .', "complete tracked source comparison missing"),
        (":(exclude)" + ALLOWED[0], "Windows workflow allowlist missing"),
        (":(exclude)" + ALLOWED[1], "installed native workflow allowlist missing"),
        (":(exclude)" + ALLOWED[2], "Linux workflow allowlist missing"),
        ("sidecars/transcription/", "Python sidecar source not checked"),
        ("scripts/", "source scripts not checked"),
        ("tests/defragmentation/native-e2e/", "native QA source not checked"),
        (":(glob)**/*.py", "global Python source not checked"),
        (":(glob)**/requirements*.txt", "Python requirements not checked"),
        (":(glob)**/pyproject.toml", "Python build config not checked"),
        (":(glob)**/setup.py", "Python setup not checked"),
        (":(glob)**/setup.cfg", "Python setup config not checked"),
        (":(glob)**/Pipfile*", "Pipfile not checked"),
        (":(glob)**/tox.ini", "tox config not checked"),
    )
    for expected, label in required_scope:
        if expected not in scope:
            findings.append(label)
    if scope.count(':(exclude)') != 3:
        findings.append("Expanded exclusions beyond exactly three trusted workflows")

    # No free-form actions or shell can execute ahead of these *two* guard steps.
    for i, part in enumerate(s[:4]):
        if i != 1 and re.search(r"(?m)^\s+run:\s*", part):
            findings.append(f"Arbitrary executable run before trusted guard at step {i + 1}")
        if i in (0, 2, 3) and re.search(r"(?m)^\s+cache:\s*\S+", part):
            findings.append(f"Untrusted preguard package cache at step {i + 1}")
    if re.search(r"(?m)^\s+run:\s*", trusted):
        findings.append("Untrusted run embedded in trusted checkout step")
    return findings


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", "-C", str(repo), *args], text=True,
                          capture_output=True, check=check)


def scope_diff(repo: Path, before: str, after: str) -> int:
    return git(repo, "diff", "--quiet", before, after, "--", ".",
               *[":(exclude)" + path for path in ALLOWED], check=False).returncode


def original_source_check(root: Path) -> None:
    assert git(root, "rev-parse", "HEAD").stdout.strip() == CANDIDATE, "Candidate HEAD changed"
    if git(root, "cat-file", "-e", BASE + "^{commit}", check=False).returncode:
        git(root, "fetch", "--no-tags", "--depth=1", "origin", BASE)
    names = git(root, "diff", "--name-only", BASE, CANDIDATE).stdout.splitlines()
    assert sorted(names) == sorted(ALLOWED), ("Unreviewed candidate source files:", names)
    assert scope_diff(root, BASE, CANDIDATE) == 0, "Source allowlist bypass"
    original = (root / WORKFLOW).read_text(encoding="utf-8")
    findings = inspect(original)
    assert not findings, ("Source trust boundary defects:", findings)
    print("PASS: exact combined PR168 source has only reviewed three-YAML diff, source and guard checks intact")


def mutants(source: str) -> None:
    marker = "      - name: Checkout immutable reviewed Agent11 pre-Vite gate"
    assert marker in source
    checks = [
        ("early Python", source.replace(marker, "      - name: Inject code\n        run: python sidecars/transcription/build_runtime.py\n" + marker, 1)),
        ("early pip", source.replace(marker, "      - name: Inject pip\n        run: pip install -r sidecars/transcription/requirements-bundle.txt\n" + marker, 1)),
        ("early Cargo", source.replace(marker, "      - name: Inject Cargo\n        run: cargo install tauri-cli\n" + marker, 1)),
        ("early npm", source.replace(marker, "      - name: Inject npm\n        run: npm ci\n" + marker, 1)),
        ("early tauri icon", source.replace(marker, "      - name: Inject icon\n        run: tauri icon scriptotar.svg\n" + marker, 1)),
        ("early custom action", source.replace(marker, "      - uses: attacker/supply-chain@main\n" + marker, 1)),
        ("npm preguard cache", source.replace("          node-version: '22'", "          node-version: '22'\n          cache: npm", 1)),
        ("source credentials", source.replace("          persist-credentials: false", "          persist-credentials: true", 1)),
        ("guard credentials", source.replace("          path: .agent11-trusted-guard\n          persist-credentials: false",
                                             "          path: .agent11-trusted-guard\n          persist-credentials: true", 1)),
        ("unpinned scanner", source.replace("          ref: " + GUARD_COMMIT, "          ref: main", 1)),
        ("missing guard hash", source.replace('hash-object --path="$guard" "$guard"', 'hash-object-missing --path="$guard" "$guard"', 1)),
        ("missing clean guard checkout", source.replace('diff --quiet -- "$guard"', 'diff --not-quiet -- "$guard"', 1)),
        ("late guard authentication", source.replace("          git -C \"$trusted\" diff --quiet -- \"$guard\"\n",
                                                     "          git -C \"$trusted\" diff --quiet -- \"$guard\"\n", 1).replace("          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --self-test",
                                                     "          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --workspace apps/desktop-ui\n          node .agent11-trusted-guard/apps/desktop-ui/scripts/guard-vite-build-patterns.mjs --self-test", 1)),
        ("reviewed baseline changed", source.replace("reviewed=" + BASE, "reviewed=main", 1)),
        ("full-path source scope narrowed", source.replace('git diff --quiet "$reviewed" HEAD -- .', 'git diff --quiet "$reviewed" HEAD -- scripts/', 1)),
        ("extra allowed workflow", source.replace("':(exclude).github/workflows/linux-tauri.yml'; then",
                                                  "':(exclude).github/workflows/linux-tauri.yml' \\\n            ':(exclude).github/workflows/release.yml'; then", 1)),
        ("Python runtime path omitted", source.replace("sidecars/transcription/ scripts/", "not-the-sidecar/ scripts/", 1)),
        ("SHA assertion command extended", source.replace('run: test "$(git rev-parse HEAD)" = "$EXPECTED_SOURCE_SHA"',
                                                          'run: test "$(git rev-parse HEAD)" = "$EXPECTED_SOURCE_SHA"; python scripts/prepare_mcp_tools.py', 1)),
    ]
    for name, mutated in checks:
        assert mutated != source, f"Mutation was inert: {name}"
        problems = inspect(mutated)
        assert problems, f"FAIL OPEN: mutation accepted: {name}"
        print("PASS: fail-closed against " + name)
    print(f"PASS: {len(checks)} independent source/credential/ordering/allowlist mutations")


def git_negative_controls() -> None:
    with tempfile.TemporaryDirectory(prefix="agent10-linux-167-") as d:
        repo = Path(d)
        git(repo, "init", "-q")
        git(repo, "config", "user.name", "Independent Agent10 negative fixture")
        git(repo, "config", "user.email", "agent10@example.invalid")
        for name in ALLOWED:
            path = repo / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("name: initial\n", encoding="utf-8")
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "initial")
        before = git(repo, "rev-parse", "HEAD").stdout.strip()
        for name in ALLOWED:
            (repo / name).write_text("name: approved combination\n", encoding="utf-8")
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "approved-only")
        clean = git(repo, "rev-parse", "HEAD").stdout.strip()
        assert scope_diff(repo, before, clean) == 0, "Valid Linux-only diff was denied"
        for label, name in NEGATIVE_FILES:
            git(repo, "reset", "--hard", clean)
            git(repo, "clean", "-fd")
            attacked = repo / name
            attacked.parent.mkdir(parents=True, exist_ok=True)
            attacked.write_text("unreviewed change\n", encoding="utf-8")
            git(repo, "add", "--all")
            git(repo, "commit", "-qm", label)
            after = git(repo, "rev-parse", "HEAD").stdout.strip()
            assert scope_diff(repo, before, after) != 0, ("FAIL OPEN in real git diff", label)
            print("PASS: reject real committed " + label)
        git(repo, "reset", "--hard", clean)
        git(repo, "clean", "-fd")
        deletion = repo / "apps/desktop-ui/package.json"
        deletion.parent.mkdir(parents=True, exist_ok=True)
        deletion.write_text("existing input\n", encoding="utf-8")
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "seed deletion control")
        parent = git(repo, "rev-parse", "HEAD").stdout.strip()
        deletion.unlink()
        git(repo, "add", "--all")
        git(repo, "commit", "-qm", "unreviewed deletion")
        child = git(repo, "rev-parse", "HEAD").stdout.strip()
        assert scope_diff(repo, parent, child) != 0, "FAIL OPEN: deleted source ignored"
        print("PASS: reject real committed unreviewed deletion")
    print(f"PASS: {len(NEGATIVE_FILES)+1} real committed Git pathspec denials")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", type=Path, default=Path(".agent10-pr167-data"))
    args = parser.parse_args()
    root = args.target.resolve(strict=True)
    original_source_check(root)
    mutants((root / WORKFLOW).read_text(encoding="utf-8"))
    git_negative_controls()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
