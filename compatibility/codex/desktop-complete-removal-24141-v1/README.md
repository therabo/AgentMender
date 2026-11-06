# Codex Desktop build 24141 candidate evidence

This Darwin arm64 candidate has identity `codex-desktop-complete-removal-24141-v1`
and admission `DesktopUnaccepted`. It uses exact build 24141 at
`/Applications/ChatGPT.app` (`com.openai.codex`, version 26.1027.61615).
The recovered build 23880 keeps its own
[catalog evidence](../desktop-complete-removal-v1/catalog/README.md) and candidate
identity. The compiled Desktop admission list is empty.

On 2026-11-06, `codesign --verify --deep --strict` succeeded for both signed
bundles under an Apple generic anchor, bundle identifier `com.openai.codex` and
Developer ID team `2DC432GLL2`. The candidate checks the following component
sizes and SHA-256 commitments using read-only streaming. This check does not
establish that an entire bundle stayed coherent during a concurrent update.

| Component | Bytes | SHA-256 |
| --- | ---: | --- |
| `Contents/Info.plist` | 20,787 | `fb72c75a6cabb64f05420aef782b2d6ac86269a6c5557a75dabfa796905ee6ef` |
| `Contents/MacOS/ChatGPT` | 71,008 | `b3783838c86c3124d4b0fd5fa5c2d2fca327b4b4f52c7c7c369df25cfa879509` |
| `Contents/Resources/app.asar` | 674,615,301 | `bb88615052255e79b5977885f9a0cc212c268589587c79d4d3b7c4b46e881a95` |
| `Contents/Resources/codex-cli/bin/codex` launcher | 328 | `50ab38ba21d0d9f8346f32f41848382f15b556190f3c7a07e885a4fb73e379c8` |
| `Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex` | 267,754,128 | `58a91f6ea19bea477ae3a2ec48d619f0039afe75d3476260747e11d201fa7cea` |
| Separate stable Codex 0.166.0 worker | 268,972,064 | `39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1` |

The embedded CLI is 0.166.0-alpha.5.1. The embedded executable and its shell
launcher match build 23880 byte for byte. Using the stable worker with either
exact Desktop build requires its own runtime evidence.

All 27 ASAR `.vite/build` members match the recovered bundle byte for byte.
The renderer package changed. An owned scratch copy of the historical
[extractor](../desktop-complete-removal-v1/catalog/extract_catalog_evidence.py)
used the exact 24141 ASAR digest above, streaming SHA-256 and a fresh output
directory. The canonical schema, SQL, native ordered migrations, catalog class
and removal/checkpoint/suppression windows reproduced the 23880 artifacts byte
for byte. The migration asset is
`480cf9c01ab1794c1b788aaf1bc159e96972f341e7f7ec03a53a8769e7b0d175`.
The catalog/scan asset is
`c23f63f8ff156b2a92a81ee2870d80ab959ced6ce6232d6fe7e54bbefe11d333`.
The derived schema has 14 tables, 36 migrations and `user_version=34`. The
canonical schema digest is
`a275a36b59fa4895e0793eb1011d7c2b2b7ee25691c2f2bf206ff2f4342cc40a`
and the runtime ledger pin is
`5e56f348093c7c1f056a0f2afbd55d03681efa242a5ae63ad3dc3e0a6bedac45`.

Reproduce the candidate bundle check with the opt-in Rust test. Supply an owned
canonical copy of the recovered build. The test reads bundle files only.

```sh
AGENTMENDER_RECOVERED_DESKTOP_BUNDLE=/absolute/owned/path/ChatGPT.app \
  cargo test --locked -p agentmender-provider-codex --lib \
  desktop_cleanup::tests::exact_installed_and_recovered_bundle_pins -- --exact --ignored
```

Static compatibility supports reusing the typed catalog candidate on 24141.
The protected primary and catalog workers returned a schema-4 candidate on
owned closed SQLite fixtures. The observed writer prerequisite blocked the plan
before any operations were available.
Effective roots, coherent runtime identity, historical schema paths, renderer and
embedded-worker behavior, writer exclusion and both isolated graphical cycles
remain separate prerequisites. See the [admission checkpoint](../../../docs/milestones/m8-2-1-desktop-admission-2026-11-06.md).
