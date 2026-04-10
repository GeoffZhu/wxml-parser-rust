use serde_json::Value;
use wxml_parser_rs::parse_for_eslint_json;

fn collect_nodes_by_type<'a>(value: &'a Value, typ: &str, out: &mut Vec<&'a Value>) {
  match value {
    Value::Object(map) => {
      if map.get("type").and_then(Value::as_str) == Some(typ) {
        out.push(value);
      }
      for v in map.values() {
        collect_nodes_by_type(v, typ, out);
      }
    }
    Value::Array(arr) => {
      for v in arr {
        collect_nodes_by_type(v, typ, out);
      }
    }
    _ => {}
  }
}

fn nodes_by_type<'a>(value: &'a Value, typ: &str) -> Vec<&'a Value> {
  let mut out = Vec::new();
  collect_nodes_by_type(value, typ, &mut out);
  out
}

#[test]
fn parse_for_eslint_shape() {
  let result = parse_for_eslint_json("<app />");

  assert!(result.get("ast").is_some());
  assert!(result.get("services").and_then(Value::as_object).is_some());
  assert!(result.get("scopeManager").unwrap().is_null());

  let keys = result.get("visitorKeys").unwrap();
  assert_eq!(keys.get("Program").unwrap().get(0).and_then(Value::as_str), Some("errors"));
  assert_eq!(keys.get("Program").unwrap().get(1).and_then(Value::as_str), Some("body"));
}

#[test]
fn parse_for_eslint_inline_wxs_success() {
  let code = "<wxs module=\"util\">module.exports = { data: 1 }</wxs>";
  let result = parse_for_eslint_json(code);
  let ast = result.get("ast").unwrap();

  let programs = nodes_by_type(ast, "WXScriptProgram");
  assert_eq!(programs.len(), 1);

  let members = nodes_by_type(ast, "MemberExpression");
  assert!(!members.is_empty());
}

#[test]
fn parse_for_eslint_inline_wxs_error() {
  let code = "<wxs module=\"util\">missing quote(</wxs>";
  let result = parse_for_eslint_json(code);
  let ast = result.get("ast").unwrap();

  let wxs = nodes_by_type(ast, "WXScript");
  assert_eq!(wxs.len(), 1);

  let err = wxs[0].get("error").expect("WXScript should contain error");
  assert_eq!(err.get("type").and_then(Value::as_str), Some("WXScriptError"));
  assert_eq!(err.get("value").and_then(Value::as_str), Some("Unexpected token"));
}

#[test]
fn parse_for_eslint_inline_wxs_unicode_success() {
  let code = "<wxs module=\"util\">const label = \"中文🙂\"; module.exports = { label }</wxs>";
  let result = parse_for_eslint_json(code);
  let ast = result.get("ast").unwrap();

  let programs = nodes_by_type(ast, "WXScriptProgram");
  assert_eq!(programs.len(), 1);

  let wxs = nodes_by_type(ast, "WXScript");
  assert_eq!(wxs.len(), 1);
  assert!(wxs[0].get("error").is_none());
}

#[test]
fn parse_for_eslint_wxs_program_has_loc() {
  let code = "<wxs module=\"util\">module.exports = { data: 1 }</wxs>";
  let result = parse_for_eslint_json(code);
  let ast = result.get("ast").unwrap();

  let programs = nodes_by_type(ast, "WXScriptProgram");
  assert_eq!(programs.len(), 1);

  let loc = programs[0].get("loc").expect("WXScriptProgram should contain loc");
  assert!(loc.get("start").unwrap().get("line").and_then(Value::as_u64).is_some());
  assert!(loc.get("start").unwrap().get("column").and_then(Value::as_u64).is_some());
  assert!(loc.get("end").unwrap().get("line").and_then(Value::as_u64).is_some());
  assert!(loc.get("end").unwrap().get("column").and_then(Value::as_u64).is_some());
}
