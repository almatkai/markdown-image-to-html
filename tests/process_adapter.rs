#[path = "../src/process.rs"]
mod process;

use process::Adapter;
use serde_json::{json, Value};
use tezbar_extension_sdk::{Extension, ExtensionPaths, TestContext};

fn recording() -> (tempfile::TempDir, TestContext) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = TestContext::new(
        ExtensionPaths {
            package_dir: dir.path().join("package"),
            data_dir: dir.path().join("data"),
            temp_dir: dir.path().join("temp"),
        },
        &["selection.read", "clipboard.read", "clipboard.pasteText"],
    );
    (dir, ctx)
}

#[tokio::test]
async fn completed_or_uncertain_effects_are_never_replayed_in_a_session() {
    for reply in [
        Ok(json!({"ok":true,"message":"Pasted text"})),
        Err("ack lost".into()),
        Ok(Value::Null),
        Ok(json!({"ok":false})),
    ] {
        let (_dir, ctx) = recording();
        ctx.reply(reply);
        let mut state = json!({});
        let payload = json!({"argumentValues":{"markdownImage":"![a](u)"}});
        let result = ctx
            .run(Adapter.handle(
                &ctx.context,
                "index",
                "extension:run-command",
                &payload,
                &mut state,
            ))
            .await
            .unwrap();
        assert_eq!(result["mode"], "no-view");
        assert!(result["message"].as_str().is_some_and(|m| !m.is_empty()));
        let repeat = ctx
            .run(Adapter.handle(
                &ctx.context,
                "index",
                "extension:run-command",
                &payload,
                &mut state,
            ))
            .await
            .unwrap();
        assert_eq!(repeat, result);
        for operation in ["extension:refresh-session", "extension:dispose-session"] {
            assert_eq!(
                Adapter
                    .handle(&ctx.context, "index", operation, &json!({}), &mut state)
                    .await
                    .unwrap(),
                json!({"ok":true,"mode":"unchanged"})
            );
        }
        for operation in [
            "extension:invoke-action",
            "extension:search-text-changed",
            "extension:load-more",
        ] {
            assert!(Adapter
                .handle(
                    &ctx.context,
                    "index",
                    operation,
                    &json!({"actionId":"paste"}),
                    &mut state
                )
                .await
                .is_err());
        }
        let calls = ctx.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["method"], "host.clipboard.pasteText");
        assert_eq!(
            calls[0]["params"],
            json!({"text":"<img width=\"300\" alt=\"a\" src=\"u\"/>"})
        );
    }
}

#[tokio::test]
async fn unknown_command_rejects_before_host_access() {
    let (_dir, ctx) = recording();
    let mut state = json!({});
    assert!(Adapter
        .handle(
            &ctx.context,
            "forged",
            "extension:run-command",
            &json!({}),
            &mut state
        )
        .await
        .is_err());
    assert_eq!(state, json!({}));
    assert!(ctx.calls().is_empty());
}
