"""Bounded old-candidate native controls, with exact success/coverage assertions."""
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile

BASELINE = "2e8e9f4f50f16def766409bd8d4401fa8e7a3d43"
ROOT = Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory(prefix="doxguard-positive-") as temporary:
    stage = Path(temporary)
    archive = stage / "baseline.tar"
    with archive.open("wb") as output:
        subprocess.run(["git", "archive", BASELINE], cwd=ROOT, stdout=output, check=True)
    baseline = stage / "baseline"
    baseline.mkdir()
    with tarfile.open(archive) as source:
        source.extractall(baseline, filter="data")
    env = {key: value for key, value in os.environ.items()
           if not key.upper().startswith("GIT_") and key.upper() != "DOXGUARD_CONFIG"}
    env["CARGO_TARGET_DIR"] = str(stage / "target")
    subprocess.run(["cargo", "build", "--locked", "--manifest-path",
                    str(baseline / "Cargo.toml"), "--bin", "doxguard"],
                   cwd=baseline, env=env, check=True, timeout=300)
    binary = stage / "target/debug/doxguard"
    repo = stage / "repo"
    repo.mkdir()
    (repo / "needles.txt").write_text("SyntheticNeedle\n")
    (repo / "doxguard.config.json").write_text(json.dumps({
        "watchlists": [{"type": "lines", "path": "needles.txt"}],
        "includeDefaultExemptPaths": False,
        "structural": {"windowsPath": False, "posixHome": False,
                       "privateIp": False, "email": False}}))
    (repo / "sub").mkdir()
    (repo / "sub/policy.md").write_text("safe alias")
    (repo / "sub\\policy.md").write_text("SyntheticNeedle")
    outer = stage / "outer"
    (outer / "inner").mkdir(parents=True)
    (outer / "policy.md").write_text("SyntheticNeedle")
    (repo / "policy.md").write_text("safe alias")
    (repo / "link").symlink_to(outer / "inner", target_is_directory=True)
    for path in ["sub\\policy.md", "link/../policy.md"]:
        assert (repo / path).read_text() == "SyntheticNeedle"
        result = subprocess.run([str(binary), "scan", "--path", path, "--block",
                                 "--strict", "--format", "json", "--show-matches"],
                                cwd=repo, env=env, capture_output=True, text=True, timeout=20)
        assert result.returncode == 0, result.stderr
        report = json.loads(result.stdout)
        assert report["hits"] == [] and report["coverage_skips"] == 0, report
        assert report["scanned"] == 1 and report["total_files"] == 1, report
        print(f"POSITIVE CONTROL: {path!r}: native target contains needle; old exit 0, hits 0, skips 0")
print("2 old-candidate native positive controls verified")

# F02 is distinct from the original I/O alias: reproduce the exemption alias in
# the already-I/O-fixed 48e candidate, then rely on final native Rust regressions.
F02_BASELINE = "48e1f41577334291fcd4d585ba7c92c093a87dbc"
with tempfile.TemporaryDirectory(prefix="doxguard-exemption-positive-") as temporary:
    stage = Path(temporary)
    archive = stage / "baseline.tar"
    with archive.open("wb") as output:
        subprocess.run(["git", "archive", F02_BASELINE], cwd=ROOT, stdout=output, check=True)
    baseline = stage / "baseline"
    baseline.mkdir()
    with tarfile.open(archive) as source:
        source.extractall(baseline, filter="data")
    env = {key: value for key, value in os.environ.items()
           if not key.upper().startswith("GIT_") and key.upper() != "DOXGUARD_CONFIG"}
    env.update(CARGO_TARGET_DIR=str(stage / "target"), GIT_CONFIG_NOSYSTEM="1",
               GIT_CONFIG_GLOBAL=os.devnull, GIT_AUTHOR_NAME="Synthetic Fixture",
               GIT_COMMITTER_NAME="Synthetic Fixture", GIT_AUTHOR_EMAIL="fixture@example.com",
               GIT_COMMITTER_EMAIL="fixture@example.com")
    subprocess.run(["cargo", "build", "--locked", "--manifest-path",
                    str(baseline / "Cargo.toml"), "--bin", "doxguard"],
                   cwd=baseline, env=env, check=True, timeout=300)
    binary = stage / "target/debug/doxguard"
    repo = stage / "repo"
    repo.mkdir()
    hooks = stage / "empty-hooks"
    hooks.mkdir()
    def git(*args):
        subprocess.run(["git", "-c", f"core.hooksPath={hooks}",
                        "-c", "commit.gpgSign=false", *args],
                       cwd=repo, env=env, check=True, capture_output=True)
    git("init", "-q")
    git("commit", "--allow-empty", "-qm", "synthetic baseline")
    settings = {"watchlists": [{"type": "lines", "path": "needles.txt"}],
                "includeDefaultExemptPaths": True,
                "structural": {"windowsPath": False, "posixHome": False,
                               "privateIp": False, "email": False}}
    (repo / "needles.txt").write_text("SyntheticNeedle\n")
    (repo / "doxguard.config.json").write_text(json.dumps(settings))
    (repo / "tests").mkdir()
    (repo / "tests/policy.md").write_text("safe alias")
    (repo / "tests\\policy.md").write_text("SyntheticNeedle")
    git("add", "--", "tests/policy.md", "tests\\policy.md")
    for mode in ["--staged", "--diff", "--all-tracked"]:
        result = subprocess.run([str(binary), "scan", mode, "--block", "--strict",
                                 "--format", "json", "--show-matches"],
                                cwd=repo, env=env, capture_output=True, text=True, timeout=20)
        assert result.returncode == 0, result.stderr
        report = json.loads(result.stdout)
        assert report["hits"] == [] and report["coverage_skips"] == 0, report
        assert report["scanned"] == 0 and report["total_files"] == 2, report
        print(f"F02 POSITIVE CONTROL: {mode}: 48e native exit0/hits0/skips0; default exclusion alias")
    settings["exemptPaths"] = ["tests/"]
    (repo / "doxguard.config.json").write_text(json.dumps(settings))
    result = subprocess.run([str(binary), "scan", "--path", "tests\\policy.md",
                             "--block", "--strict", "--format", "json"],
                            cwd=repo, env=env, capture_output=True, text=True, timeout=20)
    assert result.returncode == 0, result.stderr
    report = json.loads(result.stdout)
    assert report["hits"] == [] and report["coverage_skips"] == 0, report
    assert report["scanned"] == 0 and report["total_files"] == 0, report
    print("F02 POSITIVE CONTROL: explicit custom exclusion: 48e native exit0/hits0/skips0")
print("4 additional 48e native exemption positive controls verified")
