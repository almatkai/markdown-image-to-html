include!("support/tauri_stub.rs");
#[path = "support/host.rs"]
mod native_core;
#[path = "support/scenario.rs"]
mod scenario;
use markdown_image_to_html_native_v2::{convert, MAX_INPUT_BYTES, MAX_OUTPUT_BYTES, NO_MATCH};
use serde_json::{json, Value};

#[tokio::test]
async fn captured_pinned_typescript_oracle() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("fixtures/oracle.json")).unwrap();
    for case in cases {
        let actual = scenario::execute(&case).await;
        assert_eq!(
            actual["status"], case["expected"]["status"],
            "{}",
            case["name"]
        );
        assert_eq!(
            actual["calls"], case["expected"]["calls"],
            "{}",
            case["name"]
        );
    }
}
#[tokio::test]
async fn malformed_arguments_fail_before_all_os_calls() {
    for payload in [
        Value::Null,
        json!([]),
        json!({"argumentValues":null}),
        json!({"argumentValues":[]}),
        json!({"preferences":null}),
        json!({"preferences":[]}),
        json!({"argumentValues":{"markdownImage":null}}),
        json!({"argumentValues":{"markdownImage":5}}),
        json!({"argumentValues":{"mardownImage":false}}),
        json!({"argumentValues":{"imageWidth":300}}),
        json!({"argumentValues":{"imageWidth":null}}),
        json!({"argumentValues":{"markdownImage":"![a](u)","imageWidth":[]}}),
    ] {
        let actual = scenario::execute(&json!({"payload":payload})).await;
        assert_eq!(actual["status"], "error", "{payload}");
        assert_eq!(actual["calls"], json!([]), "{payload}");
        assert!(!actual["message"].as_str().unwrap().is_empty());
    }
    // Canonical presence also ignores malformed legacy-only input.
    let actual = scenario::execute(&json!({"payload":{"argumentValues":{
        "markdownImage":"![canonical](u)","mardownImage":42
    }}}))
    .await;
    assert_eq!(actual["status"], "ok");
    assert_eq!(actual["calls"].as_array().unwrap().len(), 1);
}
#[tokio::test]
async fn rejects_commands_and_operations_without_any_effect() {
    for case in [
        json!({"command":"other"}),
        json!({"command":"other","operation":"extension:dispose-session"}),
        json!({"operation":"extension:invoke-action"}),
        json!({"operation":"extension:search-text-changed"}),
        json!({"operation":"extension:load-more"}),
        json!({"operation":"clipboard:read"}),
        json!({"operation":"unknown"}),
    ] {
        let actual = scenario::execute(&case).await;
        assert_eq!(actual["status"], "error");
        assert_eq!(actual["calls"], json!([]));
    }
    for operation in ["extension:dispose-session", "extension:refresh-session"] {
        let actual = scenario::execute(&json!({"operation":operation,"payload":null})).await;
        assert_eq!(actual["result"], json!({"ok":true,"mode":"unchanged"}));
        assert_eq!(actual["calls"], json!([]));
    }
}
#[tokio::test]
async fn selected_text_errors_and_envelopes_fall_back_but_clipboard_errors_surface() {
    for value in [
        Value::Null,
        json!(3),
        json!([]),
        json!({"ok":true,"text":"![fake](u)"}),
        json!({"ok":true,"error":"bad"}),
    ] {
        let actual = scenario::execute(&json!({
            "selection":{"value":value},"clipboard":{"value":"![real](u)"}
        }))
        .await;
        assert_eq!(actual["status"], "ok");
        assert_eq!(
            actual["calls"][0],
            json!(["core:platform:selected-text", []])
        );
        assert_eq!(actual["calls"][1], json!(["clipboard:read", []]));
        assert_eq!(
            actual["calls"][2],
            json!([
                "core:platform:paste-text",
                ["<img width=\"300\" alt=\"real\" src=\"u\"/>"]
            ])
        );
        let bad = scenario::execute(&json!({"clipboard":{"value":value}})).await;
        assert_eq!(bad["status"], "error");
        assert_eq!(bad["calls"].as_array().unwrap().len(), 2);
    }
    let bad =
        scenario::execute(&json!({"selection":{"error":true},"clipboard":{"error":true}})).await;
    assert_eq!(bad["message"], "Clipboard read failed");
    assert_eq!(bad["calls"].as_array().unwrap().len(), 2);
    let none = scenario::execute(&json!({})).await;
    assert_eq!(none["message"], NO_MATCH);
    assert_eq!(none["calls"].as_array().unwrap().len(), 2);
}
#[tokio::test]
async fn paste_acknowledgement_is_strict_and_never_retried() {
    for value in [
        Value::Null,
        json!(true),
        json!("ok"),
        json!([]),
        json!({}),
        json!({"success":true}),
        json!({"ok":false}),
        json!({"ok":"true"}),
        json!({"ok":true,"error":"failure"}),
        json!({"ok":true,"error":false}),
        json!({"ok":true,"error":{}}),
        json!({"ok":true,"success":false}),
        json!({"ok":true,"success":null}),
        json!({"ok":true,"success":1}),
        json!({"ok":true,"success":true,"error":"failure"}),
        json!({"ok":true,"error":{"message":"transport envelope failure"}}),
    ] {
        let actual = scenario::execute(&json!({
            "payload":{"argumentValues":{"markdownImage":"![a](u)"}}, "paste":{"value":value}
        }))
        .await;
        assert_eq!(actual["status"], "error", "{value}");
        assert_eq!(actual["calls"].as_array().unwrap().len(), 1);
        assert!(actual["message"]
            .as_str()
            .unwrap()
            .contains("may already have occurred"));
    }
    for value in [
        json!({"ok":true}),
        json!({"ok":true,"error":null}),
        json!({"ok":true,"success":true,"error":null}),
    ] {
        let actual=scenario::execute(&json!({"payload":{"argumentValues":{"markdownImage":"![a](u)"}},"paste":{"value":value}})).await;
        assert_eq!(
            actual["result"],
            json!({"ok":true,"mode":"no-view","message":"Pasted HTML image text"})
        );
        assert_eq!(actual["calls"].as_array().unwrap().len(), 1);
    }
    let actual = scenario::execute(
        &json!({"payload":{"argumentValues":{"markdownImage":"![a](u)"}},"paste":{"error":true}}),
    )
    .await;
    assert_eq!(actual["status"], "error");
    assert_eq!(actual["calls"].as_array().unwrap().len(), 1);
}
#[tokio::test]
async fn bounds_stop_work_before_paste_and_do_not_limit_normal_widths() {
    let large = "x".repeat(MAX_INPUT_BYTES + 1);
    for payload in [
        json!({"argumentValues":{"markdownImage":large}}),
        json!({"argumentValues":{"markdownImage":"![a](u)","imageWidth":large}}),
        json!({"preferences":{"imageWidth":large}}),
    ] {
        let actual = scenario::execute(&json!({"payload":payload})).await;
        assert_eq!(actual["status"], "error");
        assert_eq!(actual["calls"], json!([]));
    }
    let actual = scenario::execute(&json!({"selection":{"value":large}})).await;
    assert_eq!(actual["status"], "error");
    assert_eq!(actual["calls"].as_array().unwrap().len(), 1);
    // A large preference expands per image; no OS paste when output is too large.
    let width = "x".repeat(MAX_INPUT_BYTES);
    let actual = scenario::execute(&json!({"payload":{
        "argumentValues":{"markdownImage":"![a](u) ![b](v) ![c](w) ![d](x) ![e](y)"},
        "preferences":{"imageWidth":width}
    }}))
    .await;
    assert_eq!(actual["status"], "error");
    assert_eq!(actual["calls"], json!([]));
    let text = format!("{}{}", "prefix".repeat(1000), "![a](u) ".repeat(4000));
    assert!(convert(&text, "300").unwrap_err().contains("work budget"));
    // Empty captures must not make scanning a huge preference unbounded.
    assert!(
        convert(&"![](u) ".repeat(100), &"$1".repeat(MAX_INPUT_BYTES / 2))
            .unwrap_err()
            .contains("work budget")
    );
    assert!(
        convert("![a](u)", &"x".repeat(MAX_INPUT_BYTES))
            .unwrap()
            .len()
            < MAX_OUTPUT_BYTES
    );
    assert!(convert("![a](u)", "0").unwrap().contains("width=\"0\""));
}
#[test]
fn command_catalog_and_valid_utf8_are_exact() {
    assert_eq!(native_core::extensions::adapter::commands(), &["index"]);
    assert_eq!(
        convert("😃 ![😃](u\u{0085}😃)", "300").unwrap(),
        "😃 <img width=\"300\" alt=\"😃\" src=\"u\u{0085}😃\"/>"
    );
}
