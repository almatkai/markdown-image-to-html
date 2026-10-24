//! Compile unchanged inside native_core::extensions (see INTEGRATION.md).
use super::v2_sdk;
use crate::native_core::common::{CoreContext, CoreResult};
use markdown_image_to_html_native_v2::{run, Host};
use serde_json::Value;

pub fn commands() -> &'static [&'static str] {
    &["index"]
}
struct NativeHost<'a>(&'a CoreContext);
impl Host for NativeHost<'_> {
    async fn request(&self, channel: &str, args: Vec<Value>) -> CoreResult {
        self.0.request(channel, args).await
    }
}
pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    _state: &mut Value,
) -> CoreResult {
    if !commands().contains(&command) {
        return Err("Unknown Markdown Image to HTML command".into());
    }
    match operation {
        "extension:run-command" => {
            run(&NativeHost(ctx), payload).await?;
            Ok(v2_sdk::no_view("Pasted HTML image text"))
        }
        "extension:dispose-session" | "extension:refresh-session" => Ok(v2_sdk::unchanged()),
        _ => Err("Unsupported Markdown Image to HTML operation".into()),
    }
}
