//! Allocation-bounded reader for OpenPencil's top-level `editorMeta` extension.
//!
//! The canonical schema intentionally ignores this editor-only object, so
//! hosts must read it before constructing an [`op_editor_core::EditorState`].
//! This scanner finds only the top-level value and deserializes that small
//! slice; it never materializes or rewrites the document-sized JSON tree.

use crate::editor_meta_error::EditorMetaWriteError;

/// Editor state that affects how a canonical document is reopened.
///
/// Both fields default to their legacy behavior, so files written before a
/// field existed remain compatible. Snake-case aliases accept the former
/// sidecar spelling as well as the canonical camel-case wire spelling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorMeta {
    /// Zero-based active page index at save time.
    #[serde(default, alias = "active_page_index")]
    pub active_page_index: usize,
    /// Use the numeric parent-local geometry authored by a Preserve-mode
    /// Figma import instead of resolving the tree through flex layout.
    #[serde(default, alias = "preserve_authored_geometry")]
    pub preserve_authored_geometry: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireEditorMeta {
    #[serde(default, alias = "active_page_index")]
    active_page_index: usize,
    #[serde(default, alias = "preserve_authored_geometry")]
    preserve_authored_geometry: Option<bool>,
}

/// Parsed metadata plus compatibility inference used for migration decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorMetaExtraction {
    pub meta: EditorMeta,
    /// The short-lived Figma writer omitted the geometry bit and it was
    /// recovered from the generated first-page id.
    pub inferred_preserve_authored_geometry: bool,
}

/// Extract the last top-level `editorMeta` object from canonical JSON.
///
/// Invalid, nested, or absent metadata returns `None`; the normal canonical
/// loader remains responsible for validating the complete document. Matching
/// the last duplicate key follows `serde_json` object semantics.
pub fn extract_editor_meta(src: &str) -> Option<EditorMeta> {
    extract_editor_meta_with_report(src).map(|extraction| extraction.meta)
}

/// Like [`extract_editor_meta`], but preserves whether a known legacy Figma
/// metadata omission had to be inferred.
pub fn extract_editor_meta_with_report(src: &str) -> Option<EditorMetaExtraction> {
    let scan = scan_top_level(src, "editorMeta")?;
    let wire = serde_json::from_str::<WireEditorMeta>(scan.value?).ok()?;
    let inferred_preserve_authored_geometry =
        wire.preserve_authored_geometry.is_none() && scan.first_page_has_figma_id;
    Some(EditorMetaExtraction {
        meta: EditorMeta {
            active_page_index: wire.active_page_index,
            // A short-lived writer version emitted editorMeta without this
            // field for Preserve-mode Figma imports. Recover only when the
            // canonical first page carries op-figma's unambiguous generated
            // ID. Explicit true/false always wins.
            preserve_authored_geometry: wire
                .preserve_authored_geometry
                .unwrap_or(scan.first_page_has_figma_id),
        },
        inferred_preserve_authored_geometry,
    })
}

/// Copy canonical JSON to `writer`, replacing or appending only its top-level
/// `editorMeta` value.
///
/// This is the clean-document Save-As path: the canonical document bytes stay
/// untouched (including legacy/future schema fields and image tables), while
/// the current active page and authored-geometry mode are persisted. The
/// scanner retains only byte offsets, so a large source document is never
/// materialized as a `String`, `Value`, or `PenDocument` clone.
pub fn write_source_with_editor_meta<W: std::io::Write>(
    writer: &mut W,
    src: &str,
    meta: EditorMeta,
) -> Result<(), EditorMetaWriteError> {
    let scan = scan_top_level(src, "editorMeta").ok_or(EditorMetaWriteError::NotTopLevelObject)?;
    let src_bytes = src.as_bytes();
    let write = |result: std::io::Result<()>| {
        result.map_err(|error| EditorMetaWriteError::Write(error.to_string()))
    };
    let serialize = |error: serde_json::Error| EditorMetaWriteError::Serialize(error.to_string());

    if let Some((value_start, value_end)) = scan.value_range {
        write(writer.write_all(&src_bytes[..value_start]))?;
        serde_json::to_writer(&mut *writer, &meta).map_err(serialize)?;
        write(writer.write_all(&src_bytes[value_end..]))?;
        return Ok(());
    }

    write(writer.write_all(&src_bytes[..scan.root_close]))?;
    if scan.has_members {
        write(writer.write_all(b",\"editorMeta\":"))?;
    } else {
        write(writer.write_all(b"\"editorMeta\":"))?;
    }
    serde_json::to_writer(&mut *writer, &meta).map_err(serialize)?;
    write(writer.write_all(&src_bytes[scan.root_close..]))
}

/// Copy canonical JSON while upgrading only the top-level format marker and
/// editor metadata. Every nested byte, including unknown future fields, is
/// preserved verbatim.
pub fn write_source_with_current_schema<W: std::io::Write>(
    writer: &mut W,
    src: &str,
    meta: EditorMeta,
) -> Result<(), EditorMetaWriteError> {
    #[derive(Clone, Copy)]
    enum Replacement {
        FormatVersion,
        EditorMeta,
    }

    let meta_scan =
        scan_top_level(src, "editorMeta").ok_or(EditorMetaWriteError::NotTopLevelObject)?;
    let format_scan =
        scan_top_level(src, "formatVersion").ok_or(EditorMetaWriteError::NotTopLevelObject)?;
    let write_err = |error: std::io::Error| EditorMetaWriteError::Write(error.to_string());
    let serialize_err =
        |error: serde_json::Error| EditorMetaWriteError::Serialize(error.to_string());
    let src_bytes = src.as_bytes();
    let mut replacements = Vec::with_capacity(2);
    if let Some((start, end)) = format_scan.value_range {
        replacements.push((start, end, Replacement::FormatVersion));
    }
    if let Some((start, end)) = meta_scan.value_range {
        replacements.push((start, end, Replacement::EditorMeta));
    }
    replacements.sort_unstable_by_key(|replacement| replacement.0);

    let mut cursor = 0usize;
    for (start, end, replacement) in replacements {
        writer
            .write_all(&src_bytes[cursor..start])
            .map_err(write_err)?;
        match replacement {
            Replacement::FormatVersion => {
                serde_json::to_writer(
                    &mut *writer,
                    jian_ops_schema::version::FORMAT_VERSION_CURRENT,
                )
                .map_err(serialize_err)?;
            }
            Replacement::EditorMeta => {
                serde_json::to_writer(&mut *writer, &meta).map_err(serialize_err)?;
            }
        }
        cursor = end;
    }
    writer
        .write_all(&src_bytes[cursor..meta_scan.root_close])
        .map_err(write_err)?;

    let missing_format = format_scan.value_range.is_none();
    let missing_meta = meta_scan.value_range.is_none();
    let mut needs_comma = meta_scan.has_members;
    if missing_format {
        if needs_comma {
            writer.write_all(b",").map_err(write_err)?;
        }
        writer.write_all(b"\"formatVersion\":").map_err(write_err)?;
        serde_json::to_writer(
            &mut *writer,
            jian_ops_schema::version::FORMAT_VERSION_CURRENT,
        )
        .map_err(serialize_err)?;
        needs_comma = true;
    }
    if missing_meta {
        if needs_comma {
            writer.write_all(b",").map_err(write_err)?;
        }
        writer.write_all(b"\"editorMeta\":").map_err(write_err)?;
        serde_json::to_writer(&mut *writer, &meta).map_err(serialize_err)?;
    }
    writer
        .write_all(&src_bytes[meta_scan.root_close..])
        .map_err(write_err)
}

/// Apply embedded editor metadata to a freshly loaded editor state.
///
/// The page index is clamped to the document's current page count. The
/// authored-geometry bit is copied verbatim; callers should invoke this only
/// when metadata was actually present so an absent extension retains whatever
/// host-specific default was already installed.
pub fn apply_editor_meta(state: &mut op_editor_core::EditorState, meta: EditorMeta) {
    let page_count = state
        .doc
        .pages
        .as_ref()
        .map(|pages| pages.len())
        .unwrap_or(1)
        .max(1);
    state.ui.active_page_index = meta.active_page_index.min(page_count - 1);
    state.editor_ui.preserve_authored_geometry = meta.preserve_authored_geometry;
}

/// Apply saved metadata, or use the legacy reopen policy when it is absent.
///
/// Old files predate Preserve-mode geometry and therefore always reopen in
/// layout mode. If their first page is empty, land on the first page with
/// content so a valid multi-page document does not appear blank.
pub fn apply_editor_meta_or_legacy_fallback(
    state: &mut op_editor_core::EditorState,
    meta: Option<EditorMeta>,
) {
    if let Some(meta) = meta {
        apply_editor_meta(state, meta);
        return;
    }
    state.editor_ui.preserve_authored_geometry = false;
    state.ui.active_page_index = state
        .doc
        .pages
        .as_ref()
        .and_then(|pages| pages.iter().position(|page| !page.children.is_empty()))
        .unwrap_or(0);
}

struct TopLevelScan<'a> {
    value: Option<&'a str>,
    value_range: Option<(usize, usize)>,
    first_page_has_figma_id: bool,
    root_close: usize,
    has_members: bool,
}

fn scan_top_level<'a>(src: &'a str, wanted: &str) -> Option<TopLevelScan<'a>> {
    let bytes = src.as_bytes();
    let mut cursor = skip_ws(bytes, 0);
    let mut found = None;
    let mut found_range = None;
    let mut first_page_has_figma_id = false;
    let mut has_members = false;
    if bytes.get(cursor) != Some(&b'{') {
        return None;
    }
    cursor += 1;

    loop {
        cursor = skip_ws(bytes, cursor);
        if bytes.get(cursor) == Some(&b'}') {
            return Some(TopLevelScan {
                value: found,
                value_range: found_range,
                first_page_has_figma_id,
                root_close: cursor,
                has_members,
            });
        }
        has_members = true;
        let key_start = cursor;
        let key_end = string_end(bytes, key_start)?;
        let matches = key_matches(&src[key_start..key_end], wanted);
        let is_pages = key_matches(&src[key_start..key_end], "pages");

        cursor = skip_ws(bytes, key_end);
        if bytes.get(cursor) != Some(&b':') {
            return None;
        }
        let value_start = skip_ws(bytes, cursor + 1);
        if is_pages {
            // PenPage's canonical field order begins with `id`. Inspect only a
            // bounded prefix while the main scanner is already at `pages`, so
            // a 200+ MB document is not traversed a second time for migration.
            const PAGE_HEAD_LIMIT: usize = 512;
            let mut head_end = value_start.saturating_add(PAGE_HEAD_LIMIT).min(src.len());
            while !src.is_char_boundary(head_end) {
                head_end -= 1;
            }
            first_page_has_figma_id = first_page_has_reliable_figma_id(&src[value_start..head_end]);
        }
        let value_end = value_end(bytes, value_start)?;
        if matches {
            found = Some(&src[value_start..value_end]);
            found_range = Some((value_start, value_end));
        }

        cursor = skip_ws(bytes, value_end);
        match bytes.get(cursor) {
            Some(b',') => cursor += 1,
            Some(b'}') => {
                return Some(TopLevelScan {
                    value: found,
                    value_range: found_range,
                    first_page_has_figma_id,
                    root_close: cursor,
                    has_members,
                });
            }
            _ => return None,
        }
    }
}

fn first_page_has_reliable_figma_id(pages_head: &str) -> bool {
    let bytes = pages_head.as_bytes();
    let mut cursor = skip_ws(bytes, 0);
    if bytes.get(cursor) != Some(&b'[') {
        return false;
    }
    cursor = skip_ws(bytes, cursor + 1);
    if bytes.get(cursor) != Some(&b'{') {
        return false;
    }
    cursor = skip_ws(bytes, cursor + 1);
    let Some(key_end) = string_end(bytes, cursor) else {
        return false;
    };
    if !key_matches(&pages_head[cursor..key_end], "id") {
        return false;
    }
    cursor = skip_ws(bytes, key_end);
    if bytes.get(cursor) != Some(&b':') {
        return false;
    }
    cursor = skip_ws(bytes, cursor + 1);
    let Some(id_end) = string_end(bytes, cursor) else {
        return false;
    };
    let Ok(id) = serde_json::from_str::<String>(&pages_head[cursor..id_end]) else {
        return false;
    };
    id.strip_prefix("figma-page-").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn skip_ws(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes
        .get(cursor)
        .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
    {
        cursor += 1;
    }
    cursor
}

fn string_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'"') {
        return None;
    }
    let mut cursor = start + 1;
    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            b'"' => return Some(cursor + 1),
            b'\\' => cursor = cursor.checked_add(2)?,
            0x00..=0x1f => return None,
            _ => cursor += 1,
        }
    }
    None
}

fn key_matches(raw_json_string: &str, wanted: &str) -> bool {
    let inner = &raw_json_string[1..raw_json_string.len() - 1];
    if !inner.as_bytes().contains(&b'\\') {
        return inner == wanted;
    }
    serde_json::from_str::<String>(raw_json_string).is_ok_and(|key| key == wanted)
}

fn value_end(bytes: &[u8], start: usize) -> Option<usize> {
    match bytes.get(start)? {
        b'"' => string_end(bytes, start),
        b'{' | b'[' => compound_value_end(bytes, start),
        _ => {
            let mut end = start;
            while !matches!(bytes.get(end), None | Some(b',') | Some(b'}')) {
                end += 1;
            }
            while end > start && matches!(bytes[end - 1], b' ' | b'\n' | b'\r' | b'\t') {
                end -= 1;
            }
            (end > start).then_some(end)
        }
    }
}

fn compound_value_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut stack = vec![*bytes.get(start)?];
    let mut cursor = start + 1;
    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            b'"' => cursor = string_end(bytes, cursor)?,
            b'{' | b'[' => {
                stack.push(byte);
                cursor += 1;
            }
            b'}' | b']' => {
                let expected = if byte == b'}' { b'{' } else { b'[' };
                if stack.pop() != Some(expected) {
                    return None;
                }
                cursor += 1;
                if stack.is_empty() {
                    return Some(cursor);
                }
            }
            _ => cursor += 1,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_camel_and_legacy_snake_case_fields() {
        assert_eq!(
            extract_editor_meta(
                r#"{"version":"1","editorMeta":{"activePageIndex":3,"preserveAuthoredGeometry":true},"children":[]}"#,
            ),
            Some(EditorMeta {
                active_page_index: 3,
                preserve_authored_geometry: true,
            })
        );
        assert_eq!(
            extract_editor_meta(
                r#"{"editorMeta":{"active_page_index":2,"preserve_authored_geometry":true}}"#,
            ),
            Some(EditorMeta {
                active_page_index: 2,
                preserve_authored_geometry: true,
            })
        );
    }

    #[test]
    fn absent_preserve_field_keeps_legacy_false_semantics() {
        assert_eq!(
            extract_editor_meta(r#"{"editorMeta":{"activePageIndex":7},"children":[]}"#),
            Some(EditorMeta {
                active_page_index: 7,
                preserve_authored_geometry: false,
            })
        );
    }

    #[test]
    fn missing_preserve_field_recovers_figma_geometry_from_canonical_page_id() {
        assert_eq!(
            extract_editor_meta(
                r#"{"version":"1","pages":[{"id":"figma-page-12","name":"Imported","children":[]}],"children":[],"editorMeta":{"activePageIndex":0}}"#,
            ),
            Some(EditorMeta {
                active_page_index: 0,
                preserve_authored_geometry: true,
            })
        );
        assert_eq!(
            extract_editor_meta(
                r#"{"editorMeta":{"active_page_index":0},"pages":[{"id":"figma-page-0","name":"Imported","children":[]}],"children":[]}"#,
            ),
            Some(EditorMeta {
                active_page_index: 0,
                preserve_authored_geometry: true,
            })
        );
    }

    #[test]
    fn explicit_false_overrides_figma_page_migration_for_both_spellings() {
        for meta in [
            r#"{"activePageIndex":0,"preserveAuthoredGeometry":false}"#,
            r#"{"active_page_index":0,"preserve_authored_geometry":false}"#,
        ] {
            let src = format!(
                r#"{{"version":"1","pages":[{{"id":"figma-page-0","name":"Imported","children":[]}}],"children":[],"editorMeta":{meta}}}"#
            );
            assert_eq!(
                extract_editor_meta(&src),
                Some(EditorMeta {
                    active_page_index: 0,
                    preserve_authored_geometry: false,
                })
            );
        }
    }

    #[test]
    fn missing_metadata_does_not_migrate_even_with_a_figma_page_id() {
        let src = r#"{"version":"1","pages":[{"id":"figma-page-0","name":"Imported","children":[]}],"children":[]}"#;
        let document = jian_ops_schema::load_str(src).expect("fixture").value;
        let mut state = op_editor_core::EditorState::from_document(document);
        state.editor_ui.preserve_authored_geometry = true;

        let meta = extract_editor_meta(src);
        apply_editor_meta_or_legacy_fallback(&mut state, meta);

        assert_eq!(meta, None);
        assert!(!state.editor_ui.preserve_authored_geometry);
    }

    #[test]
    fn figma_like_but_noncanonical_page_id_does_not_trigger_migration() {
        assert_eq!(
            extract_editor_meta(
                r#"{"version":"1","pages":[{"id":"figma-page-preview","name":"Ordinary","children":[]}],"children":[],"editorMeta":{"activePageIndex":0}}"#,
            ),
            Some(EditorMeta {
                active_page_index: 0,
                preserve_authored_geometry: false,
            })
        );
    }

    #[test]
    fn applying_metadata_clamps_page_and_restores_geometry_mode() {
        let document = jian_ops_schema::load_str(
            r#"{"version":"1","pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[]}],"children":[]}"#,
        )
        .expect("fixture")
        .value;
        let mut state = op_editor_core::EditorState::from_document(document);

        apply_editor_meta(
            &mut state,
            EditorMeta {
                active_page_index: 99,
                preserve_authored_geometry: true,
            },
        );

        assert_eq!(state.ui.active_page_index, 1);
        assert!(state.editor_ui.preserve_authored_geometry);
    }

    #[test]
    fn absent_metadata_resets_preserve_and_opens_first_nonempty_page() {
        let document = jian_ops_schema::load_str(
            r#"{"version":"1","pages":[
              {"id":"p1","name":"Empty","children":[]},
              {"id":"p2","name":"Content","children":[
                {"type":"rectangle","id":"visible","width":10,"height":10}
              ]}
            ],"children":[]}"#,
        )
        .expect("fixture")
        .value;
        let mut state = op_editor_core::EditorState::from_document(document);
        state.editor_ui.preserve_authored_geometry = true;

        apply_editor_meta_or_legacy_fallback(&mut state, None);

        assert_eq!(state.ui.active_page_index, 1);
        assert!(!state.editor_ui.preserve_authored_geometry);
    }

    #[test]
    fn escaped_strings_nested_values_and_duplicate_keys_are_bounded_correctly() {
        let src = r#"{
          "note":"quoted \"editorMeta\" and braces } ]",
          "plugin":{"editorMeta":{"preserveAuthoredGeometry":false}},
          "editorMeta":{"activePageIndex":1},
          "editor\u004deta":{"activePageIndex":4,"preserveAuthoredGeometry":true},
          "children":[]
        }"#;
        assert_eq!(
            extract_editor_meta(src),
            Some(EditorMeta {
                active_page_index: 4,
                preserve_authored_geometry: true,
            })
        );
    }

    #[test]
    fn nested_absent_and_invalid_metadata_are_ignored() {
        assert_eq!(
            extract_editor_meta(
                r#"{"plugin":{"editorMeta":{"preserveAuthoredGeometry":true}},"children":[]}"#,
            ),
            None
        );
        assert_eq!(extract_editor_meta(r#"{"children":[]}"#), None);
        assert_eq!(extract_editor_meta(r#"{"editorMeta":"invalid"}"#), None);
    }

    #[test]
    fn streaming_rewrite_preserves_legacy_document_bytes_outside_editor_meta() {
        let src = concat!(
            "{\n",
            "  \"version\":\"0.8.0\",\n",
            "  \"futureExtension\":{\"keep\":true},\n",
            "  \"editorMeta\":{\"activePageIndex\":1,\"oldField\":\"kept-nowhere\"},\n",
            "  \"children\":[]\n",
            "}\n"
        );
        let old_meta = r#"{"activePageIndex":1,"oldField":"kept-nowhere"}"#;
        let mut output = Vec::new();

        write_source_with_editor_meta(
            &mut output,
            src,
            EditorMeta {
                active_page_index: 7,
                preserve_authored_geometry: true,
            },
        )
        .expect("streaming metadata rewrite");

        let output = String::from_utf8(output).expect("UTF-8 output");
        let expected = src.replace(
            old_meta,
            r#"{"activePageIndex":7,"preserveAuthoredGeometry":true}"#,
        );
        assert_eq!(output, expected, "only the metadata value may change");
        assert_eq!(
            extract_editor_meta(&output),
            Some(EditorMeta {
                active_page_index: 7,
                preserve_authored_geometry: true,
            })
        );
    }

    #[test]
    fn streaming_rewrite_appends_metadata_to_empty_and_nonempty_roots() {
        for (src, expected_prefix) in [
            (
                "{}\n",
                r#"{"editorMeta":{"activePageIndex":2,"preserveAuthoredGeometry":false}}"#,
            ),
            (
                "{\"version\":\"1\",\"children\":[]}\n",
                r#"{"version":"1","children":[],"editorMeta":{"activePageIndex":2,"preserveAuthoredGeometry":false}}"#,
            ),
        ] {
            let mut output = Vec::new();
            write_source_with_editor_meta(
                &mut output,
                src,
                EditorMeta {
                    active_page_index: 2,
                    preserve_authored_geometry: false,
                },
            )
            .expect("append metadata");
            let output = String::from_utf8(output).expect("UTF-8 output");
            assert_eq!(output, format!("{expected_prefix}\n"));
        }
    }

    #[test]
    fn streaming_rewrite_rejects_non_object_sources() {
        let mut output = Vec::new();
        assert!(write_source_with_editor_meta(&mut output, "[]", EditorMeta::default()).is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn current_schema_rewrite_preserves_nested_unknown_fields() {
        let source = r#"{"version":"2.8","formatVersion":"1.0","children":[{"type":"rectangle","id":"r","futureNodeField":{"mustSurvive":true}}],"editorMeta":{"activePageIndex":9}}"#;
        let mut output = Vec::new();

        write_source_with_current_schema(
            &mut output,
            source,
            EditorMeta {
                active_page_index: 2,
                preserve_authored_geometry: true,
            },
        )
        .expect("current-schema rewrite");

        let parsed: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
        assert_eq!(
            parsed["formatVersion"],
            jian_ops_schema::version::FORMAT_VERSION_CURRENT
        );
        assert_eq!(
            parsed["children"][0]["futureNodeField"]["mustSurvive"],
            true
        );
        assert_eq!(parsed["editorMeta"]["activePageIndex"], 2);
        assert_eq!(
            parsed["editorMeta"]["preserveAuthoredGeometry"],
            serde_json::Value::Bool(true)
        );
    }
}
