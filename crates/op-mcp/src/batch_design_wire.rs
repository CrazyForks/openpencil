//! Hand-rolled wire parser for the `batch_design` `nodes_json` payload:
//! the flat descriptor-array grammar plus its string/number/raw-JSON
//! scanners.
//!
//! Split out of `batch_design.rs` to stay under the 800-line cap.

use super::write_tools::{validate_hex, ALLOWED_KINDS};
use super::BatchInsertItem;

/// Hand-rolled parser for the `nodes_json` payload. Shell-core
/// stays serde-free so the wasm32 bundle doesn't grow. Returns a
/// Vec<BatchInsertItem> on success, an English error string on
/// any structural problem.
///
/// Grammar (whitespace ignored):
///   array      = '[' (item (',' item)* )? ']'
///   item       = '{' pair (',' pair)* '}'
///   pair       = string ':' value
///   string     = '"' chars '"'
///   value      = string | number
///
/// Strings handle `\"` and `\\` escapes inline; no `\u` decode
/// (the wire never carries unicode escapes in tool args today).
pub(crate) fn parse_batch_items(input: &str) -> Result<Vec<BatchInsertItem>, String> {
    // The wire-level parser doesn't unescape JSON string contents
    // — `{"nodes_json":"[\"x\"]"}` arrives here as the raw bytes
    // `[\"x\"]` (backslash + quote). Pre-pass: unescape so the
    // grammar below sees real `"` / `\` / `\n` etc.
    let unescaped = unescape_wire_string(input)?;
    let bytes = unescaped.as_bytes();
    let mut i = 0usize;
    skip_ws(bytes, &mut i);
    if i >= bytes.len() || bytes[i] != b'[' {
        return Err("nodes_json must start with `[`".into());
    }
    i += 1;
    skip_ws(bytes, &mut i);
    let mut out = Vec::new();
    if i < bytes.len() && bytes[i] == b']' {
        return Ok(out); // empty array — caller surfaces InvalidArgument
    }
    loop {
        skip_ws(bytes, &mut i);
        let item = parse_item(bytes, &mut i)?;
        out.push(item);
        skip_ws(bytes, &mut i);
        if i >= bytes.len() {
            return Err("unterminated array".into());
        }
        match bytes[i] {
            b',' => {
                i += 1;
            }
            b']' => {
                i += 1;
                skip_ws(bytes, &mut i);
                if i != bytes.len() {
                    return Err("trailing garbage after array".into());
                }
                return Ok(out);
            }
            other => {
                return Err(format!(
                    "expected `,` or `]` after item, got {:?}",
                    other as char
                ));
            }
        }
    }
}

fn parse_item(bytes: &[u8], i: &mut usize) -> Result<BatchInsertItem, String> {
    if *i >= bytes.len() || bytes[*i] != b'{' {
        return Err("expected `{` to start a descriptor".into());
    }
    *i += 1;
    let mut kind: Option<String> = None;
    let mut name: Option<String> = None;
    let mut x: Option<i32> = None;
    let mut y: Option<i32> = None;
    let mut width: Option<i32> = None;
    let mut height: Option<i32> = None;
    let mut fill_hex: Option<String> = None;
    let mut fill: Option<Vec<jian_ops_schema::style::PenFill>> = None;
    loop {
        skip_ws(bytes, i);
        if *i >= bytes.len() {
            return Err("unterminated descriptor".into());
        }
        if bytes[*i] == b'}' {
            *i += 1;
            break;
        }
        let key = parse_string(bytes, i)?;
        skip_ws(bytes, i);
        if *i >= bytes.len() || bytes[*i] != b':' {
            return Err(format!("expected `:` after key {key:?}"));
        }
        *i += 1;
        skip_ws(bytes, i);
        match key.as_str() {
            "kind" => kind = Some(parse_string(bytes, i)?),
            "name" => name = Some(parse_string(bytes, i)?),
            "fill_hex" => fill_hex = Some(parse_string(bytes, i)?),
            // Generic `fill` passthrough: a full canonical PenFill stack
            // (array of fill objects, or a single fill object) so a batch
            // item can carry gradient / mesh / image fills, not just a
            // solid `fill_hex`. Captured as a balanced raw-JSON slice and
            // deserialized straight into the canonical type.
            "fill" => {
                let raw = capture_raw_json_value(bytes, i)?;
                fill = Some(parse_fill_stack(&raw)?);
            }
            "x" => x = Some(parse_int(bytes, i)?),
            "y" => y = Some(parse_int(bytes, i)?),
            "width" => width = Some(parse_int(bytes, i)?),
            "height" => height = Some(parse_int(bytes, i)?),
            other => return Err(format!("unknown key {other:?} in descriptor")),
        }
        skip_ws(bytes, i);
        if *i < bytes.len() && bytes[*i] == b',' {
            *i += 1;
        }
    }
    let kind = kind.ok_or("descriptor missing `kind`")?;
    if !ALLOWED_KINDS.iter().any(|k| *k == kind) {
        return Err(format!(
            "kind {kind:?} not supported; allowed: {}",
            ALLOWED_KINDS.join(", ")
        ));
    }
    let name = name.ok_or("descriptor missing `name`")?;
    let x = x.ok_or("descriptor missing `x`")?;
    let y = y.ok_or("descriptor missing `y`")?;
    let width = width.ok_or("descriptor missing `width`")?;
    let height = height.ok_or("descriptor missing `height`")?;
    if width < 0 || height < 0 {
        return Err("width / height must be non-negative".into());
    }
    if let Some(ref hex) = fill_hex {
        if !validate_hex(hex) {
            return Err(format!(
                "fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {hex:?}"
            ));
        }
    }
    Ok(BatchInsertItem {
        kind,
        name,
        x,
        y,
        width,
        height,
        fill_hex,
        fill,
    })
}

/// Deserialize a raw JSON `fill` slice into a canonical fill stack.
/// Accepts either an array of fill objects (`[{...}, ...]`) or a single
/// fill object (`{...}`, wrapped into a 1-entry stack), mirroring the
/// `normalize_fill` shape-tolerance on the JSON nodes path.
fn parse_fill_stack(raw: &str) -> Result<Vec<jian_ops_schema::style::PenFill>, String> {
    use jian_ops_schema::style::PenFill;
    let trimmed = raw.trim_start();
    if trimmed.starts_with('[') {
        serde_json::from_str::<Vec<PenFill>>(raw).map_err(|e| format!("invalid `fill` array: {e}"))
    } else {
        serde_json::from_str::<PenFill>(raw)
            .map(|f| vec![f])
            .map_err(|e| format!("invalid `fill` object: {e}"))
    }
}

/// Scan one balanced JSON value (object / array / string / number /
/// `true` / `false` / `null`) starting at `*i`, advance `*i` past it,
/// and return the raw slice. Respects nesting + string escapes so a
/// `}`/`]` inside a string doesn't prematurely close the value.
fn capture_raw_json_value(bytes: &[u8], i: &mut usize) -> Result<String, String> {
    skip_ws(bytes, i);
    if *i >= bytes.len() {
        return Err("expected a JSON value".into());
    }
    let start = *i;
    match bytes[*i] {
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut in_str = false;
            let mut escaped = false;
            while *i < bytes.len() {
                let c = bytes[*i];
                if in_str {
                    if escaped {
                        escaped = false;
                    } else if c == b'\\' {
                        escaped = true;
                    } else if c == b'"' {
                        in_str = false;
                    }
                } else {
                    match c {
                        b'"' => in_str = true,
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                *i += 1;
                                return slice_utf8(bytes, start, *i);
                            }
                        }
                        _ => {}
                    }
                }
                *i += 1;
            }
            Err("unterminated JSON value".into())
        }
        b'"' => {
            // Reuse the string parser to advance past escapes correctly,
            // then return the original quoted slice.
            let _ = parse_string(bytes, i)?;
            slice_utf8(bytes, start, *i)
        }
        _ => {
            // Number / literal — run to the next delimiter.
            while *i < bytes.len()
                && !matches!(bytes[*i], b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
            {
                *i += 1;
            }
            slice_utf8(bytes, start, *i)
        }
    }
}

fn slice_utf8(bytes: &[u8], start: usize, end: usize) -> Result<String, String> {
    std::str::from_utf8(&bytes[start..end])
        .map(|s| s.to_string())
        .map_err(|_| "invalid UTF-8 in JSON value".to_string())
}

/// Reverse the JSON-string escaping the wire parser left intact.
/// Handles `\"` / `\\` / `\n` / `\t` / `\r` / `\/`. Anything else
/// passes through verbatim (no `\u` decode today).
fn unescape_wire_string(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            match next {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'/' => out.push('/'),
                _ => {
                    // Unknown escape — pass through verbatim so
                    // typos surface as parser errors downstream.
                    out.push('\\');
                    out.push(next as char);
                }
            }
            i += 2;
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\\' {
                i += 1;
            }
            let slice = std::str::from_utf8(&bytes[start..i])
                .map_err(|_| "invalid UTF-8 in nodes_json".to_string())?;
            out.push_str(slice);
        }
    }
    Ok(out)
}

fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn parse_string(bytes: &[u8], i: &mut usize) -> Result<String, String> {
    if *i >= bytes.len() || bytes[*i] != b'"' {
        return Err("expected string".into());
    }
    *i += 1;
    let mut out = String::new();
    while *i < bytes.len() {
        let c = bytes[*i];
        if c == b'"' {
            *i += 1;
            return Ok(out);
        }
        if c == b'\\' {
            *i += 1;
            if *i >= bytes.len() {
                return Err("unterminated escape".into());
            }
            let esc = bytes[*i];
            match esc {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'/' => out.push('/'),
                other => return Err(format!("unsupported escape \\{}", other as char)),
            }
            *i += 1;
        } else {
            // Find the next escape/quote and slice so multi-byte
            // chars stay intact (per-byte append would split them).
            let start = *i;
            while *i < bytes.len() && bytes[*i] != b'"' && bytes[*i] != b'\\' {
                *i += 1;
            }
            let slice = std::str::from_utf8(&bytes[start..*i])
                .map_err(|_| "invalid UTF-8 in string".to_string())?;
            out.push_str(slice);
        }
    }
    Err("unterminated string".into())
}

fn parse_int(bytes: &[u8], i: &mut usize) -> Result<i32, String> {
    let start = *i;
    if *i < bytes.len() && bytes[*i] == b'-' {
        *i += 1;
    }
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    if start == *i {
        return Err("expected integer".into());
    }
    let raw = std::str::from_utf8(&bytes[start..*i])
        .map_err(|_| "invalid UTF-8 in integer".to_string())?;
    raw.parse::<i32>()
        .map_err(|_| format!("expected i32, got {raw:?}"))
}
