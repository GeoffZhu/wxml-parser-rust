use serde_json::Value;
use wxml_parser_core::parse_json;

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
fn program_shape_and_empty_input() {
  let ast = parse_json("<app></app>");
  assert_eq!(ast.get("type").and_then(Value::as_str), Some("Program"));
  assert!(ast.get("body").and_then(Value::as_array).is_some());
  assert!(ast.get("errors").and_then(Value::as_array).is_some());
  assert!(ast.get("loc").is_some());
  assert!(ast.get("range").and_then(Value::as_array).is_some());

  let empty = parse_json("");
  assert_eq!(empty.get("type").and_then(Value::as_str), Some("Program"));
  assert!(empty.get("errors").and_then(Value::as_array).unwrap().is_empty());
  assert!(empty.get("start").unwrap().is_null());
  assert!(empty.get("end").unwrap().is_null());
  assert!(empty.get("range").unwrap().get(0).unwrap().is_null());
  assert!(empty.get("range").unwrap().get(1).unwrap().is_null());
}

#[test]
fn base_element_and_attribute() {
  let ast = parse_json("<popup show main=\"zhuzhu\" quote='single' />");

  let elements = nodes_by_type(&ast, "WXElement");
  assert_eq!(elements.len(), 1);
  let start_tag = elements[0].get("startTag").unwrap();
  assert_eq!(start_tag.get("selfClosing").and_then(Value::as_bool), Some(true));

  let attrs = nodes_by_type(&ast, "WXAttribute");
  assert_eq!(attrs.len(), 3);
  assert_eq!(attrs[0].get("key").and_then(Value::as_str), Some("show"));
  assert!(attrs[0].get("value").unwrap().is_null());

  assert_eq!(attrs[1].get("key").and_then(Value::as_str), Some("main"));
  assert_eq!(attrs[1].get("value").and_then(Value::as_str), Some("zhuzhu"));
  assert_eq!(attrs[1].get("rawValue").and_then(Value::as_str), Some("\"zhuzhu\""));

  assert_eq!(attrs[2].get("key").and_then(Value::as_str), Some("quote"));
  assert_eq!(attrs[2].get("value").and_then(Value::as_str), Some("single"));
  assert_eq!(attrs[2].get("rawValue").and_then(Value::as_str), Some("'single'"));
}

#[test]
fn interpolation_content_and_attribute() {
  let ast = parse_json("<view id=\"item-{{id}}\"> {{ message }} </view>");

  let intps = nodes_by_type(&ast, "WXInterpolation");
  assert_eq!(intps.len(), 2);
  let raw_values: Vec<&str> = intps
    .iter()
    .filter_map(|n| n.get("rawValue").and_then(Value::as_str))
    .collect();
  assert!(raw_values.contains(&"{{id}}"));
  assert!(raw_values.contains(&"{{ message }}"));

  let attr_intps = nodes_by_type(&ast, "WXAttributeInterpolation");
  assert_eq!(attr_intps.len(), 1);
}

#[test]
fn error_tolerant_cases() {
  let ast = parse_json("<app>{{ nihao</app>");
  let errors = ast.get("errors").and_then(Value::as_array).unwrap();
  assert!(!errors.is_empty());
  assert_eq!(errors[0].get("rawType").and_then(Value::as_str), Some("MismatchedTokenException"));
  assert_eq!(errors[0].get("value").and_then(Value::as_str), Some("wx interpolation unexpected end"));

  let ast2 = parse_json("<app a=></app>");
  let errors2 = ast2.get("errors").and_then(Value::as_array).unwrap();
  assert!(!errors2.is_empty());
  assert_eq!(errors2[0].get("rawType").and_then(Value::as_str), Some("NoViableAltException"));

  let attrs2 = nodes_by_type(&ast2, "WXAttribute");
  assert_eq!(attrs2.len(), 1);
  assert_eq!(attrs2[0].get("key").and_then(Value::as_str), Some("a"));
  assert!(attrs2[0].get("value").unwrap().is_null());
}

#[test]
fn unicode_text_attribute_and_wxs_cases() {
  let text_ast = parse_json("<view>中文内容🙂</view>");
  let texts = nodes_by_type(&text_ast, "WXText");
  assert!(texts.iter().any(|node| node.get("value").and_then(Value::as_str) == Some("中文内容🙂")));
  assert!(text_ast.get("errors").and_then(Value::as_array).unwrap().is_empty());

  let attr_ast = parse_json("<view title=\"你好{{name}}🙂\" />");
  let attrs = nodes_by_type(&attr_ast, "WXAttribute");
  assert_eq!(attrs.len(), 1);
  assert_eq!(attrs[0].get("value").and_then(Value::as_str), Some("你好{{name}}🙂"));
  assert_eq!(attrs[0].get("rawValue").and_then(Value::as_str), Some("\"你好{{name}}🙂\""));

  let attr_intps = nodes_by_type(&attr_ast, "WXAttributeInterpolation");
  assert_eq!(attr_intps.len(), 1);
  assert_eq!(attr_intps[0].get("value").and_then(Value::as_str), Some("name"));

  let branch_ast = parse_json("<view>中{{name}}<text>尾🙂</text></view>");
  let branch_errors = branch_ast.get("errors").and_then(Value::as_array).unwrap();
  assert!(branch_errors.is_empty());
  let interpolations = nodes_by_type(&branch_ast, "WXInterpolation");
  assert_eq!(interpolations.len(), 1);
  assert_eq!(interpolations[0].get("value").and_then(Value::as_str), Some("name"));

  let scripts_ast = parse_json("<wxs module=\"u\">const s = \"中文🙂\"</wxs>");
  let scripts = nodes_by_type(&scripts_ast, "WXScript");
  assert_eq!(scripts.len(), 1);
  assert_eq!(scripts[0].get("value").and_then(Value::as_str), Some("const s = \"中文🙂\""));
}

#[test]
fn wxs_cases() {
  let ast = parse_json("<wxs module=\"util\" src=\"../../util.wxs\" />");
  let scripts = nodes_by_type(&ast, "WXScript");
  assert_eq!(scripts.len(), 1);
  assert!(scripts[0].get("value").unwrap().is_null());
  assert!(scripts[0].get("endTag").unwrap().is_null());

  let ast2 = parse_json("<wxs module=\"util\">module.exports = { a: 1 }</wxs>");
  let scripts2 = nodes_by_type(&ast2, "WXScript");
  assert_eq!(scripts2.len(), 1);
  assert_eq!(scripts2[0].get("name").and_then(Value::as_str), Some("wxs"));
  assert!(scripts2[0].get("value").and_then(Value::as_str).unwrap().contains("module.exports"));

  let ast3 = parse_json("<wxs>const a = 1");
  let errors3 = ast3.get("errors").and_then(Value::as_array).unwrap();
  assert!(errors3.iter().any(|e| {
    e.get("value")
      .and_then(Value::as_str)
      == Some("Expecting token of type --> WXS_SLASH_CLOSE <-- but found --> EOF <--")
  }));
}
