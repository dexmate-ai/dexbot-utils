//! Immutable model documents behind a stable C ABI. See dexbot.h for ownership.
#![allow(clippy::missing_safety_doc)] // The C header documents pointer contracts.
use serde_json::Value;
use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

thread_local! { static ERROR: RefCell<CString> = RefCell::new(CString::new("").unwrap()); }
fn error(message: impl ToString) {
    let message = message.to_string().replace('\0', "\\0");
    ERROR.with(|slot| *slot.borrow_mut() = CString::new(message).unwrap());
}
fn guard<T>(fallback: T, f: impl FnOnce() -> Result<T, String>) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(value)) => value,
        Ok(Err(message)) => {
            error(message);
            fallback
        }
        Err(_) => {
            error("internal model error (panic caught)");
            fallback
        }
    }
}
unsafe fn text<'a>(value: *const c_char) -> Result<&'a str, String> {
    if value.is_null() {
        return Err("null string argument".into());
    }
    CStr::from_ptr(value)
        .to_str()
        .map_err(|_| "input must be UTF-8".into())
}
unsafe fn value<'a>(node: *const Value) -> Result<&'a Value, String> {
    node.as_ref().ok_or_else(|| "null document argument".into())
}
#[no_mangle]
pub extern "C" fn dexbot_abi_version() -> u32 {
    1
}
#[no_mangle]
pub extern "C" fn dexbot_last_error() -> *const c_char {
    ERROR.with(|slot| slot.borrow().as_ptr())
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_resolve(
    source: *const c_char,
    from_file: bool,
    overlay: *const c_char,
) -> *mut Value {
    guard(ptr::null_mut(), || {
        let source = text(source)?;
        let mut config = if from_file {
            model::RobotConfig::from_file(source)
        } else {
            model::RobotConfig::from_profile(source)
        }
        .map_err(|e| e.to_string())?;
        if !overlay.is_null() {
            config = config
                .with_overlay_yaml("api", text(overlay)?)
                .map_err(|e| e.to_string())?;
        }
        let config = config.resolve().map_err(|e| e.to_string())?;
        Ok(Box::into_raw(Box::new(
            serde_json::to_value(config).map_err(|e| e.to_string())?,
        )))
    })
}
#[no_mangle]
pub extern "C" fn dexbot_profiles() -> *mut Value {
    guard(ptr::null_mut(), || {
        Ok(Box::into_raw(Box::new(serde_json::json!(
            model::available_profiles()
        ))))
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_profile_for(name: *const c_char) -> *mut Value {
    guard(ptr::null_mut(), || {
        Ok(Box::into_raw(Box::new(Value::String(
            model::try_profile_for_robot_name(text(name)?)
                .map_err(|e| e.to_string())?
                .into(),
        ))))
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_parse_urdf(source: *const c_char) -> *mut Value {
    guard(ptr::null_mut(), || {
        let model = model::UrdfModel::parse(text(source)?).map_err(|e| e.to_string())?;
        Ok(Box::into_raw(Box::new(
            serde_json::to_value(model).map_err(|e| e.to_string())?,
        )))
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_document_free(node: *mut Value) {
    if !node.is_null() {
        drop(Box::from_raw(node));
    }
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_get(node: *const Value, key: *const c_char) -> *const Value {
    guard(ptr::null(), || {
        value(node)?
            .get(text(key)?)
            .map(|v| v as *const _)
            .ok_or_else(|| "field not found".into())
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_at(node: *const Value, index: usize) -> *const Value {
    guard(ptr::null(), || {
        value(node)?
            .as_array()
            .and_then(|v| v.get(index))
            .map(|v| v as *const _)
            .ok_or_else(|| "array index out of range".into())
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_type(node: *const Value) -> i32 {
    guard(-1, || {
        Ok(match value(node)? {
            Value::Null => 0,
            Value::Bool(_) => 1,
            Value::Number(_) => 2,
            Value::String(_) => 3,
            Value::Array(_) => 4,
            Value::Object(_) => 5,
        })
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_size(node: *const Value) -> usize {
    guard(0, || match value(node)? {
        Value::Array(v) => Ok(v.len()),
        Value::Object(v) => Ok(v.len()),
        _ => Err("expected array or object".into()),
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_keys(node: *const Value) -> *mut Value {
    guard(ptr::null_mut(), || {
        let keys = value(node)?
            .as_object()
            .ok_or("expected object")?
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        Ok(Box::into_raw(Box::new(serde_json::json!(keys))))
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_string(node: *const Value, length: *mut usize) -> *const c_char {
    guard(ptr::null(), || {
        let length = length.as_mut().ok_or("null length output")?;
        *length = 0;
        let s = value(node)?.as_str().ok_or("expected string")?;
        *length = s.len();
        Ok(s.as_ptr().cast())
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_number(node: *const Value, output: *mut f64) -> bool {
    guard(false, || {
        *output.as_mut().ok_or("null number output")? =
            value(node)?.as_f64().ok_or("expected number")?;
        Ok(true)
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_boolean(node: *const Value, output: *mut bool) -> bool {
    guard(false, || {
        *output.as_mut().ok_or("null boolean output")? =
            value(node)?.as_bool().ok_or("expected boolean")?;
        Ok(true)
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_is_null(node: *const Value) -> bool {
    guard(true, || Ok(value(node)?.is_null()))
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_json(node: *const Value) -> *mut c_char {
    guard(ptr::null_mut(), || {
        Ok(
            CString::new(serde_json::to_string(value(node)?).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
                .into_raw(),
        )
    })
}
#[no_mangle]
pub unsafe extern "C" fn dexbot_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

#[cfg(test)]
mod tests;
