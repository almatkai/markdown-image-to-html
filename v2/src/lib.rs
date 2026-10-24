//! Source-compatible text conversion; no HTML evaluation or OS implementation.
use regex::Regex;
use serde_json::Value;
use std::{future::Future, sync::LazyLock};

pub const NO_MATCH: &str =
    "No valid markdown image found as parameter, selected or in your clipboard.";
pub const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
// Captures iteration may rescan suffixes; replacement scanning costs work too.
pub const MAX_WORK: usize = 64 * 1024 * 1024;
const JS_WS: &str = r"\x09-\x0D\x20\u{00A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}";
static IMAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"!\[([^\n\r\u{{2028}}\u{{2029}}]*?)]\(([^{JS_WS}]+)\)"
    ))
    .expect("constant image regex")
});
static NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[+-]?(?:Infinity|(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)|0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][01]+)$")
        .expect("constant number regex")
});

pub trait Host {
    fn request(
        &self,
        channel: &str,
        args: Vec<Value>,
    ) -> impl Future<Output = Result<Value, String>> + Send;
}

fn bounded(text: &str) -> Result<(), String> {
    if text.len() > MAX_INPUT_BYTES {
        Err("Input exceeds the 4 MiB UTF-8 limit".into())
    } else {
        Ok(())
    }
}
fn js_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{00A0}' | '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}')
}
fn numeric(text: &str) -> bool {
    let trimmed = text.trim_matches(js_whitespace);
    trimmed.is_empty() || NUMBER.is_match(trimmed)
}
fn text_arg<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    match args.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => {
            bounded(s)?;
            Ok(Some(s))
        }
        Some(_) => Err(format!("argumentValues.{key} must be a text string")),
    }
}
// JSON-representable preferences use JavaScript template-literal ToString.
// This is deliberately NOT width validation, HTML escaping, or sanitization.
fn js_string(value: &Value, out: &mut String) -> Result<(), String> {
    match value {
        Value::Null => append(out, "null")?,
        Value::Bool(v) => append(out, if *v { "true" } else { "false" })?,
        Value::Number(v) => {
            let n = v
                .as_f64()
                .ok_or("Preference number cannot be represented")?;
            append(out, ryu_js::Buffer::new().format(n))?;
        }
        Value::String(s) => append(out, s)?,
        Value::Object(fields) => {
            // JSON cannot supply callable toString/valueOf methods. Shadowing
            // toString therefore makes OrdinaryToPrimitive fail, just like JS.
            if fields.contains_key("toString") {
                return Err("Preference cannot be converted to a JavaScript string".into());
            }
            append(out, "[object Object]")?;
        }
        Value::Array(values) => {
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    append(out, ",")?;
                }
                if !v.is_null() {
                    js_string(v, out)?;
                }
            }
        }
    }
    Ok(())
}
fn inputs(payload: &Value) -> Result<(Option<&str>, String), String> {
    if !payload.is_object() {
        return Err("Launch payload must be an object".into());
    }
    static EMPTY: LazyLock<Value> = LazyLock::new(|| serde_json::json!({}));
    let args = payload.get("argumentValues").unwrap_or(&EMPTY);
    if !args.is_object() {
        return Err("argumentValues must be an object".into());
    }
    let markdown = text_arg(
        args,
        if args.get("markdownImage").is_some() {
            "markdownImage"
        } else {
            "mardownImage"
        },
    )?;
    let argument_width = text_arg(args, "imageWidth")?;
    let preferences = payload.get("preferences").unwrap_or(&EMPTY);
    if !preferences.is_object() {
        return Err("preferences must be an object".into());
    }
    let width = if let Some(width) = argument_width.filter(|s| !s.is_empty() && numeric(s)) {
        width.to_string()
    } else {
        let mut width = String::new();
        match preferences.get("imageWidth") {
            None | Some(Value::Null) => width.push_str("300"),
            Some(value) => js_string(value, &mut width)?,
        }
        width
    };
    bounded(&width)?;
    Ok((markdown, width))
}
fn append(out: &mut String, text: &str) -> Result<(), String> {
    if out
        .len()
        .checked_add(text.len())
        .is_none_or(|n| n > MAX_OUTPUT_BYTES)
    {
        return Err("Output exceeds the 16 MiB UTF-8 limit".into());
    }
    out.push_str(text);
    Ok(())
}

/// JavaScript GetSubstitution for a replacement string with two captures.
fn substitute(
    out: &mut String,
    replacement: &str,
    text: &str,
    capture: &regex::Captures<'_>,
) -> Result<(), String> {
    let whole = capture.get(0).expect("whole match");
    let mut rest = replacement;
    while let Some(pos) = rest.find('$') {
        append(out, &rest[..pos])?;
        rest = &rest[pos + 1..];
        let Some(next) = rest.bytes().next() else {
            append(out, "$")?;
            break;
        };
        let (part, consumed) = match next {
            b'$' => ("$", 1),
            b'&' => (whole.as_str(), 1),
            b'`' => (&text[..whole.start()], 1),
            b'\'' => (&text[whole.end()..], 1),
            b'0' if rest
                .as_bytes()
                .get(1)
                .is_some_and(|b| matches!(b, b'1' | b'2')) =>
            {
                (
                    capture
                        .get((rest.as_bytes()[1] - b'0') as usize)
                        .expect("capture")
                        .as_str(),
                    2,
                )
            }
            b'1' | b'2' => (
                capture
                    .get((next - b'0') as usize)
                    .expect("capture")
                    .as_str(),
                1,
            ),
            _ => ("$", 0),
        };
        append(out, part)?;
        rest = &rest[consumed..];
    }
    append(out, rest)
}

pub fn convert(text: &str, width: &str) -> Result<String, String> {
    bounded(text)?;
    bounded(width)?;
    let replacement = format!(r#"<img width="{width}" alt="$1" src="$2"/>"#);
    let mut out = String::new();
    let mut end = 0;
    let mut work = text.len();
    for capture in IMAGE.captures_iter(text) {
        work = work
            .checked_add(text.len())
            .and_then(|n| n.checked_add(replacement.len()))
            .ok_or("Replacement work budget overflow")?;
        if work > MAX_WORK {
            return Err("Replacement exceeds the 64 MiB work budget".into());
        }
        let whole = capture.get(0).expect("whole match");
        append(&mut out, &text[end..whole.start()])?;
        substitute(&mut out, &replacement, text, &capture)?;
        end = whole.end();
    }
    append(&mut out, &text[end..])?;
    Ok(out)
}
fn matching(text: &str) -> Result<bool, String> {
    bounded(text)?;
    Ok(IMAGE.is_match(text))
}

/// Launch once. No automatic retry, clipboard write, hide, or storage operation.
pub async fn run(host: &impl Host, payload: &Value) -> Result<(), String> {
    let (argument, width) = inputs(payload)?;
    let markdown = if let Some(text) = argument.filter(|s| !s.is_empty()) {
        if matching(text)? {
            Some(text.to_owned())
        } else {
            None
        }
    } else {
        None
    };
    let markdown = if markdown.is_some() {
        markdown
    } else {
        // Source catches selected-text errors. Malformed read responses are also
        // unavailable selection; never log the error or selected content.
        match host.request("core:platform:selected-text", vec![]).await {
            Ok(Value::String(text)) if matching(&text)? => Some(text),
            _ => None,
        }
    };
    let markdown = if let Some(text) = markdown {
        text
    } else {
        let value = host
            .request("clipboard:read", vec![])
            .await
            .map_err(|_| "Clipboard read failed".to_string())?;
        let text = value
            .as_str()
            .ok_or("Clipboard read returned a malformed response")?;
        if !matching(text)? {
            return Err(NO_MATCH.into());
        }
        text.to_owned()
    };
    let output = convert(&markdown, &width)?;
    let ack = host
        .request("core:platform:paste-text", vec![Value::String(output)])
        .await
        .map_err(|_| {
            "Paste failed or was not acknowledged; it may already have occurred".to_string()
        })?;
    if !ack.is_object()
        || ack.get("ok") != Some(&Value::Bool(true))
        || ack.get("error").is_some_and(|v| !v.is_null())
        || ack.get("success").is_some_and(|v| v != &Value::Bool(true))
    {
        return Err(
            "Paste failed or returned a malformed acknowledgement; it may already have occurred"
                .into(),
        );
    }
    Ok(())
}
