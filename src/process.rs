use markdown_image_to_html_native_v2::{run, Host};
use serde_json::{json, Value};
use tezbar_extension_sdk::{BoxFuture, CoreResult, Extension, ExtensionContext};

pub struct Adapter;
struct ProcessHost<'a>(&'a ExtensionContext);
impl Host for ProcessHost<'_> {
    async fn request(&self, channel: &str, args: Vec<Value>) -> CoreResult {
        match (channel, args.as_slice()) {
            ("core:platform:selected-text", []) => {
                self.0.host("host.selection.read", json!({})).await
            }
            ("clipboard:read", []) => self.0.host("host.clipboard.read", json!({})).await,
            ("core:platform:paste-text", [Value::String(text)]) => {
                self.0
                    .host("host.clipboard.pasteText", json!({"text": text}))
                    .await
            }
            _ => Err("Unsupported Markdown Image to HTML host request".into()),
        }
    }
}
impl Extension for Adapter {
    fn id(&self) -> &'static str {
        "com.tezbar.native.markdown-image-to-html"
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn commands(&self) -> &'static [&'static str] {
        &["index"]
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        command: &'a str,
        operation: &'a str,
        payload: &'a Value,
        state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            if command != "index" {
                return Err("Unknown Markdown Image to HTML command".into());
            }
            match operation {
                "extension:run-command" => {
                    // One attempt per session, including uncertain acknowledgements. Only a
                    // fresh, deliberate command invocation can paste again. The SDK stages
                    // this result and commits only for the owning non-cancelled operation.
                    if let Some(result) = state.get("result") {
                        return Ok(result.clone());
                    }
                    let result = match run(&ProcessHost(ctx), payload).await {
                        Ok(()) => {
                            json!({"ok":true,"mode":"no-view","message":"Pasted HTML image text"})
                        }
                        // Domain failures are successful protocol responses so the actual
                        // caller can dispose its no-view session and show a truthful error.
                        Err(message) => json!({"ok":false,"mode":"no-view","message":message}),
                    };
                    *state = json!({"result":result});
                    Ok(result)
                }
                "extension:dispose-session" | "extension:refresh-session" => {
                    Ok(tezbar_extension_sdk::unchanged())
                }
                // This command has no rendered or global actions. Never accept a forged
                // invoke-action or turn a refresh into an implicit retry of paste.
                _ => Err("Unsupported Markdown Image to HTML operation".into()),
            }
        })
    }
}
