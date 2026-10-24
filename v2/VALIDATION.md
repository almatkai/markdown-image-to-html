# Validation

Run from this Cargo package with Rust and Node >=22.13 installed. All Rust tests use mocked routes; no selected-text, clipboard, focus or paste OS actions run. The exact beta common/SDK fixtures also retain their own isolated temporary-file tests. Keep test temporary directories local if desired:

```sh
mkdir -p .validation/tmp
export TMPDIR="$PWD/.validation/tmp"
cargo test --locked --all-targets
cargo +1.85.0 test --locked --all-targets
cargo fmt --all -- --check
cargo +1.85.0 fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo +1.85.0 clippy --locked --all-targets -- -D warnings
cargo check --locked --all-targets --target x86_64-pc-windows-gnu
cargo clippy --locked --all-targets --target x86_64-pc-windows-gnu -- -D warnings
cargo +1.85.0 check --locked --all-targets --target x86_64-pc-windows-gnu
cargo +1.85.0 clippy --locked --all-targets --target x86_64-pc-windows-gnu -- -D warnings
node scripts/oracle.mjs
```

The Windows GNU target must be installed in both toolchains. Check/clippy compile without linking or running a Windows binary. The lockfile resolves on Rust 1.85; runtime dependencies are only regex, serde_json and ryu-js.

`node scripts/oracle.mjs` executes the actual pinned TypeScript function with type erasure and API mocks, checks its stored snapshot, and compares the production Rust adapter's captured RPC calls and output for **406 cases**. The only behavioral wrapper is the documented canonical-to-legacy argument adaptation. Cases exercise selection/clipboard precedence and failures, no reads for valid arguments, canonical/typo precedence, actual string-valued numeric coercion, preference conversion and replacement dollar tokens, all image replacements, greedy nested parentheses, lazy alt, all JavaScript whitespace characters, line terminators, Unicode, surrounding text and deterministic adversarial grammar. It never generates expected output through a second implementation of the algorithm. `--write` deliberately regenerates the snapshot and still compares the live Rust probe.

The Rust contract suite includes the snapshot and separate malformed JSON argument/metadata checks, lifecycle/command validation, selected-text and clipboard transport/error envelopes, strict paste acknowledgements (including `ok: true` with non-null error or false success), no retry/duplicate paste, no-match errors, UTF-8 matching and input/output/work bounds. The exact beta common/SDK's five existing tests also run. The test probe binary is an example/dev target and is not used by the production adapter.

Full schema validation needs Python's `jsonschema` package (validated with 4.25.1). It checks the complete pinned Draft 2020-12 schema, exact source command/argument/preference metadata, credits, not-run verification, all copied reference hashes, icon size/hash, aggregate license and Git publication visibility:

```sh
python3 -m venv .validation/schema-env
.validation/schema-env/bin/python -m pip install jsonschema==4.25.1
.validation/schema-env/bin/python scripts/validate.py
```

Package verification must include the fixtures, adapter, schema, scripts, licenses, icon and lockfile:

```sh
cargo package --locked --allow-dirty
mkdir -p .validation/extracted
tar -xzf target/package/markdown-image-to-html-native-v2-0.1.0.crate -C .validation/extracted
cd .validation/extracted/markdown-image-to-html-native-v2-0.1.0
cargo test --locked --all-targets
node scripts/oracle.mjs
```

The extracted package needs no sibling repository, checkout, generated bundle, host profile or external source fixture. The schema validator can run there with the same Python environment.

V1 compatibility checks run in a disposable copy of the pinned V1 tracked files, under an ignored validation directory. With pnpm dependencies installed in that copy, `pnpm test` and `pnpm typecheck` pass. The root V1 source, manifests, assets and package-lock remain byte-exact; its only allowed edit is a README link to this package. No V1 production build or real host integration is needed for these checks.

Recorded environment: macOS ARM64; Rust 1.98.0 and 1.85.0; Node 26.7.0. Current/MSRV native tests, formatting and strict all-targets clippy; both toolchains' Windows GNU all-targets check/clippy; live TS oracle; full schema/provenance/icon checks; Cargo packaging and extracted-package tests/oracle; and disposable V1 pnpm test/typecheck were exercised. Actual macOS/Windows selection, clipboard, focus and paste remain **not-run**, as declared in the manifest. Host installation/rebuild is an integration step, not established by mocks or JSON metadata.
