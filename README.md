# Markdown Image to HTML

Transform a Markdown Image to HTML. This allows us to fix image size in Markdown base files.

## Installable native Rust extension for Tezbar

The root [extension.json](extension.json) describes the standalone Rust process package,
version **0.2.0**, ID `com.tezbar.native.markdown-image-to-html`, replacing
`raycast.markdown-image-to-html`. The original Raycast TypeScript source, manifest,
command names, arguments, preferences, icon and licenses remain in this repository.
The mature [Native V2 engine and its original tests](v2/README.md) are reused as an
owned path dependency. Production has no Tezbar/Tauri dependency, JavaScript/Bun
subprocess, frozen-host fixture, or sibling-repository requirement.

| Command | Mode | macOS arm64 / Windows x64 behavior |
|---|---|---|
| `index` — Markdown Image to HTML | no-view | Find Markdown image text in arguments, selected text, then clipboard; replace all image matches and paste the resulting HTML. |

`markdownImage` takes precedence over the historical source typo `mardownImage`
when present, even if empty or invalid. `imageWidth` argument uses the original
JavaScript numeric-string check; otherwise command preference `imageWidth` is used
(default `"300"`). Width is not restricted to positive integers and source HTML is
not sanitized. JavaScript replacement dollar tokens, Unicode, surrounding text,
and real line breaks are preserved. Selection errors fall back to clipboard;
clipboard errors and missing matches report failure without paste.

Host preference resolution remains authoritative. The command retains its exact
preference name/type/default and receives the host's preference snapshot. The
original command uses no LocalStorage or Cache and the v2 engine owns no stored
data. No extension storage migration paths or broad profile reads are needed.

The only host grants are `selection.read`, `clipboard.read`, `clipboard.pasteText`.
A paste is successful only after an object acknowledgement with `ok:true`, no
non-null `error`, and `success` absent or true. A lost, failed or malformed
acknowledgement reports that paste may already have happened. Refresh, dispose,
and rejected actions never retry it. Retry requires a deliberate new command
launch and can repeat an unacknowledged paste. There are no rendered actions or
forms; forged action IDs and unsupported operations reject before effects.

The SDK at immutable revision `aaec571673ab64a85a136cf6bd054ef660c4d93c`
handles protocol-1 JSONL hello, initialize, sessions, cancellation, transactional
state and shutdown. Expected domain failures are protocol successes containing
`{ok:false,mode:"no-view",message:...}`. This keeps the session available for the
host's mandatory no-view disposal and exposes truthful launcher errors.

Input strings/chosen width are capped at 4 MiB UTF-8, conversion output at 16 MiB,
and conservative replacement work at 64 MiB. The SDK additionally caps encoded
JSON frames at 16 MiB; escaping or output expansion can hit that limit earlier.
An oversized reverse paste frame is rejected before sending any paste request.

## Build and package

Rust **1.85** is the minimum toolchain. Cargo.lock and the MSRV-aware resolver
configuration are committed. Keep build outputs inside this checkout and use two
Cargo workers:

```sh
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="$PWD/target"
cargo +1.85.0 test --workspace --all-targets --locked -j2
cargo +1.85.0 clippy --workspace --all-targets --locked -j2 -- -D warnings
cargo +1.85.0 fmt --all -- --check
node v2/scripts/oracle.mjs
cargo +1.85.0 build --release --locked --target aarch64-apple-darwin -j2
python3 scripts/vendor-licenses.py --check
python3 scripts/package.py --target aarch64-apple-darwin \
  --binary target/aarch64-apple-darwin/release/tezbar-markdown-image-to-html
python3 scripts/smoke-package.py dist/markdown-image-to-html-aarch64-apple-darwin.zip
```

For source and archive schema checks (Python 3.9+):

```sh
python3 -m venv .validation/venv
.validation/venv/bin/python -m pip install jsonschema==4.25.1
.validation/venv/bin/python scripts/validate.py
.validation/venv/bin/python v2/scripts/validate.py
```

The root schema snapshot is from accepted Tezbar host commit
`d77f03b12a93d72cd2a56a73faa1a04fae94a2cd`; see [NOTICE](NOTICE).
`protocol-smoke.py <binary>` runs the actual executable through a narrow recording
host transport, including all 406 original oracle cases. `smoke-package.py`
checks the complete SHA inventory and licenses, extracts the ZIP, and exercises
that binary with a minimal PATH and private data/temp paths. Tests never touch
the user's desktop, clipboard, selected text or real Tezbar profile.

ZIP layout: `extension.json`, `bin/tezbar-markdown-image-to-html[.exe]`,
`resources/assets/command-icon.png`, `LICENSE`, `NOTICE`, `licenses/**`.
The package manifest inventories every file except itself. Packaging writes
archive and executable `.sha256` files, `.artifact.json` and `.files.json`
sidecars. It verifies binary identity/architecture and the original runtime/build
dependency license/NOTICE files for **both** supported targets; missing/stale
notices block packaging. To refresh after changing Cargo.lock, run
`python3 scripts/vendor-licenses.py` and review the resulting documents.

Windows packaging must run on Windows with a real x64 MSVC binary; use the same
scripts with target `x86_64-pc-windows-msvc` and its `.exe`. Dependency metadata and
licenses for Windows do not establish a successful Windows build or execution.

## Installation and release gates

A reviewed target ZIP is installed by Tezbar's schema-3 package installer; no
Cargo/npm installation runs on the user's machine. The published trusted catalog
or trusted local-import manifest must pin the real archive/executable hashes and
artifact URL. The source/archive `artifacts:{}` is intentional before publication;
sidecars alone are not a trusted catalog entry. After independent review, central
Windows build/checks and approved artifact metadata, installation uses the normal
catalog or `tezbar --extensions-v3-import <trusted-release-manifest> <package.zip>`.
The accepted baseline catalog has no entry for this extension yet. Remote installation
and legacy alias routing require approved catalog metadata; canonical local import
uses the selected trusted manifest. No release, tag, workflow build or publication
is performed by these scripts.

Real desktop QA remains **unverified** on macOS and Windows: selected text,
clipboard permissions, focus restoration and paste delivery require actual host
installation checks. The manifest's verification notes record that limit.

Accepted host `d77f03b` supports all three required typed host APIs, no-view result
annotation and session disposal. Its launcher displays the top-level message, but
its hotkey callback ignores no-view success/error feedback. **Hotkey feedback is
an explicit remaining host gap and release gate** until an independently accepted
host fix is available; this package assumes only the accepted baseline above. No new host API is required. No host version number is invented to
pretend that this renderer gap is fixed. The command's real desktop effects and
full installation remain gated on the checks above.
