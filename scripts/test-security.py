"""Behavioral tests for the read-only security CLI; all credentials are synthetic."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("security.py")


class SecurityTests(unittest.TestCase):
    def cli(self, *args):
        return subprocess.run([sys.executable, str(SCRIPT), *map(str, args)],
                              capture_output=True, text=True)

    def test_clean_audit_report_is_explicit(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "audit.json"
            report.write_text(json.dumps({"database": {"last-commit": "a" * 40},
                                         "vulnerabilities": {"found": False, "count": 0, "list": []},
                                         "warnings": {}}))
            result = self.cli("evaluate", "--kind", "rust", "--report", report, "--exit-code", 0)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["status"], "clean")

    def test_advisories_are_visible_baseline_not_an_unapproved_merge_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "audit.json"
            report.write_text(json.dumps({"database": {"last-commit": "a" * 40},
                "vulnerabilities": {"found": True, "count": 1, "list": [{
                    "advisory": {"id": "RUSTSEC-2099-0001", "cvss": "CVSS:3.1/AV:N"},
                    "package": {"name": "synthetic-package", "version": "1.0.0"},
                    "versions": {"patched": [">=1.0.1"]}}]}, "warnings": {}}))
            result = self.cli("evaluate", "--kind", "rust", "--report", report, "--exit-code", 1)
            self.assertEqual(result.returncode, 0)
            data = json.loads(result.stdout)
            self.assertEqual(data["status"], "findings")
            self.assertEqual(data["findings"][0]["advisory"]["id"], "RUSTSEC-2099-0001")

    def test_missing_database_and_scanner_errors_are_unverified(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "audit.json"
            for content, code in [({}, 0), ({"database": {"last-commit": "a" * 40}}, 3),
                                  ({"database": {"last-commit": "a" * 40},
                                    "vulnerabilities": {"found": False, "count": 2, "list": []}}, 0)]:
                with self.subTest(content=content, code=code):
                    report.write_text(json.dumps(content))
                    result = self.cli("evaluate", "--kind", "rust", "--report", report, "--exit-code", code)
                    self.assertEqual(result.returncode, 2)
                    self.assertEqual(json.loads(result.stdout)["status"], "unverified")

    def test_secret_tool_report_is_allowlisted_and_errors_never_echo_raw_output(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "secrets.json"
            canary = "ghp_" + "SYNTHETIC0123456789" * 2
            report.write_text(json.dumps([{"RuleID": "github-pat", "File": "fixture.txt",
                "StartLine": 3, "Commit": "a" * 40, "Secret": canary,
                "Match": "password=" + canary, "Author": canary}]))
            result = self.cli("evaluate", "--kind", "secrets", "--report", report, "--exit-code", 1)
            self.assertEqual(result.returncode, 1)
            self.assertNotIn(canary, result.stdout + result.stderr)
            self.assertEqual(json.loads(result.stdout)["findings"], [{"rule": "github-pat",
                "path": "fixture.txt", "line": 3, "commit": "a" * 40}])

    def test_nuget_incomplete_and_vulnerable_results_remain_distinct(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "nuget.json"
            report = {"version": 1, "sources": ["https://api.nuget.org/v3/index.json"], "projects": [{
                "path": "fixture.csproj", "frameworks": [{"framework": "net8.0", "transitivePackages": [{
                    "id": "Synthetic.Package", "resolvedVersion": "1.2.3", "vulnerabilities": [{
                        "severity": "High", "advisoryurl": "https://github.com/advisories/GHSA-0000-0000-0000"}]}]}]}]}
            path.write_text(json.dumps(report))
            result = self.cli("evaluate", "--kind", "nuget", "--report", path, "--exit-code", 0)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["status"], "findings")
            for broken in ({"version": 1, "projects": []}, {"version": 1, "projects": [{}], "sources": []},
                           {**report, "problems": [{"severity": "error", "text": "failed advisory query"}]}):
                path.write_text(json.dumps(broken))
                result = self.cli("evaluate", "--kind", "nuget", "--report", path, "--exit-code", 0)
                self.assertEqual(result.returncode, 2)

    def test_missing_scanner_and_process_errors_are_unverified_without_diagnostics(self):
        with tempfile.TemporaryDirectory() as directory:
            for tool in ("rolauncher-intentionally-absent-scanner", sys.executable):
                result = self.cli("audit", "--kind", "rust", "--root", directory, "--tool", tool)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(json.loads(result.stdout), {"findings": [], "status": "unverified"})

    def test_error_report_never_prints_a_secret_or_sensitive_tool_message(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "bad-report.json"
            canary = "SYNTHETIC_PRIVATE_VALUE_" + "not-a-real-credential"
            report.write_text(json.dumps({"error": canary, "Secret": canary}))
            for kind in ("rust", "nuget", "secrets"):
                result = self.cli("evaluate", "--kind", kind, "--report", report, "--exit-code", 3)
                self.assertEqual(result.returncode, 2)
                self.assertNotIn(canary, result.stdout + result.stderr)

    @unittest.skipUnless(os.environ.get("CARGO_AUDIT"), "Set CARGO_AUDIT to the pinned scanner executable")
    def test_real_unreachable_advisory_database_is_unverified(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.lock").write_text('version = 4\n[[package]]\nname="fixture"\nversion="0.1.0"\n')
            result = self.cli("audit", "--kind", "rust", "--root", root,
                "--tool", os.environ["CARGO_AUDIT"], "--database", root / "absent-db",
                "--database-url", (root / "absent-upstream").as_uri())
            self.assertEqual(result.returncode, 2)
            self.assertEqual(json.loads(result.stdout)["status"], "unverified")

    def test_tool_download_cache_tampering_is_not_executed(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "gitleaks_8.30.1_linux_x64.tar.gz").write_bytes(b"synthetic tampered archive")
            result = subprocess.run([sys.executable, str(SCRIPT.with_name("security-tools.py")),
                "--directory", directory, "--platform", "linux", "--offline"], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn("unverified", result.stdout)
            self.assertFalse((Path(directory) / "gitleaks").exists())

    def test_empty_nuget_test_lock_is_not_a_clean_runtime_audit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "fixture.csproj").write_text('<Project Sdk="Microsoft.NET.Sdk"/>')
            (root / "packages.lock.json").write_text(json.dumps({"version": 1, "dependencies": {"net8.0": {}}}))
            result = self.cli("audit", "--kind", "nuget", "--root", root, "--project", "fixture.csproj",
                              "--tool", "intentionally-absent")
            self.assertEqual(result.returncode, 0)
            self.assertEqual(json.loads(result.stdout)["status"], "not-applicable")

    def test_workflow_uses_untrusted_pr_boundary_without_writes_secrets_or_artifacts(self):
        # JSON is valid YAML; stdlib parsing keeps the trust contract test dependency-free.
        workflow = json.loads(SCRIPT.parent.parent.joinpath(".github/workflows/security.yml").read_text())
        self.assertEqual(set(workflow["on"]), {"pull_request", "workflow_dispatch"})
        self.assertEqual(workflow["on"]["pull_request"]["branches"], ["dev", "main"])
        self.assertEqual(workflow["permissions"], {"contents": "read"})
        for job in workflow["jobs"].values():
            self.assertNotIn("permissions", job)
            for step in job["steps"]:
                if "uses" in step:
                    self.assertRegex(step["uses"], r"^[a-zA-Z0-9/-]+@[a-f0-9]{40}$")
                    self.assertNotIn("upload", step["uses"])
                if step.get("uses", "").startswith("actions/checkout@"):
                    self.assertFalse(step["with"]["persist-credentials"])
                text = json.dumps(step)
                self.assertNotIn("secrets.", text)
                self.assertNotIn("github.event.pull_request.title", text)
                self.assertNotIn("github.head_ref", text)
        self.assertNotIn("publish-release", json.dumps(workflow))
        self.assertNotIn("release-intents", json.dumps(workflow))

    @unittest.skipUnless(os.environ.get("CARGO_DENY"), "Set CARGO_DENY to the pinned scanner executable")
    def test_real_license_scanner_reports_missing_and_incompatible_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("pub fn synthetic() {}\n")
            for license, status in [("MIT", "clean"), (None, "findings"), ("GPL-2.0-only", "findings")]:
                manifest = '[package]\nname="synthetic-license"\nversion="0.1.0"\nedition="2021"\n'
                if license:
                    manifest += f'license="{license}"\n'
                (root / "Cargo.toml").write_text(manifest)
                subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=root, capture_output=True, check=True)
                result = self.cli("audit", "--kind", "licenses", "--root", root,
                                  "--tool", os.environ["CARGO_DENY"])
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(json.loads(result.stdout)["status"], status, result.stdout)

    @unittest.skipUnless(os.environ.get("GITLEAKS"), "Set GITLEAKS to the pinned scanner executable")
    def test_real_secret_scanner_clean_canary_false_positive_and_history(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            def commit():
                subprocess.run(["git", "-C", str(root), "add", "."], check=True)
                subprocess.run(["git", "-C", str(root), "-c", "user.name=Synthetic",
                                "-c", "user.email=synthetic@example.invalid", "commit", "-qm", "fixture"], check=True)
            fixture = root / "fixture.txt"
            fixture.write_text("cookie=$ENV_COOKIE\nhttps://example.invalid/documentation\n")
            commit()
            clean = self.cli("secrets", "--root", root, "--gitleaks", os.environ["GITLEAKS"])
            self.assertEqual(clean.returncode, 0, clean.stdout + clean.stderr)
            base = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
            canary = "ghp_" + "Ab12Cd34Ef56Gh78Ij90Kl12Mn34Op56Qr78"
            cookie = "_|WARNING:" + "-DO-NOT-SHARE-THIS." + "SYNTHETIC0123456789" * 3
            launch = "roblox-player:1+launchmode:play+authenticationTicket:" + "SYNTHETIC0123456789" * 3
            fixture.write_text(f"token={canary}\ncookie={cookie}\n{launch}\n")
            commit()
            detected = self.cli("secrets", "--root", root, "--gitleaks", os.environ["GITLEAKS"])
            self.assertEqual(detected.returncode, 1, detected.stdout + detected.stderr)
            for value in (canary, cookie, launch):
                self.assertNotIn(value, detected.stdout + detected.stderr)
            rules = {f["rule"] for f in json.loads(detected.stdout)["findings"]}
            self.assertTrue({"github-pat", "roblox-cookie", "roblox-authenticated-launch"} <= rules)
            fixture.write_text("credential removed\n")
            commit()
            head = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
            history = self.cli("secrets", "--root", root, "--base", base, "--head", head,
                               "--gitleaks", os.environ["GITLEAKS"])
            self.assertEqual(history.returncode, 1, history.stdout + history.stderr)
            self.assertNotIn(canary, history.stdout + history.stderr)



if __name__ == "__main__":
    unittest.main()
