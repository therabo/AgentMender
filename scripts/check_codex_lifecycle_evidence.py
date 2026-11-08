#!/usr/bin/env python3
"""Deterministic evidence checks; no provider, network, or user-state access."""
import hashlib
import json
from pathlib import Path
import uuid

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "compatibility/codex/0.166.0/lifecycle-m8-1"


def read(name):
    return json.loads((EVIDENCE / name).read_text())


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, description):
    if not condition:
        raise SystemExit("Codex evidence guard failed: " + description)


def main():
    manifest = read("evidence-manifest.json")
    actual = {p.relative_to(EVIDENCE).as_posix() for p in EVIDENCE.rglob("*") if p.is_file()}
    require(actual == set(manifest) | {"evidence-manifest.json"}, "exact committed evidence file set")
    for name, expected in manifest.items():
        require(not Path(name).is_absolute() and ".." not in Path(name).parts, "bounded manifest path")
        require(sha(EVIDENCE / name) == expected, "committed artifact SHA-256: " + name)

    profile = read("profile.json")
    require(profile["version"] == "0.166.0" and not profile["production_profile_registered"], "research-only exact version")
    require(profile["binary_sha256"] == "39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1", "exact acquired binary")
    require(profile["execution"]["archive"] == profile["execution"]["delete"] == "Unsupported", "no capability promotion")
    require(profile["compensation"] == "NotCompensatable", "no inferred inverse")
    for name in ["complete_predispatch_effect_boundary", "authoritative_complete_deletion_proof",
                 "complete_retained_verification", "read_only_complete_reconciliation"]:
        require(profile[name] is False, "missing proof must remain visible: " + name)

    selected = read("selected-schema-manifest.json")
    groups = {"v1": ["InitializeParams", "InitializeResponse"],
              "v2": [f"Thread{method}{part}"
                     for method in ["List", "Read", "Archive", "Unarchive", "Delete"]
                     for part in ["Params", "Response"]]}
    expected_paths = {f"schemas/{mode}/{version}/{name}.json"
                      for mode in ["stable", "experimental"]
                      for version, names in groups.items() for name in names}
    require(len(selected) == 24 and {x["path"] for x in selected} == expected_paths, "24 exact selected schemas")
    full = read("schema-manifest.json")
    require(len(full["stable"]["files"]) == 321 and len(full["experimental"]["files"]) == 467, "separate full generated sets")
    for mode, details in full.items():
        encoded = json.dumps(details["files"], sort_keys=True, separators=(",", ":")).encode()
        require(hashlib.sha256(encoded).hexdigest() == profile["schema_fingerprints"][mode], "full schema-map fingerprint")
    for entry in selected:
        _, mode, *parts = entry["path"].split("/")
        require(sha(EVIDENCE / entry["path"]) == entry["sha256"] == full[mode]["files"]["/".join(parts)], "selected/full/schema hash agreement")
    require(sha(EVIDENCE / "selected-schema-manifest.json") == profile["selected_schema_manifest_sha256"], "selected manifest commitment")
    schema = read("state-schema-5.json")
    old_schema = json.loads((ROOT / "compatibility/codex/0.165.0-alpha.1.2/state-schema-5.json").read_text())
    require(len(schema) == 66 and schema == old_schema, "storage equality is evidence, not authority")
    require(hashlib.sha256(json.dumps(schema, separators=(",", ":")).encode()).hexdigest() ==
            "1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647", "canonical state layout")

    archive = {c["case"]: c for c in read("archive-evidence.json")["cases"]}
    hidden = archive["hidden-tree"]
    child = hidden["ids"]["hidden_child"]
    require(child not in hidden["beforeListings"]["False-True"]["ids"] and
            child in hidden["beforeRelations"]["False"]["ids"], "positive empty-preview relation improvement")
    attached = archive["attached-namespace"]
    child = attached["ids"]["spawned_attached_child"]
    require(child not in attached["ancestor"]["ids"] and child in attached["attached"]["ids"] and
            attached["after_archive"][child]["state"] == "archived", "finite union recovery without closure promotion")
    partial = archive["archive-denied-child"]
    require(partial["archive"]["kind"] == "accepted" and
            partial["after_archive_files"][partial["ids"]["hidden_child"]]["state"] == "active" and
            partial["after_archive_files"][partial["ids"]["grandchild"]]["state"] == "archived", "accepted partial archive is not complete success")
    mixed = archive["mixed-tree"]
    require(mixed["before"][mixed["ids"]["hidden_child"]]["state"] == "archived" and
            mixed["after_unarchive_files"][mixed["ids"]["hidden_child"]]["state"] == "active", "unarchive is not equivalent compensation")

    deleted = read("delete-evidence.json")
    partial = next(c for c in deleted["cases"] if c["case"] == "root-rollout-denied")
    require(partial["delete"]["rpc_error_code"] == -32603, "partial delete error retained")
    child, missing = partial["child"], partial["never_seeded"]
    for observations in [partial["after"], partial["after_restart"]]:
        require(observations[str((child, False))]["metadata_present"], "stale metadata is positively visible")
        require(observations[str((child, True))]["error_category"] ==
                observations[str((missing, True))]["error_category"] == "thread_not_loaded", "history miss is not entity absence")
    for case in deleted["cases"]:
        for record in case["retained_rollout_hashes"]:
            require(record["unchanged"] and record["before_sha256"] == record["after_sha256"], "finite retained rollout measurements")
    identity = deleted["uuid_identity"]
    for raw, canonical in identity["native_success_aliases"].items():
        require(str(uuid.UUID(raw)) == canonical, "full 128-bit identity preserved separately from raw spelling")
    require(all(x != x.strip() or x in ["not-uuid", "ab000000-1111-411198111-000000000001"]
                for x in identity["native_rejected_aliases"]), "malformed/whitespace claims are not normalized")
    for name in ["archive-evidence.json", "delete-evidence.json", "reproducer-evidence.json"]:
        text = (EVIDENCE / name).read_text()
        require(not any(marker in text for marker in ["/Users/", "/private/tmp/", '"serverName"', '"installationId"']), "sanitized native evidence")
    reproduced = read("reproducer-evidence.json")
    require(len(reproduced["cases"]) == 7 and not reproduced["production_compatibility_promoted"], "independent opt-in replay")
    require(all(value == "DENIED" for value in reproduced["isolation_canaries"].values()), "kernel isolation canaries")
    require(read("reproducer-provenance.json")["script_sha256"] == sha(ROOT / "scripts/probe_codex_current_lifecycle.py"), "reproducer code identity")
    print("Codex 0.166.0 evidence guards passed: 24 schemas, exact pins, native counterexamples; no production promotion.")


def check_adaptive_read_evidence():
    directory = ROOT / "compatibility/codex/0.166.0-alpha.6/real-world-readonly"
    observed = json.loads((directory / "evidence.json").read_text())
    require(observed["executable_version"] == "0.166.0-alpha.6", "current read evidence version")
    require(not observed["binary_hash_is_read_admission_requirement"], "observed hash is not read admission")
    for field in ["native_delete_admitted", "scoped_mutation_profile_added", "schema4_admission_changed", "actual_model_tool_invocation", "human_approved_real_deletion"]:
        require(observed[field] is False, "unaccepted real-world gate stays explicit: " + field)
    require(observed["real_metadata_reader"] == "codex-state-threads-columns-v1" and observed["real_metadata_completeness"] == "partial", "bounded read contract")
    require(observed["real_source_file_hashes_equal"] and observed["source_worker_kernel_attested"], "real protected observation")
    require(sha(directory / "native-counterexamples.json") == observed["native_counterexamples_sha256"], "current native evidence integrity")
    for name, digest in observed["selected_schemas"].items():
        require(not Path(name).is_absolute() and ".." not in Path(name).parts and sha(directory / name) == digest, "current selected schema integrity")
    native = json.loads((directory / "native-counterexamples.json").read_text())
    require(native["binary_sha256"] == observed["observed_executable_sha256"] and not native["production_compatibility_promoted"], "native interface evidence grants no authority")
    cases = {case["case"]: case for case in native["cases"]}
    partial = cases["delete-denied-root"]
    require("error_code" in partial["delete"] and partial["before"] != partial["after"], "current native partial effects before error")
    require(not partial["authoritative_absence_proof"] and cases["delete-mixed-tree"]["retained_fixture_rollouts_unchanged"], "current native proof limits")
    require(all(v == "DENIED" for v in native["isolation_canaries"].values()), "current native probes remain isolated")
    print("Adaptive read evidence and current native non-admission checks passed.")


if __name__ == "__main__":
    main()
    check_adaptive_read_evidence()
