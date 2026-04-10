use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Parse WXML and return a JSON string.
/// The JS side calls JSON.parse() to get the object.
/// This is much faster than returning serde_json::Value (avoids BTreeMap overhead)
/// and faster than building JS objects directly (too many FFI calls).
#[napi]
pub fn parse(code: String) -> Result<String> {
  Ok(wxml_parser_rs::parse_json_string(&code))
}

/// Parse WXML for ESLint and return a JSON string.
/// The JS side calls JSON.parse() to get the object.
#[napi(js_name = "parseForESLint")]
pub fn parse_for_eslint(code: String) -> Result<String> {
  Ok(wxml_parser_rs::parse_for_eslint_json_string(&code))
}
