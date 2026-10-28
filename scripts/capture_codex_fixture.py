#!/usr/bin/env python3
"""Opt-in sanitized projection of real, verified native Codex evidence.

No raw database, identifier, path, timestamp, prompt or preview is written.
The output is a small contract sample, not an entire App Server response capture.
"""
import argparse
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allow-real", action="store_true", required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--state-root", type=Path, required=True)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    home = Path.home()
    output = args.output.resolve()
    protected = [args.state_root.resolve(), (home / ".codex").resolve(),
                 (home / ".claude").resolve(),
                 (home / "Library/Application Support/Claude").resolve()]
    if any(output == root or root in output.parents for root in protected):
        parser.error("fixture output must be outside provider state")
    if output.exists() or not output.parent.is_dir():
        parser.error("output must be a new directory with an existing parent")
    command = [str(args.binary), "--json", "--provider", "codex", "--codex-root",
               str(args.state_root), "--codex-executable", str(args.executable), "scan"]
    process = subprocess.run(command, capture_output=True, timeout=30)
    report = json.loads(process.stdout)["report"]
    if report["failures"] or len(report["snapshots"]) != 1:
        raise ValueError("provider did not return a discovery snapshot")
    snapshot = report["snapshots"][0]
    if snapshot["evidence_status"] != "snapshot_verified":
        raise ValueError("fixture requires independently verified snapshot evidence")
    projects = {project["id"]: project for project in snapshot["projects"]}
    mapped_projects = {}
    pages = []
    sample_count = 0
    for archived, state in [(False, "active"), (True, "archived")]:
        records = []
        for session in [s for s in snapshot["sessions"] if s["state"] == state][:3]:
            sample_count += 1
            project = projects.get(session["project_id"], {})
            project_key = project.get("path") or session["project_id"]
            if project_key not in mapped_projects:
                mapped_projects[project_key] = f"/sanitized/workspace-{len(mapped_projects) + 1}"
            records.append({
                "id": f"019a0000-0000-7000-8000-{sample_count:012d}",
                "cwd": mapped_projects[project_key], "updatedAt": 1700000000 + sample_count,
                "parentThreadId": None, "projectId": None,
                "status": {"type": "notLoaded"},
            })
        pages.append({"archived": archived,
                      "response": {"id": len(pages) + 2,
                                   "result": {"data": records, "nextCursor": None}}})
    manifest = {
        "kind": "agentmender_codex_app_server_fixture",
        "schemaVersion": snapshot["provider"]["fingerprint"]["executable_version"],
        "provenance": "observed_locally_sanitized", "complete": False, "pages": pages,
    }
    provenance = {
        "classification": "OBSERVED LOCALLY", "mechanism": "SQLite Backup API mirror + official thread/list",
        "providerVersion": manifest["schemaVersion"],
        "observedNativeSummaries": len(snapshot["sessions"]), "sampledSummaries": sample_count,
        "sanitization": "IDs, cwd and times replaced; body fields excluded; normalized metadata projection only",
        "limitations": "small partial sample; not a raw provider capture; temporary runtime status is not real-client liveness",
    }
    output.mkdir(mode=0o700)
    for name, value in [("manifest.json", manifest), ("provenance.json", provenance)]:
        with (output / name).open("x") as destination:
            json.dump(value, destination, indent=2)
            destination.write("\n")
    print(json.dumps({"sampled_summaries": sample_count,
                      "observed_native_summaries": len(snapshot["sessions"])}))


if __name__ == "__main__":
    main()
