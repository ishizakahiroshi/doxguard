"""Offline, synthetic release identity checks. No GitHub or npm writes."""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / ".github/scripts/resolve-release-source.sh"
with tempfile.TemporaryDirectory(prefix="release-identity-") as temporary:
    root = Path(temporary)
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
               GIT_AUTHOR_NAME="Synthetic Fixture", GIT_COMMITTER_NAME="Synthetic Fixture",
               GIT_AUTHOR_EMAIL="fixture@example.com", GIT_COMMITTER_EMAIL="fixture@example.com")
    def git(*args):
        return subprocess.check_output(["git", "-c", "commit.gpgSign=false", *args],
                                       cwd=root, env=env, text=True).strip()
    git("init", "-q")
    (root / "package.json").write_text('{"version":"1.0.0"}')
    git("add", "package.json")
    git("commit", "-qm", "synthetic A")
    a = git("rev-parse", "HEAD")
    git("tag", "v1.0.0")
    git("commit", "--allow-empty", "-qm", "synthetic B same version")
    b = git("rev-parse", "HEAD")
    cases = [("push", "", a, "v1.0.0", True),
             ("push", "", b, "v1.0.0", False),
             ("workflow_dispatch", "false", a, "v1.0.0", True),
             ("workflow_dispatch", "false", b, "v1.0.0", False),
             ("workflow_dispatch", "true", b, "v1.0.0", True),
             ("workflow_dispatch", "true", b, "v9.9.9", False),
             ("workflow_dispatch", "true", b, "v1.0.0;touch SENTINEL", False)]
    for number, (event, npm_only, sha, tag, expected) in enumerate(cases):
        output = root / f"output-{number}"
        case_env = dict(env, EVENT_NAME=event, NPM_ONLY=npm_only, EVENT_SHA=sha,
                        RELEASE_TAG=tag, GITHUB_OUTPUT=str(output))
        result = subprocess.run(["bash", str(SCRIPT)], cwd=root, env=case_env,
                                capture_output=True, text=True, timeout=20)
        assert (result.returncode == 0) == expected, result.stderr
        value = output.read_text() if output.exists() else ""
        assert value == (f"commit={a}\n" if expected else ""), value
    assert not (root / "SENTINEL").exists()
    workflow = (ROOT / ".github/workflows/release.yml").read_text()
    assert workflow.count("ref: ${{ needs.resolve-source.outputs.commit }}") == 4
    assert "needs.resolve-source.result == 'success'" in workflow
    assert "SOURCE_SHA: ${{ steps.source.outputs.commit }}" in workflow
    print("7 release identity cases and 4 workflow binding checks passed")
