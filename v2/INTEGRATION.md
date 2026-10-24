# Compiled-in integration

Contract reference: Tezbar beta `fd300e276356587726566026f072563eb9c34cb1`. The adapter compiles unchanged against its exact `CoreContext` and `v2_sdk`; byte-exact reference fixtures and their provenance ship in this package. Runtime JSON cannot load Rust code.

1. Vendor this complete Cargo package at **`src-tauri/native-extensions/markdown-image-to-html`**. Keep the locked dependency graph. In the host's `src-tauri/Cargo.toml`, add:

   ```toml
   markdown-image-to-html-native-v2 = { path = "native-extensions/markdown-image-to-html" }
   ```

2. In `src-tauri/src/native_core/extensions/mod.rs`, declare the production adapter directly from that stable vendor location:

   ```rust
   #[path = "../../../native-extensions/markdown-image-to-html/src/tezbar_adapter.rs"]
   mod v2_markdown_image_to_html;
   ```

   Its sibling `super::v2_sdk` is the host SDK. The standalone library has no host dependency. The test harness uses the same production adapter file, exact beta `common.rs`/`v2_sdk.rs`, and only a router plus a Tauri emitter stub. No simplified SDK replaces the contract.

3. Register `markdown-image-to-html` in all three actual `v2_runtime.rs` dispatch locations:

   - `slug`: include `"markdown-image-to-html"` among recognized values (the existing prefix handling resolves canonical and `raycast.` IDs).
   - `commands`: `"markdown-image-to-html" => super::v2_markdown_image_to_html::commands(),`.
   - `handle`: `"markdown-image-to-html" => super::v2_markdown_image_to_html::handle(ctx, command, operation, payload, state).await,`.

4. Add the complete object from `manifest.json` to `src-tauri/src/native_core/extensions/v2_catalog.json`. Preserve `argumentValues` in launcher requests. The existing native-session code resolves command preferences and places saved values at **`payload.preferences`**; do not flatten command preference metadata or add a separate preference source. Preserve the command's original argument and preference metadata. The manifest deliberately puts author/contributor credits in its description because the pinned public schema does not support a root `author` property.

5. Copy `assets/command-icon.png` byte-for-byte to **`src/renderer/public/extensions/icons/markdown-image-to-html.png`**. Its URL is `/extensions/icons/markdown-image-to-html.png`; size is 63,310 bytes, SHA-256 `9a603d247a45706dbeef887e4eb0b405700a6e32efe0621059c80b2c8c56d118`.

6. Rebuild the host and use its existing native install/catalog path. A manifest, marker, external executable, JSON dispatcher, or dynamic Rust loader alone does not enable execution. Keep the V1 package available for fallback. The compiled adapter reads no legacy files itself and performs no preference or storage migration.

The command `index` has no view, actions, search state, external app, network, or storage. No host profile/settings edits are required. Only successful paste acknowledgement produces a success result; no-view error visibility depends on the host's existing error presentation. Platform verification stays `not-run` until real OS QA records selection, clipboard, focus and paste results independently on macOS and Windows.
