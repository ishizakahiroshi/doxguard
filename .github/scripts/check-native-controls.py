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
