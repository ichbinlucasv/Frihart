//! Boa bridge (Campaign B-JS, milestone M1).
//!
//! Off unless the crate is built with `--features boa`. A fresh Boa
//! context has ECMAScript builtins and no web host (no `fetch`, no
//! `WebSocket`, no DOM). The default build still refuses every call.
//! Tor and I2P must not pass `allow = true`; this function does not
//! know the tab network, so the caller has to.

use frihart_core::{FrihartError, Result};

use crate::Value;

/// Largest source Boa may see, in bytes.
pub const MAX_SCRIPT_BYTES: usize = 64 * 1024;

/// How many inline classic scripts one document may contribute.
pub const MAX_INLINE_SCRIPTS: usize = 32;

/// Run one source in Boa when `allow` is true and this build has the feature.
pub fn eval_inline(src: &str, allow: bool) -> Result<Value> {
    if !allow {
        return Err(FrihartError::Message("js off".into()));
    }
    if src.len() > MAX_SCRIPT_BYTES {
        return Err(FrihartError::Message("script too large".into()));
    }
    eval_with_boa(src)
}

#[cfg(not(feature = "boa"))]
fn eval_with_boa(_src: &str) -> Result<Value> {
    Err(FrihartError::Message(
        "this build has no Boa (cargo feature `boa`)".into(),
    ))
}

#[cfg(feature = "boa")]
fn eval_with_boa(src: &str) -> Result<Value> {
    let mut context = boa_engine::Context::default();
    let value = context
        .eval(boa_engine::Source::from_bytes(src))
        .map_err(|err| {
            let text = err.to_string();
            let text = text.chars().take(180).collect::<String>();
            FrihartError::Message(format!("boa: {text}"))
        })?;
    map_primitive(&value)
}

#[cfg(feature = "boa")]
fn map_primitive(value: &boa_engine::JsValue) -> Result<Value> {
    if value.is_undefined() {
        return Ok(Value::Undefined);
    }
    if value.is_null() {
        return Ok(Value::Null);
    }
    if let Some(bit) = value.as_boolean() {
        return Ok(Value::Bool(bit));
    }
    if let Some(n) = value.as_number() {
        return Ok(Value::Number(n));
    }
    if let Some(s) = value.as_string() {
        return Ok(Value::String(s.to_std_string_lossy()));
    }
    Err(FrihartError::Message(
        "boa result is not a primitive".into(),
    ))
}

/// Classic inline `<script>` bodies. External scripts (`src`) and
/// non-JS types are skipped. This does not execute anything.
pub fn inline_script_bodies(html: &str) -> Vec<String> {
    let bytes = html.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 7 < bytes.len() && out.len() < MAX_INLINE_SCRIPTS {
        if !starts_with_ignore_ascii_case(&bytes[i..], b"<script") {
            i += 1;
            continue;
        }
        let after = i + 7;
        let boundary = bytes.get(after).copied().unwrap_or(b' ');
        if boundary.is_ascii_alphanumeric() {
            i += 1;
            continue;
        }
        let Some(tag_end) = find_bytes(&bytes[after..], b">") else {
            break;
        };
        let open = &bytes[after..after + tag_end];
        let body_at = after + tag_end + 1;
        let Some(close_rel) = find_ignore_ascii_case(&bytes[body_at..], b"</script") else {
            break;
        };
        let body = &bytes[body_at..body_at + close_rel];
        let external = contains_ignore_ascii_case(open, b"src");
        let classic = is_classic_script(open);
        if !external
            && classic
            && let Ok(text) = std::str::from_utf8(body)
            && text.len() <= MAX_SCRIPT_BYTES
        {
            out.push(text.to_string());
        }
        i = body_at + close_rel + 1;
    }
    out
}

fn is_classic_script(open_tag_inside: &[u8]) -> bool {
    let Some(rel) = find_ignore_ascii_case(open_tag_inside, b"type") else {
        return true;
    };
    let mut j = rel + 4;
    while j < open_tag_inside.len() && open_tag_inside[j].is_ascii_whitespace() {
        j += 1;
    }
    if open_tag_inside.get(j) != Some(&b'=') {
        return true;
    }
    j += 1;
    while j < open_tag_inside.len() && open_tag_inside[j].is_ascii_whitespace() {
        j += 1;
    }
    let quote = open_tag_inside.get(j).copied();
    if matches!(quote, Some(b'"' | b'\'')) {
        j += 1;
    }
    let start = j;
    while j < open_tag_inside.len()
        && open_tag_inside[j] != b'"'
        && open_tag_inside[j] != b'\''
        && !open_tag_inside[j].is_ascii_whitespace()
        && open_tag_inside[j] != b'>'
    {
        j += 1;
    }
    let ty = &open_tag_inside[start..j];
    ty.is_empty()
        || eq_ignore_ascii_case(ty, b"text/javascript")
        || eq_ignore_ascii_case(ty, b"javascript")
        || eq_ignore_ascii_case(ty, b"")
}

fn starts_with_ignore_ascii_case(hay: &[u8], needle: &[u8]) -> bool {
    hay.len() >= needle.len() && eq_ignore_ascii_case(&hay[..needle.len()], needle)
}

fn eq_ignore_ascii_case(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn contains_ignore_ascii_case(hay: &[u8], needle: &[u8]) -> bool {
    find_ignore_ascii_case(hay, needle).is_some()
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn find_ignore_ascii_case(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len())
        .position(|w| eq_ignore_ascii_case(w, needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_false_never_runs() {
        assert!(eval_inline("1+1", false).is_err());
    }

    #[test]
    fn default_build_has_no_boa_unless_the_feature_is_on() {
        let result = eval_inline("1+1", true);
        if cfg!(feature = "boa") {
            assert_eq!(result.unwrap(), Value::Number(2.0));
        } else {
            let err = result.unwrap_err().to_string();
            assert!(err.contains("no Boa"), "{err}");
        }
    }

    #[test]
    fn inline_bodies_skip_external_and_json() {
        let html = r#"
            <script>1+1</script>
            <script src="https://example.com/a.js"></script>
            <script type="application/json">{"a":1}</script>
            <script type="text/javascript">2+2</script>
        "#;
        let bodies = inline_script_bodies(html);
        assert_eq!(bodies.len(), 2);
        assert!(bodies[0].contains("1+1"));
        assert!(bodies[1].contains("2+2"));
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_has_no_web_host() {
        assert_eq!(
            eval_inline("typeof fetch", true).unwrap(),
            Value::String("undefined".into())
        );
        assert_eq!(
            eval_inline("typeof WebSocket", true).unwrap(),
            Value::String("undefined".into())
        );
        assert!(eval_inline("fetch('https://example.com')", true).is_err());
    }
}
