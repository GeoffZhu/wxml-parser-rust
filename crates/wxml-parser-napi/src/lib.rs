use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub fn parse(code: String) -> Result<serde_json::Value> {
  Ok(wxml_parser_core::parse_json(&code))
}

#[napi(js_name = "parseForESLint")]
pub fn parse_for_eslint(code: String) -> Result<serde_json::Value> {
  Ok(wxml_parser_core::parse_for_eslint_json(&code))
}
