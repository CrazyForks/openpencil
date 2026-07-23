//! Main-thread bridge to the isolated Figma conversion Worker.

use wasm_bindgen::{prelude::*, JsCast};

#[wasm_bindgen(module = "/src/figma_temp_worker.js")]
extern "C" {
    #[wasm_bindgen(js_name = opStartFigmaTempImport, catch)]
    fn op_start_figma_temp_import(
        file: &web_sys::File,
        file_name: &str,
        session_id: &str,
        done: &js_sys::Function,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = opDeleteFigmaTempSession)]
    fn op_delete_figma_temp_session(session_id: &str, page_count: usize);

    #[wasm_bindgen(js_name = opCancelAllFigmaTempImports)]
    fn op_cancel_all_figma_temp_imports();
}

/// Successful output returned after the Worker has committed its manifest.
pub(crate) struct FigmaTempResult {
    pub session_id: String,
    pub page_count: usize,
    pub full_document_json: String,
    pub warnings_json: String,
}

fn string_field(value: &JsValue, name: &str) -> Option<String> {
    js_sys::Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|field| field.as_string())
}

fn usize_field(value: &JsValue, name: &str) -> usize {
    js_sys::Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|field| field.as_f64())
        .filter(|number| number.is_finite() && *number >= 0.0)
        .map_or(0, |number| number as usize)
}

fn parse_result(value: JsValue) -> Result<FigmaTempResult, String> {
    let ok = js_sys::Reflect::get(&value, &JsValue::from_str("ok"))
        .ok()
        .and_then(|field| field.as_bool())
        .unwrap_or(false);
    if !ok {
        return Err(string_field(&value, "error")
            .unwrap_or_else(|| "Figma Worker failed without an error message".to_string()));
    }
    let session_id = string_field(&value, "sessionId")
        .ok_or_else(|| "Figma Worker result is missing sessionId".to_string())?;
    let page_count = usize_field(&value, "pageCount");
    if page_count == 0 {
        return Err("Figma Worker result contains no pages".to_string());
    }
    let full_document_json = string_field(&value, "fullDocumentJson")
        .ok_or_else(|| "Figma Worker result is missing canonical JSON".to_string())?;
    let warnings_json = string_field(&value, "warningsJson").unwrap_or_else(|| "[]".to_string());
    Ok(FigmaTempResult {
        session_id,
        page_count,
        full_document_json,
        warnings_json,
    })
}

/// Hand a browser `File` directly to the module Worker. The raw bytes never
/// cross Rust's main-thread WASM boundary.
pub(crate) fn start(
    file: &web_sys::File,
    file_name: &str,
    session_id: &str,
    done: impl FnOnce(Result<FigmaTempResult, String>) + 'static,
) -> Result<(), JsValue> {
    let done = Closure::<dyn FnMut(JsValue)>::once(move |value| done(parse_result(value)));
    let result =
        op_start_figma_temp_import(file, file_name, session_id, done.as_ref().unchecked_ref());
    // The JS adapter calls exactly once on success, IndexedDB failure, Worker
    // error, or construction/CSP failure. Keep the callback alive until then.
    if result.is_ok() {
        done.forget();
    }
    result
}

/// Remove a completed temp session whose callback lost the host import-
/// generation race (for example, the user opened another document first).
pub(crate) fn delete_session(session_id: &str, page_count: usize) {
    op_delete_figma_temp_session(session_id, page_count);
}

/// Terminate every isolated parser Worker before a new document replacement
/// takes ownership. The generation guard still decides whether a queued
/// completion may install or fall back.
pub(crate) fn cancel_all() {
    op_cancel_all_figma_temp_imports();
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;

    #[test]
    fn failed_worker_result_preserves_the_error_for_fallback_logging() {
        let value = js_sys::Object::new();
        js_sys::Reflect::set(&value, &"ok".into(), &false.into()).unwrap();
        js_sys::Reflect::set(&value, &"error".into(), &"quota exceeded".into()).unwrap();
        let error = match parse_result(value.into()) {
            Ok(_) => panic!("failure result must not parse as success"),
            Err(error) => error,
        };
        assert_eq!(error, "quota exceeded");
    }
}
