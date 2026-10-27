use crate::native_core::{
    common::{CoreContext, CorePaths},
    extensions::adapter,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub async fn execute(case: &Value) -> Value {
    // CoreContext::new does not read or create these paths. Tests only route RPCs.
    let unused = std::path::PathBuf::from("NO-IO-ALLOWED");
    let ctx = CoreContext::new(
        CorePaths {
            data_dir: unused.clone(),
            config_dir: unused.clone(),
            resource_dir: unused.clone(),
            temp_dir: unused.clone(),
            home_dir: unused,
        },
        None,
    );
    let calls = Arc::new(Mutex::new(Vec::<Value>::new()));
    let log = calls.clone();
    let config = case.clone();
    ctx.set_router(Arc::new(move |channel, args| {
        let calls = log.clone();
        let config = config.clone();
        Box::pin(async move {
            calls.lock().unwrap().push(json!([channel, args]));
            let (key, default) = match channel.as_str() {
                "core:platform:selected-text" => ("selection", json!("")),
                "clipboard:read" => ("clipboard", json!("")),
                "core:platform:paste-text" => ("paste", json!({"ok":true})),
                _ => panic!("Unexpected OS route: {channel}"),
            };
            let spec = &config[key];
            if spec.get("error").is_some() {
                Err("fixture transport error (never echo user text)".into())
            } else {
                Ok(spec.get("value").cloned().unwrap_or(default))
            }
        })
    }))
    .unwrap();
    let result = adapter::handle(
        &ctx,
        case.get("command")
            .and_then(Value::as_str)
            .unwrap_or("index"),
        case.get("operation")
            .and_then(Value::as_str)
            .unwrap_or("extension:run-command"),
        case.get("payload").unwrap_or(&json!({})),
        &mut json!({}),
    )
    .await;
    let calls = calls.lock().unwrap().clone();
    match result {
        Ok(value) => json!({"status":"ok", "calls":calls, "result":value}),
        Err(message) => json!({"status":"error", "calls":calls, "message":message}),
    }
}
