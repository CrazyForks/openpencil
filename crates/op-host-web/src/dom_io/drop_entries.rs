//! Recursive browser directory-drop support.
//!
//! `DataTransfer.files` flattens directory structure in common browsers. The
//! legacy entry API is still the interoperable way to retain each file's
//! `fullPath`; calls are kept behind reflection so this module remains usable
//! when a browser exposes only ordinary file drops.

use std::collections::VecDeque;

use js_sys::{Array, Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};

pub(super) struct DroppedProjectFile {
    pub(super) relative_path: String,
    pub(super) file: web_sys::File,
}

type ProjectDone = Box<dyn FnOnce(Result<Vec<DroppedProjectFile>, String>)>;

/// Starts a recursive directory read when the drop contains at least one
/// directory entry. Returns false for ordinary file-only drops so callers can
/// continue through `DataTransfer.files`.
pub(super) fn read_dropped_directory_project(
    transfer: &web_sys::DataTransfer,
    done: ProjectDone,
) -> bool {
    let entries = transfer_entries(transfer);
    if !entries.iter().any(entry_is_directory) {
        return false;
    }
    spawn_local(async move {
        match collect_files(entries).await {
            Ok(files) => done(Ok(files)),
            Err(error) => done(Err(error)),
        }
    });
    true
}

fn transfer_entries(transfer: &web_sys::DataTransfer) -> Vec<JsValue> {
    let items = transfer.items();
    let mut entries = Vec::with_capacity(items.length() as usize);
    for index in 0..items.length() {
        let Some(item) = items.get(index) else {
            continue;
        };
        if item.kind() != "file" {
            continue;
        }
        let Ok(method) = Reflect::get(item.as_ref(), &JsValue::from_str("webkitGetAsEntry"))
            .and_then(|value| value.dyn_into::<Function>())
        else {
            continue;
        };
        if let Ok(entry) = method.call0(item.as_ref()) {
            if !entry.is_null() && !entry.is_undefined() {
                entries.push(entry);
            }
        }
    }
    entries
}

async fn collect_files(entries: Vec<JsValue>) -> Result<Vec<DroppedProjectFile>, String> {
    let mut queue: VecDeque<JsValue> = entries.into();
    let mut files = Vec::new();
    while let Some(entry) = queue.pop_front() {
        if entry_is_file(&entry) {
            let value = JsFuture::from(callback_method_promise(&entry, "file")?)
                .await
                .map_err(js_error)?;
            let file = value
                .dyn_into::<web_sys::File>()
                .map_err(|_| "directory entry did not return a File".to_string())?;
            files.push(DroppedProjectFile {
                relative_path: entry_path(&entry, &file.name()),
                file,
            });
            if files.len() > op_html::MAX_PROJECT_FILES {
                return Err(format!(
                    "HTML project contains more than {} files",
                    op_html::MAX_PROJECT_FILES
                ));
            }
        } else if entry_is_directory(&entry) {
            let reader = call_method0(&entry, "createReader")?;
            loop {
                let value = JsFuture::from(callback_method_promise(&reader, "readEntries")?)
                    .await
                    .map_err(js_error)?;
                let batch = Array::from(&value);
                if batch.length() == 0 {
                    break;
                }
                for index in 0..batch.length() {
                    queue.push_back(batch.get(index));
                }
            }
        }
    }
    if files.is_empty() {
        return Err("dropped directory contains no readable files".to_string());
    }
    Ok(files)
}

fn callback_method_promise(target: &JsValue, name: &str) -> Result<Promise, String> {
    let method = Reflect::get(target, &JsValue::from_str(name))
        .map_err(js_error)?
        .dyn_into::<Function>()
        .map_err(|_| format!("directory entry method `{name}` is unavailable"))?;
    let target = target.clone();
    Ok(Promise::new(&mut move |resolve, reject| {
        if let Err(error) = method.call2(&target, &resolve, &reject) {
            let _ = reject.call1(&JsValue::NULL, &error);
        }
    }))
}

fn call_method0(target: &JsValue, name: &str) -> Result<JsValue, String> {
    Reflect::get(target, &JsValue::from_str(name))
        .map_err(js_error)?
        .dyn_into::<Function>()
        .map_err(|_| format!("directory entry method `{name}` is unavailable"))?
        .call0(target)
        .map_err(js_error)
}

fn entry_is_file(entry: &JsValue) -> bool {
    Reflect::get(entry, &JsValue::from_str("isFile"))
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn entry_is_directory(entry: &JsValue) -> bool {
    Reflect::get(entry, &JsValue::from_str("isDirectory"))
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn entry_path(entry: &JsValue, fallback: &str) -> String {
    Reflect::get(entry, &JsValue::from_str("fullPath"))
        .ok()
        .and_then(|value| value.as_string())
        .map(|path| path.trim_start_matches('/').to_string())
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn js_error(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "browser directory read failed".to_string())
}
