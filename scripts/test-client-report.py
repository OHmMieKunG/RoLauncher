"""Offline fixtures, never Roblox acceptance."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("report", Path(__file__).with_name("client-report.py"))
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)


class EvidenceTests(unittest.TestCase):
    def test_activity_requires_full_duration_input_and_same_connection(self):
        start = report.timestamp("2026-10-17T00:00:00Z")
        events = [{"event": "begin", "scenario": "Activity25", "run_id": "activity", "place_id": 1}]
        points = []
        for seconds in range(0, 1501, 10):
            at = report.datetime.fromtimestamp((start + seconds * 1000) / 1000, report.timezone.utc).isoformat()
            events.append({"event": "sample", "utc": at, "accounts": [{"account_id": "1", "pid": 1,
                "creation_time": "1", "generation": "a", "status": "running", "connected_since": "same",
                "last_input_submitted_at": "2026-10-17T00:10:00Z" if seconds >= 600 else None}]})
            points.append({"at_ms": start + seconds * 1000, "progress_count": seconds,
                "client_ack_count": seconds, "input_count": seconds // 600, "idle_event_count": 0})
        record = {"account_id": "1", "nonce": "one", "joined_ms": start - 1,
            "observed_through_ms": start + 1500000, "samples": points}
        export = {"schema": 1, "run_id": "activity", "place_id": 1, "records": [record]}
        self.assertEqual(report.continuity(events, export)["status"], "EVIDENCE_COMPLETE")
        self.assertEqual(report.continuity(events[:-2], export)["status"], "UNVERIFIED")
        for scenario in ("Idle25", "Idle60", "IdleRecovery25", "IdleRecovery60"):
            events[0]["scenario"] = scenario
            self.assertEqual(report.continuity(events[:4], export)["status"], "UNVERIFIED")
        events[0]["scenario"] = "Activity25"
        for event in events[1:]: event["accounts"][0]["last_input_submitted_at"] = "2026-10-16T00:00:00Z"
        self.assertEqual(report.continuity(events, export)["status"], "UNVERIFIED")
        for event in events[1:]: event["accounts"][0]["last_input_submitted_at"] = None
        self.assertEqual(report.continuity(events, export)["status"], "UNVERIFIED")
        self.assertNotEqual(report.continuity(events, export)["status"], "PASS")

    def test_distributions_and_missing_metrics(self):
        self.assertIsNone(report.distribution([None, float("nan"), float("inf"), True]))
        self.assertEqual(report.distribution([3, 1, 2])["median"], 2)
        self.assertEqual(report.distribution(list(range(100)))["p95"], 94)
        self.assertEqual(report.distribution([3, 1, 2])["peak"], 3)

    def test_balanced_experiment_preserves_policy_and_missing_telemetry(self):
        events = [{"event":"begin"}, {"event":"sample", "phase":"a0-b0-test_only_hidden_balanced", "accounts":[
            {"account_id":"1", "pid":10, "dedicated_gpu_memory_bytes":None, "native":{"priority":16384,"power_control_mask":0,"memory_priority":5}}
        ]}]
        with tempfile.TemporaryDirectory() as directory:
            phase = report.summarize(events,Path(directory))["phases"][0]
        self.assertTrue(phase["test_only"])
        self.assertEqual(phase["effective_policies"][0]["power_control_mask"],0)
        self.assertEqual(phase["missing_telemetry"]["gpu_sampled_at"],1)
        self.assertIsNone(phase["dedicated_gpu_memory_bytes"])

    def test_blocked_is_not_a_measurement(self):
        result = report.blocked({"active_processes": []})
        self.assertEqual(set(result["cases"].values()), {"BLOCKED"})
        self.assertIsNone(result["roblox_measurements"])
        self.assertEqual(result["network_only"], "UNSUPPORTED")

    def test_peer_and_launcher_metrics_are_excluded_and_strings_not_copied(self):
        events = [{"event": "begin", "version": "fixture"}, {"event": "sample", "phase": "a0-b0-normal", "accounts": [
            {"account_id": "1", "pid": 10, "native": {"cpu_percent_machine": 2}, "ticket": "SECRET"},
            {"account_id": "2", "pid": 20, "native": {"cpu_percent_machine": 99}}],
            "gpu_engines": [{"Name": "pid_10_luid_1_phys_0_eng_0_engtype_3D", "UtilizationPercentage": 5},
                            {"Name": "pid_999_launcher", "UtilizationPercentage": 90}]}]
        with tempfile.TemporaryDirectory() as directory:
            result = report.summarize(events, Path(directory))
        self.assertEqual(len(result["phases"]), 1)
        self.assertEqual(result["phases"][0]["native"]["cpu_percent_machine"]["mean"], 2)
        self.assertEqual(result["phases"][0]["peer_native"]["cpu_percent_machine"]["mean"], 99)
        self.assertNotIn("SECRET", json.dumps(result))
        self.assertNotIn("launcher\"", json.dumps(result))
        self.assertEqual(result["status"], "UNVERIFIED")
        self.assertFalse(result["completed"])

    def test_presentmon_rejects_wrong_pid_and_does_not_infer_no_render(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frames.csv"
            path.write_text("ProcessID,MsGPUBusy,CPUStartQPCTime,SwapChainAddress,Application\n10,0,1000,0x123,SECRET\n10,1,1020,0x123,SECRET\n", encoding="utf-8")
            result = report.frames(path, 10)
            self.assertEqual(result["present_rows"], 2)
            self.assertEqual(result["metrics_ms"]["MsGPUBusy"]["mean"], .5)
            self.assertEqual(result["per_swapchain"][0]["observed_present_rate_hz"], 50)
            self.assertEqual(result["rendering_disabled"], "UNVERIFIED")
            self.assertNotIn("SECRET", json.dumps(result))
            with self.assertRaises(ValueError):
                report.frames(path, 20)

    def test_server_export_requires_matching_run_connection_and_progress(self):
        start = report.timestamp("2026-10-17T00:00:00Z")
        events = [{"event": "begin", "run_id": "run-a", "place_id": 1}]
        events += [{"event": "sample", "utc": f"2026-10-17T00:00:{s:02d}Z", "accounts": [{"account_id": "1", "pid":10,
                     "creation_time":"123", "generation":"fixture", "status":"running", "connected_since":"same-connection"}]} for s in (0, 10, 20)]
        record = {"account_id": "1", "nonce": "connection-a", "joined_ms": start - 1, "left_ms": None, "observed_through_ms":start + 20000,
                  "samples": [{"at_ms": start + i * 10000, "progress_count": i, "client_ack_count": i} for i in range(3)]}
        export = {"schema": 1, "run_id": "run-a", "place_id": 1, "records": [record]}
        self.assertEqual(report.continuity(events, export)["status"], "EVIDENCE_COMPLETE")
        record["observed_through_ms"] = start + 10000
        self.assertEqual(report.continuity(events, export)["status"], "UNVERIFIED")
        record["observed_through_ms"] = start + 20000
        events[-1]["accounts"][0]["pid"] = 20
        self.assertEqual(report.continuity(events, export)["status"], "UNVERIFIED")
        events[-1]["accounts"][0]["pid"] = 10
        record["samples"][-1]["client_ack_count"] = 1
        self.assertEqual(report.continuity(events, export)["status"], "UNVERIFIED")
        record["samples"][-1]["client_ack_count"] = 2
        export["records"].append(dict(record, nonce="connection-b"))
        self.assertEqual(report.continuity(events, export)["status"], "UNVERIFIED")
        export["run_id"] = "another-run"
        self.assertEqual(report.continuity(events, export)["status"], "UNVERIFIED")


if __name__ == "__main__":
    unittest.main()
