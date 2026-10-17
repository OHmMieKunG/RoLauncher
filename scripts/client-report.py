"""Summarize allowlisted Roblox evidence; missing evidence never becomes a live pass."""
import argparse
import csv
from datetime import datetime, timezone
import json
import math
import re
from pathlib import Path
import statistics


CASES = (
    "official_normal_headless", "bloxstrap_normal_headless", "fishstrap_normal_headless",
    "two_accounts_same_provider", "official_bloxstrap", "official_fishstrap",
    "normal_minimized_hidden_balanced_low_resource", "rendering_suppression",
    "restoration_latency", "unexpected_exit_recovery", "server_disconnect_recovery",
    "stop_restart_peer_isolation", "idle_25_minutes", "idle_60_minutes",
    "activity_25_minutes_no_recovery", "activity_60_minutes_no_recovery", "headless_power_experiments",
    "test_only_hidden_balanced", "different_game_isolation", "launcher_restart_restoration",
    "idle_recovery_25_minutes", "idle_recovery_60_minutes",
)


def distribution(values):
    values = sorted(v for v in values if isinstance(v, (float, int)) and not isinstance(v, bool) and math.isfinite(v))
    if not values:
        return None
    return {"count": len(values), "mean": statistics.fmean(values),
            "median": statistics.median(values), "p95": values[math.ceil(len(values) * .95) - 1],
            "min": values[0], "max": values[-1], "peak": values[-1]}


def timestamp(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00")).timestamp() * 1000


def input_in_interval(account, start, end):
    try:
        return start <= timestamp(account.get("last_input_submitted_at")) <= end
    except (TypeError, AttributeError, ValueError):
        return False


def continuity(events, export):
    """Join/leave evidence plus bounded client progress gaps; uptime alone is insufficient."""
    samples = [e for e in events if e.get("event") == "sample"]
    if not samples or not export:
        return {"status": "UNVERIFIED", "reason": "Independent server export and live samples required"}
    begin = next((e for e in events if e.get("event") == "begin"), {})
    if export.get("schema") != 1 or export.get("run_id") != begin.get("run_id") or export.get("place_id") != begin.get("place_id"):
        return {"status": "UNVERIFIED", "reason": "Observer export does not match run and place"}
    start, end = timestamp(samples[0]["utc"]), timestamp(samples[-1]["utc"])
    ids = {a["account_id"] for s in samples for a in s["accounts"]}
    results = {}
    for account_id in sorted(ids):
        records = [r for r in export.get("records", []) if r.get("account_id") == account_id
                   and r.get("joined_ms", float("inf")) <= end
                   and (r.get("left_ms") is None or r["left_ms"] >= start)]
        record = records[0] if len(records) == 1 else {}
        client = [a for s in samples for a in s["accounts"] if a["account_id"] == account_id]
        identities = {(a.get("pid"), a.get("creation_time"), a.get("generation")) for a in client}
        backend_steady = (len(identities) == 1 and all(all(part is not None for part in identity) for identity in identities)
                          and all(a.get("status") == "running" for a in client)
                          and len({a.get("connected_since") for a in client}) == 1 and client[0].get("connected_since") is not None
                          and len({a.get("last_disconnect") for a in client}) == 1
                          and len({a.get("reconnection_attempts") for a in client}) == 1)
        points = sorted(record.get("samples", []), key=lambda p: p["at_ms"])
        covering = (record.get("joined_ms", float("inf")) <= start and record.get("observed_through_ms", 0) >= end
                    and (record.get("left_ms") is None or record["left_ms"] >= end))
        relevant = [p for p in points if start - 20000 <= p["at_ms"] <= end + 20000]
        gaps = [b["at_ms"] - a["at_ms"] for a, b in zip(relevant, relevant[1:])]
        progress = (len(relevant) >= 2 and relevant[0]["at_ms"] <= start + 20000
                    and relevant[-1]["at_ms"] >= end - 20000 and max(gaps, default=float("inf")) <= 30000
                    and all(b["progress_count"] > a["progress_count"] and b["client_ack_count"] > a["client_ack_count"]
                            for a, b in zip(relevant, relevant[1:])))
        input_points = [p for p in relevant if all(type(p.get(key)) is int and 0 <= p[key] <= 1000000
                        for key in ("input_count", "idle_event_count"))]
        activity_run = begin.get("scenario") in ("Activity25", "Activity60")
        duration_required = {"Activity25": 1500, "Activity60": 3600, "Idle25": 1500, "Idle60": 3600,
                             "IdleRecovery25": 1500, "IdleRecovery60": 3600}.get(begin.get("scenario"), 0)
        duration_complete = end - start >= (duration_required - 2) * 1000
        activity_evidence = (not activity_run or (end - start >= (duration_required - 2) * 1000
            and len(input_points) == len(relevant) and len(input_points) >= 2
            and input_points[-1]["input_count"] > input_points[0]["input_count"]
            and all(b["input_count"] >= a["input_count"] and b["idle_event_count"] >= a["idle_event_count"]
                    for a, b in zip(input_points, input_points[1:]))
            and any(input_in_interval(a, start, end) for a in client)))
        results[account_id] = {"connections_observed": len(records),
                               "observation_duration_complete": duration_complete,
                               "input_events_observed": input_points[-1]["input_count"] - input_points[0]["input_count"] if len(input_points) >= 2 else None,
                               "idle_events_observed": input_points[-1]["idle_event_count"] - input_points[0]["idle_event_count"] if len(input_points) >= 2 else None,
                               "activity_evidence_complete": activity_evidence if activity_run else None,
                               "continuous_connection_and_progress": bool(record.get("nonce") and covering and progress and backend_steady and activity_evidence and duration_complete)}
    return {"status": "EVIDENCE_COMPLETE" if ids and all(v["continuous_connection_and_progress"] for v in results.values()) else "UNVERIFIED",
            "accounts": results, "reason": "Connection intervals remain separate after recovery; this is not a gameplay pass"}


def frames(path, expected_pid):
    # Never copy arbitrary CSV strings, paths or unrecognized columns to the report.
    metrics = {key: [] for key in ("CPUFrameTime", "MsCPUBusy", "MsCPUWait", "MsGPUBusy", "MsGPUTime", "MsGPULatency", "GPUBusy", "DisplayedTime", "MsBetweenPresents")}
    chains = {}
    rows = 0
    with path.open(encoding="utf-8-sig", newline="") as source:
        for row in csv.DictReader(source):
            pid = row.get("ProcessID", row.get("ProcessId"))
            if not pid or int(pid) != expected_pid:
                raise ValueError("PresentMon CSV includes an unexpected process")
            rows += 1
            try:
                clock = float(row.get("CPUStartQPCTime", ""))
                if math.isfinite(clock):
                    chains.setdefault(row.get("SwapChainAddress", "unknown"), []).append(clock)
            except (TypeError, ValueError):
                pass
            for key, values in metrics.items():
                try:
                    value = float(row.get(key, ""))
                except (TypeError, ValueError):
                    continue
                if value >= 0 and math.isfinite(value):
                    values.append(value)
    rates = [{"swapchain_index": i, "timed_presents": len(times),
              "observed_present_rate_hz": (len(times) - 1) * 1000 / (max(times) - min(times)) if len(times) > 1 and max(times) > min(times) else None}
             for i, times in enumerate(chains.values())]
    return {"present_rows": rows, "per_swapchain": rates, "metrics_ms": {k: distribution(v) for k, v in metrics.items()},
            "rendering_disabled": "UNVERIFIED", "note": "Missing presents do not prove zero rendering; displayed duration is not render duration"}


def summarize(events, directory, export=None):
    begin = next((e for e in events if e.get("event") == "begin"), {})
    phases = {}
    for event in events:
        if event.get("event") != "sample" or event.get("phase", "").startswith("settle-"):
            continue
        phase = event["phase"]
        for index, account in enumerate(event["accounts"]):
            if phase.startswith("a") and not phase.startswith(f"a{index}-"):
                continue  # Peer samples prove isolation; they are not the selected-condition benchmark.
            key = (phase, account["account_id"])
            target = phases.setdefault(key, {"phase": phase, "account_id": account["account_id"],
                                             "pid": account.get("pid"), "samples": [], "gpu": {}})
            target["samples"].append(account.get("native", {}))
            for engine in event.get("gpu_engines") or []:
                if engine["Name"].startswith(f"pid_{account.get('pid')}_"):
                    target["gpu"].setdefault(engine["Name"], []).append(engine["UtilizationPercentage"])
    output = []
    for target in phases.values():
        phase = target["phase"]
        result = {k: target[k] for k in ("phase", "account_id", "pid")}
        result["native"] = {key: distribution([s.get(key) for s in target["samples"]])
                            for key in ("cpu_percent_machine", "working_set_bytes", "private_bytes", "page_faults_per_second")}
        result["gpu_engines"] = {name: distribution(values) for name, values in target["gpu"].items()}
        result["gpu_adapter_mapping"] = "LUID/physical index in engine name; correlate with ETW adapter inventory"
        result["total_gpu_percent"] = None
        observations = [a for e in events if e.get("event") == "sample" and e.get("phase") == phase
                        for a in e["accounts"] if a.get("account_id") == target["account_id"]]
        result["test_only"] = "test_only_hidden_balanced" in phase
        result["effective_policies"] = [{key: s[key] if type(s.get(key)) is int and 0 <= s[key] <= 0xffffffff else None for key in ("priority", "power_control_mask", "power_state_mask", "memory_priority")}
                                        for s in target["samples"]]
        result["missing_telemetry"] = {key: sum(a.get(key) is None for a in observations)
                                        for key in ("sampled_at", "gpu_sampled_at", "dedicated_gpu_memory_bytes")}
        result["dedicated_gpu_memory_bytes"] = distribution([a.get("dedicated_gpu_memory_bytes") for a in observations])
        frame_file = directory / f"{phase}-presentmon.csv"
        result["frames"] = frames(frame_file, target["pid"]) if frame_file.is_file() and target["pid"] else None
        peer = next((a for e in events if e.get("event") == "sample" and e.get("phase") == phase
                     for a in e["accounts"] if a.get("account_id") != target["account_id"]), {})
        peer_file = directory / f"{phase}-peer-presentmon.csv"
        result["peer_frames"] = frames(peer_file, peer["pid"]) if peer_file.is_file() and peer.get("pid") else None
        peer_samples = [a.get("native", {}) for e in events if e.get("event") == "sample" and e.get("phase") == phase
                        for a in e["accounts"] if a.get("account_id") == peer.get("account_id")]
        result["peer_account_id"] = peer.get("account_id")
        result["peer_native"] = {key: distribution([s.get(key) for s in peer_samples])
                                 for key in ("cpu_percent_machine", "working_set_bytes", "private_bytes", "page_faults_per_second")}
        diagnostics = "".join(path.read_text(encoding="utf-8-sig", errors="replace") for path in
                              (directory / f"{phase}-presentmon-stdout.txt", directory / f"{phase}-presentmon-stderr.txt") if path.is_file())
        result["capture_diagnostics"] = {"review_required": bool(diagnostics.strip()),
                                           "lost_event_warning": bool(re.search(r"lost.*event|event.*lost|buffer.*lost", diagnostics, re.I))}
        output.append(result)
    endings = [e for e in events if e.get("event") == "end"]
    transitions = [e for e in events if e.get("event") == "transition" and e.get("mode") == "normal"]
    process_versions = [{k: e.get(k) for k in ("account_id", "pid", "player_version")} for e in events if e.get("event") == "process"]
    return {"schema": 1, "version": begin.get("version"), "run_id": begin.get("run_id"), "process_versions":process_versions,
            "status": "FAIL" if any(e.get("status") == "FAIL" for e in endings) else "UNVERIFIED",
            "completed": bool(endings), "phases": output,
            "restore_native_ms": distribution([e.get("native_window_ms") for e in transitions]),
            "restore_backend_ms": distribution([e.get("backend_confirmed_ms") for e in transitions]),
            "server_evidence": continuity(events, export),
            "network_only": "UNSUPPORTED", "rendering_off": "UNSUPPORTED",
            "scope": "RobloxPlayerBeta.exe only; no launcher metrics; independent gameplay and capture-quality review required"}


def blocked(inventory):
    return {"schema": 1, "generated_utc": datetime.now(timezone.utc).isoformat(),
            "status": "BLOCKED", "cases": {case: "BLOCKED" for case in CASES},
            "reason": "Dedicated accounts and authorized test experience are not ready",
            "network_only": "UNSUPPORTED", "roblox_measurements": None,
            "inventory": inventory, "scope": "Preflight inventory is not a Roblox benchmark"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--blocked", type=Path)
    source.add_argument("--journal", type=Path)
    parser.add_argument("--server-export", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.blocked:
        result = blocked(json.loads(args.blocked.read_text(encoding="utf-8-sig")))
    else:
        events = [json.loads(line) for line in args.journal.read_text(encoding="utf-8-sig").splitlines() if line.strip()]
        export = json.loads(args.server_export.read_text(encoding="utf-8-sig")) if args.server_export else None
        result = summarize(events, args.journal.parent, export)
    with args.output.open("x", encoding="utf-8") as output:
        json.dump(result, output, indent=2, allow_nan=False)
        output.write("\n")
    print(f"Report status: {result['status']}; {args.output}")


if __name__ == "__main__":
    main()
