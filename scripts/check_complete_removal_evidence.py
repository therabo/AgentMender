#!/usr/bin/env python3
"""Validate static M8.2.1 research; never start a provider or inspect user state."""
import argparse
import copy
import hashlib
import json
import pathlib
import plistlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "compatibility/codex/complete-removal-research"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON field: " + key)
        result[key] = value
    return result


def read(name):
    return json.loads((EVIDENCE / name).read_text(), object_pairs_hook=unique_object)


def sha(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value)


def validate(inventory, provenance, schema):
    require(inventory["schema_version"] == provenance["schema_version"] == schema["schema_version"] == 1,
            "unreviewed research version")
    require(inventory["verdict"] == "OPEN" and not inventory["provider_impossibility_proven"],
            "static evidence cannot establish PASS or provider impossibility")
    require(inventory["graphical_profile_enabled"] is False
            and inventory["restart_reappearance_tested"] is False
            and inventory["admitted_complete_removal_profiles"] == [],
            "static inventory must not enable a graphical capability")
    required = {"identity", "provider_component", "storage", "classification", "session_reference",
                "user_visible_effect", "proposed_mutation", "refresh_behavior", "restart_behavior",
                "verification", "status", "evidence", "persistence", "authority", "reconstruction_risk"}
    identities = set()
    require(inventory["surfaces"], "empty projection inventory")
    for surface in inventory["surfaces"]:
        require(required <= surface.keys(), "missing surface semantics")
        require(all(surface[key] for key in required), "empty surface semantics")
        require(surface["identity"] not in identities, "duplicate projection identity")
        identities.add(surface["identity"])
        require(surface["classification"] in {"RequiredProjection", "DerivedProjection", "SharedProjection", "UnknownProjection"},
                "no researched surface was established NotRelevant")
        require(surface["persistence"] in {"persistent", "in_memory", "persistent_and_in_memory"},
                "missing persistence role")
        require(surface["reconstruction_risk"] == "unclosed_for_complete_removal",
                "static investigation is not reconstruction closure")
    require({"desktop-thread-catalog", "desktop-global-state-and-backup", "desktop-pinned-and-custom-sections",
             "desktop-recents", "desktop-title-and-content-search", "desktop-archived-all",
             "desktop-project-workspace-lists", "desktop-renderer-atoms-and-caches",
             "desktop-other-thread-records", "electron-userData-browser-storage",
             "loaded-transcript-runtime", "remote-cloud-hosts", "title-index", "command-history",
             "canonical-active-and-archived-lists", "desktop-catalog-scan-records",
             "desktop-automation-inbox-history"} == identities,
            "investigated surface omitted or unreviewed surface added")
    desktop = provenance["desktop"]
    require((desktop["bundle_id"], desktop["version"], desktop["build"])
            == ("com.openai.codex", "26.1027.51404", "23880"), "different Desktop candidate")
    require(desktop["embedded_cli_package"]["version"] == "0.166.0-alpha.5.1",
            "embedded alpha must not be relabeled stable")
    require(sha(desktop["app_asar_sha256"]), "invalid archive hash")
    assets = desktop["assets"]
    require(len({asset["path"] for asset in assets}) == len(assets), "duplicate asset")
    for asset in assets:
        require(sha(asset["sha256"]) and asset["bytes"] > 0, "invalid static asset commitment")
    for binary in desktop["binaries"]:
        require(sha(binary["sha256"]), "invalid binary hash")
    require(provenance["official_source"]["commit"] == "6ff1e1ca3b30fc8b914720681fcf5fe62e95f856",
            "different official source revision")
    require(all(sha(item["sha256"]) for item in provenance["official_source"]["files"]),
            "invalid source commitment")
    require(provenance["safety"]["provider_started"] is False
            and provenance["safety"]["real_user_state_read"] is False
            and provenance["safety"]["real_user_state_mutated"] is False
            and provenance["safety"]["app_restarted"] is False,
            "this provenance is static only")
    require(any(asset["path"] == schema["source_asset"] and asset["sha256"] == schema["source_sha256"] for asset in assets),
            "schema source is not an exact committed asset")
    require(schema["catalog_primary_key"] == ["host_id", "thread_id"],
            "bare thread UUID is not Desktop catalog identity")
    require("NOT runtime-captured" in schema["evidence_class"] and schema["scope_warning"],
            "derived schema must retain its scope limitation")
    columns = schema["catalog_current_columns_derived"]
    require(len({column["name"] for column in columns}) == len(columns), "duplicate derived column")
    require([column["name"] for column in columns if column["pk_ordinal"]] == ["host_id", "thread_id"],
            "derived primary key mismatch")


def canaries(inventory, provenance, schema):
    for mutation in (lambda d: d.update(graphical_profile_enabled=True),
                     lambda d: d.update(restart_reappearance_tested=True),
                     lambda d: d["surfaces"].pop(),
                     lambda d: d["surfaces"].append(copy.deepcopy(d["surfaces"][0])),
                     lambda d: d["surfaces"][0].update(classification="NotRelevant"),
                     lambda d: d["surfaces"][0].pop("restart_behavior")):
        changed = copy.deepcopy(inventory)
        mutation(changed)
        try:
            validate(changed, provenance, schema)
        except (ValueError, KeyError):
            pass
        else:
            raise ValueError("evidence rejection canary failed")
    try:
        json.loads('{"scope":1,"scope":2}', object_pairs_hook=unique_object)
    except ValueError:
        pass
    else:
        raise ValueError("duplicate-field canary failed")


def verify_installed(bundle, provenance):
    require(bundle.is_absolute(), "bundle path must be absolute")
    desktop = provenance["desktop"]
    with (bundle / "Contents/Info.plist").open("rb") as handle:
        info = plistlib.load(handle)
    require((info["CFBundleIdentifier"], info["CFBundleShortVersionString"], str(info["CFBundleVersion"]))
            == (desktop["bundle_id"], desktop["version"], desktop["build"]), "installed bundle changed")
    targets = [(bundle / "Contents/Resources/app.asar", desktop["app_asar_sha256"])]
    for binary in desktop["binaries"]:
        relative = pathlib.Path(binary["path"]).relative_to(desktop["bundle"])
        targets.append((bundle / relative, binary["sha256"]))
    for path, expected in targets:
        with path.open("rb") as handle:
            actual = hashlib.file_digest(handle, "sha256").hexdigest()
        require(actual == expected, "static installed identity changed: " + str(path))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--installed-bundle", type=pathlib.Path)
    args = parser.parse_args()
    inventory = read("projection-inventory.json")
    provenance = read("projection-provenance.json")
    schema = read("desktop-catalog-schema-static.json")
    validate(inventory, provenance, schema)
    canaries(inventory, provenance, schema)
    isolation = read("isolation-result.json")
    require(isolation["verdict"] == "OPEN" and isolation["app_startup_attempt"] is True
            and isolation["app_ready"] is False and isolation["structured_gui_observation"] is False
            and isolation["native_cli_started"] is False
            and isolation["real_user_state_read"] is False
            and isolation["real_user_state_mutated"] is False
            and isolation["real_app_restarted"] is False,
            "startup feasibility must not be promoted to GUI acceptance")
    require(isolation["result"]["exit_code"] == -5
            and isolation["result"]["failure"] is None
            and isolation["parent_child_canary"]["exit_code"] == 0,
            "isolation result changed without reviewed evidence")
    if args.installed_bundle:
        verify_installed(args.installed_bundle, provenance)
    print(f"Complete-removal research checks passed: {len(inventory['surfaces'])} surfaces; static-only evidence; zero admitted graphical profiles; verdict OPEN.")


if __name__ == "__main__":
    main()
