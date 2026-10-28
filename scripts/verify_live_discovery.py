#!/usr/bin/env python3
"""Explicit opt-in live query verification; emits aggregates, never raw history.

Fingerprints omit atime. Claude checks the two observed metadata/inventory roots;
Codex checks its fixed source DB/WAL/SHM files. This is not a global atomic snapshot.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess


def record(path, digest=False):
    value = path.lstat()
    result = [value.st_mode, value.st_size, value.st_mtime_ns, value.st_ctime_ns,
              value.st_ino, value.st_dev, value.st_nlink]
    if digest and stat.S_ISREG(value.st_mode):
        if value.st_size > 64 * 1024 * 1024:
            raise ValueError("fingerprint byte limit exceeded")
        hasher = hashlib.sha256()
        with path.open("rb") as source:
            for block in iter(lambda: source.read(1024 * 1024), b""):
                hasher.update(block)
        if path.lstat() != value:
            # atime can change; compare only the explicit metadata tuple below.
            after = path.lstat()
            if result != [after.st_mode, after.st_size, after.st_mtime_ns,
                          after.st_ctime_ns, after.st_ino, after.st_dev, after.st_nlink]:
                raise ValueError("file changed while fingerprinted")
        result.append(hasher.hexdigest())
    return result


def fingerprint(args):
    result = {}
    if args.provider == "codex":
        for name in ["state_5.sqlite", "state_5.sqlite-wal", "state_5.sqlite-shm"]:
            path = args.state_root / name
            result[name] = record(path, True) if path.exists() else None
        return result
    roots = [args.state_root / "projects",
             args.desktop_root / "claude-code-sessions"]
    for index, root in enumerate(roots):
        if not root.exists() or root.is_symlink():
            result[str(index)] = None
            continue
        result[str(index)] = record(root)
        for directory, dirs, files in os.walk(root, followlinks=False):
            dirs.sort()
            files.sort()
            for name in dirs + files:
                path = Path(directory) / name
                if len(result) >= 100_000:
                    raise ValueError("fingerprint entry limit exceeded")
                # Full bytes only for the small metadata files the adapter reads.
                is_sidecar = index == 1 and name.startswith("local_") and name.endswith(".json")
                result[f"{index}/{path.relative_to(root)}"] = record(path, is_sidecar)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allow-real", action="store_true", required=True)
    parser.add_argument("--provider", choices=["codex", "claude"], required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--state-root", type=Path, required=True)
    parser.add_argument("--executable", type=Path)
    parser.add_argument("--desktop-root", type=Path)
    parser.add_argument("--app-bundle", type=Path)
    args = parser.parse_args()
    if args.provider == "claude" and (args.desktop_root is None or args.app_bundle is None):
        parser.error("Claude verification requires explicit desktop and app-bundle roots")
    for path in [args.binary, args.state_root, args.executable, args.desktop_root, args.app_bundle]:
        if path is not None and not path.is_absolute():
            parser.error("all paths must be absolute")
    command = [str(args.binary), "--json", "--provider", args.provider,
               f"--{args.provider}-root", str(args.state_root)]
    if args.executable is not None:
        command += [f"--{args.provider}-executable", str(args.executable)]
    if args.desktop_root is not None:
        command += ["--claude-desktop-root", str(args.desktop_root),
                    "--claude-app-bundle", str(args.app_bundle)]

    def query(operation):
        process = subprocess.run(command + operation, capture_output=True, timeout=30)
        if process.returncode not in [0, 2] or not process.stdout:
            raise ValueError("query failed to return a structured report")
        return process.returncode, json.loads(process.stdout)["report"]

    before = fingerprint(args)
    providers_exit, providers = query(["providers"])
    scan_exit, scan = query(["scan"])
    if scan["failures"] or len(scan["snapshots"]) != 1:
        raise ValueError("discovery returned a structured provider failure")
    snapshot = scan["snapshots"][0]
    inspected = False
    if snapshot["sessions"]:
        _, inspection = query(["inspect", snapshot["sessions"][0]["id"]])
        inspected = inspection["entity"]["kind"] == "session"
    after = fingerprint(args)
    changed = sum(before.get(key) != after.get(key) for key in before.keys() | after.keys())
    provider = providers["providers"][0]
    print(json.dumps({
        "provider": args.provider,
        "providers_exit": providers_exit,
        "scan_exit": scan_exit,
        "presence": provider["presence"],
        "version": provider["fingerprint"]["executable_version"],
        "installations": [{"surface": item["surface"], "version": item["version"]}
                          for item in provider["installations"]],
        "compatibility": provider["compatibility"]["status"],
        "evidence": snapshot["evidence_status"],
        "completeness": snapshot["completeness"],
        "workspaces": len(snapshot["workspaces"]), "projects": len(snapshot["projects"]),
        "sessions": len(snapshot["sessions"]), "artifacts": len(snapshot["artifacts"]),
        "references": len(snapshot["references"]), "inspect_succeeded": inspected,
        "diagnostics": sorted({item["code"] for item in snapshot["diagnostics"]}),
        "coverage": snapshot["coverage"],
        "fingerprinted_entries": len(before), "changed_entries": changed,
        "fingerprints_match": before == after, "atime_excluded": True,
    }, indent=2))


if __name__ == "__main__":
    main()
