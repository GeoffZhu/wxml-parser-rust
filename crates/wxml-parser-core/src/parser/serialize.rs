use std::fmt::Write;

use super::ir::{
  AttributeIr,
  EndTagIr,
  InterpolationIr,
  NodeIr,
  ParseErrorIr,
  ParsedProgram,
  ScriptBodyIr,
  ScriptCommentIr,
  ScriptErrorIr,
  ScriptLocIr,
  ScriptNodeIr,
  ScriptProgramIr,
  Span,
  StartTagIr,
};

fn write_json_str(s: &str, out: &mut String) {
  out.push('"');
  for ch in s.chars() {
    match ch {
      '"' => out.push_str("\\\""),
      '\\' => out.push_str("\\\\"),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      c if c.is_control() => write!(out, "\\u{:04x}", c as u32).unwrap(),
      c => out.push(c),
    }
  }
  out.push('"');
}

fn span_to_json_loc(span: &Span, out: &mut String) {
  let mut end_col = span.end_col;
  if span.start_idx != span.end_idx {
    end_col = end_col.saturating_sub(1);
  }
  out.push_str(r#"{"start":{"line:"#);
  write!(out, "{},\"column\":{}}}", span.start_line, span.start_col).unwrap();
  out.push_str(r#","end":{"line":"#);
  write!(out, "{},\"column\":{}}}", span.end_line, end_col).unwrap();
  out.push('}');
}

fn serialize_script_loc(loc: &ScriptLocIr, out: &mut String) {
  out.push_str(r#"{"start":{"line":"#);
  write!(out, "{},\"column\":{}}}", loc.start_line, loc.start_col).unwrap();
  out.push_str(r#","end":{"line":"#);
  write!(out, "{},\"column\":{}}}", loc.end_line, loc.end_col).unwrap();
  out.push('}');
}

fn serialize_script_body(body: &ScriptBodyIr, out: &mut String) {
  match body {
    ScriptBodyIr::MemberExpression { loc } => {
      out.push_str(r#"{"type":"MemberExpression","loc":"#);
      serialize_script_loc(loc, out);
      out.push_str(r#","range":[0,0]}"#);
    }
  }
}

fn serialize_script_comment(comment: &ScriptCommentIr, out: &mut String) {
  out.push_str(r#"{"type":"#);
  write_json_str(&comment.typ, out);
  out.push_str(r#"","loc":"#);
  serialize_script_loc(&comment.loc, out);
  out.push_str(r#","range":[0,0]}"#);
}

fn serialize_script_program(program: &ScriptProgramIr, out: &mut String) {
  out.push_str(r#"{"type":"WXScriptProgram","offset":[],"body":["#);
  let mut first = true;
  for body in &program.body {
    if !first { out.push(','); }
    first = false;
    serialize_script_body(body, out);
  }
  out.push_str(r#"],"comments":["#);
  let mut first = true;
  for comment in &program.comments {
    if !first { out.push(','); }
    first = false;
    serialize_script_comment(comment, out);
  }
  out.push_str(r#"],"loc":"#);
  serialize_script_loc(&program.loc, out);
  out.push_str(r#","range":[0,0]}"#);
}

fn serialize_interpolation(interp: &InterpolationIr, out: &mut String) {
  let (start, end) = (interp.span.start_idx, interp.span.end_idx);
  out.push_str(r#"{"type":"#);
  write_json_str(interp.typ, out);
  out.push_str(r#"","rawValue":"#);
  write_json_str(&interp.raw_value, out);
  out.push_str(r#"","value":"#);
  write_json_str(interp.value, out);
  out.push_str(r#"","start":#);
  write!(out, "{},\"end\":{}", start, end).unwrap();
  out.push_str(r#","loc":"#);
  span_to_json_loc(&interp.span, out);
  out.push_str(r#","range":["#);
  write!(out, "{},{}", start, end).unwrap();
  out.push(']');
  out.push('}');
}

fn serialize_attribute(attr: &AttributeIr, out: &mut String) {
  let (start, end) = (attr.span.start_idx, attr.span.end_idx);
  out.push_str(r#"{"type":"WXAttribute","key":"#);
  write_json_str(attr.key, out);
  out.push_str(r#"","quote":"#);
  match attr.quote {
    Some(q) => write_json_str(q, out),
    None => out.push_str("null"),
  }
  out.push_str(r#"","value":"#);
  match &attr.value {
    Some(v) => write_json_str(v, out),
    None => out.push_str("null"),
  }
  out.push_str(r#"","rawValue":"#);
  match &attr.raw_value {
    Some(v) => write_json_str(v, out),
    None => out.push_str("null"),
  }
  out.push_str(r#"","children":["#);
  let mut first = true;
  for child in &attr.children {
    if !first { out.push(','); }
    first = false;
    serialize_node(child, out);
  }
  out.push_str(r#"],"interpolations":["#);
  let mut first = true;
  for interp in &attr.interpolations {
    if !first { out.push(','); }
    first = false;
    out.push_str(r#"{"type":"WXInterpolation","rawValue":"#);
    write_json_str(&interp.raw_value, out);
    out.push_str(r#"","value":"#);
    write_json_str(interp.value, out);
    out.push_str(r#"","start":#);
    write!(out, "{},\"end\":{}", interp.span.start_idx, interp.span.end_idx).unwrap();
    out.push_str(r#","loc":"#);
    span_to_json_loc(&interp.span, out);
    out.push_str(r#","range":["#);
    write!(out, "{},{}", interp.span.start_idx, interp.span.end_idx).unwrap();
    out.push_str("]}");
  }
  out.push_str(r#"],"start":#);
  write!(out, "{},\"end\":{}", start, end).unwrap();
  out.push_str(r#","loc":"#);
  span_to_json_loc(&attr.span, out);
  out.push_str(r#","range":["#);
  write!(out, "{},{}", start, end).unwrap();
  out.push(']');
  out.push('}');
}

fn serialize_start_tag(tag: &StartTagIr, out: &mut String) {
  let (start, end) = (tag.span.start_idx, tag.span.end_idx);
  out.push_str(r#"{"type":"WXStartTag","name":"#);
  write_json_str(tag.name, out);
  out.push_str(r#"","attributes":["#);
  let mut first = true;
  for attr in &tag.attributes {
    if !first { out.push(','); }
    first = false;
    serialize_attribute(attr, out);
  }
  out.push_str(r#"],"selfClosing":#);
  out.push_str(if tag.self_closing { "true" } else { "false" });
  out.push_str(r#","start":#);
  write!(out, "{},\"end\":{}", start, end).unwrap();
  out.push_str(r#","loc":"#);
  span_to_json_loc(&tag.span, out);
  out.push_str(r#","range":["#);
  write!(out, "{},{}", start, end).unwrap();
  out.push(']');
  out.push('}');
}

fn serialize_end_tag(tag: &EndTagIr, out: &mut String) {
  let (start, end) = (tag.span.start_idx, tag.span.end_idx);
  out.push_str(r#"{"type":"WXEndTag","name":"#);
  write_json_str(tag.name, out);
  out.push_str(r#"","start":#);
  write!(out, "{},\"end\":{}", start, end).unwrap();
  out.push_str(r#","loc":"#);
  span_to_json_loc(&tag.span, out);
  out.push_str(r#","range":["#);
  write!(out, "{},{}", start, end).unwrap();
  out.push(']');
  out.push('}');
}

fn serialize_script_error(err: &ScriptErrorIr, out: &mut String) {
  let (start, end) = (err.span.start_idx, err.span.end_idx);
  out.push_str(r#"{"type":"WXScriptError","value":"#);
  write_json_str(&err.value, out);
  out.push_str(r#"","start":#);
  write!(out, "{},\"end\":{}", start, end).unwrap();
  out.push_str(r#","loc":{"start":{"line":#);
  write!(out, "{},\"column\":{}}},\"end\":{{\"line\":{},\"column\":{}}}}}", err.line, err.column, err.line, err.column).unwrap();
  out.push_str(r#","range":["#);
  write!(out, "{},{}", start, end).unwrap();
  out.push(']');
  out.push('}');
}

fn serialize_script_node(script: &ScriptNodeIr, out: &mut String) {
  let (start, end) = (script.span.start_idx, script.span.end_idx);
  out.push_str(r#"{"type":"WXScript","name":"#);
  write_json_str(script.name, out);
  out.push('"');
  out.push_str(r#","value":"#);
  match script.value {
    Some(v) => write_json_str(v, out),
    None => out.push_str("null"),
  }
  out.push_str(r#","startTag":"#);
  match &script.start_tag {
    Some(t) => serialize_start_tag(t, out),
    None => out.push_str("null"),
  }
  out.push_str(r#","endTag":"#);
  match &script.end_tag {
    Some(t) => serialize_end_tag(t, out),
    None => out.push_str("null"),
  }
  if let Some(body) = &script.body {
    out.push_str(r#","body":["#);
    serialize_script_program(body, out);
    out.push(']');
  }
  if let Some(error) = &script.error {
    out.push_str(r#","error":"#);
    serialize_script_error(error, out);
  }
  out.push_str(r#","start":#);
  write!(out, "{},\"end\":{}", start, end).unwrap();
  out.push_str(r#","loc":"#);
  span_to_json_loc(&script.span, out);
  out.push_str(r#","range":["#);
  write!(out, "{},{}", start, end).unwrap();
  out.push_str("]}");
}

fn serialize_node(node: &NodeIr, out: &mut String) {
  match node {
    NodeIr::Text { value, span } => {
      let (start, end) = (span.start_idx, span.end_idx);
      out.push_str(r#"{"type":"WXText","value":"#);
      write_json_str(value, out);
      out.push_str(r#"","start":#);
      write!(out, "{},\"end\":{}", start, end).unwrap();
      out.push_str(r#","loc":"#);
      span_to_json_loc(span, out);
      out.push_str(r#","range":["#);
      write!(out, "{},{}", start, end).unwrap();
      out.push_str("]}");
    }
    NodeIr::Comment { value, span } => {
      let (start, end) = (span.start_idx, span.end_idx);
      out.push_str(r#"{"type":"WXComment","value":"#);
      write_json_str(value, out);
      out.push_str(r#"","start":#);
      write!(out, "{},\"end\":{}", start, end).unwrap();
      out.push_str(r#","loc":"#);
      span_to_json_loc(span, out);
      out.push_str(r#","range":["#);
      write!(out, "{},{}", start, end).unwrap();
      out.push_str("]}");
    }
    NodeIr::Interpolation(interp) => serialize_interpolation(interp, out),
    NodeIr::Element {
      name,
      children,
      start_tag,
      end_tag,
      span,
    } => {
      let (start, end) = (span.start_idx, span.end_idx);
      out.push_str(r#"{"type":"WXElement","name":"#);
      write_json_str(name, out);
      out.push_str(r#"","children":["#);
      let mut first = true;
      for child in children {
        if !first { out.push(','); }
        first = false;
        serialize_node(child, out);
      }
      out.push_str(r#"],"startTag":"#);
      match start_tag {
        Some(t) => serialize_start_tag(t, out),
        None => out.push_str("null"),
      }
      out.push_str(r#","endTag":"#);
      match end_tag {
        Some(t) => serialize_end_tag(t, out),
        None => out.push_str("null"),
      }
      out.push_str(r#","start":#);
      write!(out, "{},\"end\":{}", start, end).unwrap();
      out.push_str(r#","loc":"#);
      span_to_json_loc(span, out);
      out.push_str(r#","range":["#);
      write!(out, "{},{}", start, end).unwrap();
      out.push_str("]}");
    }
    NodeIr::Script(script) => serialize_script_node(script, out),
  }
}

fn serialize_error(err: &ParseErrorIr, out: &mut String) {
  let span = &err.span;
  out.push_str(r#"{"type":"#);
  write_json_str(err.typ, out);
  out.push('"');
  if let Some(raw_type) = err.raw_type {
    out.push_str(r#","rawType":"#);
    write_json_str(raw_type, out);
    out.push('"');
  }
  out.push_str(r#","value":"#);
  write_json_str(&err.value, out);
  out.push_str(r#"","start":#);
  write!(out, "{},\"end\":{}", span.start_idx, span.end_idx).unwrap();
  out.push_str(r#","loc":"#);
  span_to_json_loc(span, out);
  out.push_str(r#","range":["#);
  write!(out, "{},{}", span.start_idx, span.end_idx).unwrap();
  out.push(']');
  out.push('}');
}

pub(crate) fn serialize_program(program: ParsedProgram) -> serde_json::Value {
  if program.code_len == 0 {
    return serde_json::json!({
      "type": "Program",
      "body": [],
      "comments": [],
      "errors": [],
      "tokens": [],
      "start": serde_json::Value::Null,
      "end": serde_json::Value::Null,
      "loc": {
        "start": { "line": serde_json::Value::Null, "column": serde_json::Value::Null },
        "end": { "line": serde_json::Value::Null, "column": serde_json::Value::Null }
      },
      "range": [serde_json::Value::Null, serde_json::Value::Null],
    });
  }

  let mut out = String::with_capacity(program.code_len * 4);
  out.push_str(r#"{"type":"Program","body":["#);
  let mut first = true;
  for node in &program.body {
    if !first { out.push(','); }
    first = false;
    serialize_node(node, &mut out);
  }
  out.push_str(r#"],"comments":["#);
  let mut first = true;
  for idx in &program.comment_indices {
    if let Some(node) = program.body.get(*idx) {
      if !first { out.push(','); }
      first = false;
      serialize_node(node, &mut out);
    }
  }
  out.push_str(r#"],"errors":["#);
  let mut first = true;
  for err in &program.errors {
    if !first { out.push(','); }
    first = false;
    serialize_error(err, &mut out);
  }
  out.push_str("],\"tokens\":[],\"start\":0,\"end\":");
  write!(out, "{}", program.code_len).unwrap();
  out.push_str(",\"loc\":{\"start\":{\"line\":1,\"column\":1},\"end\":{\"line\":");
  write!(out, "{},\"column\":{}}}", program.end_line, program.end_col).unwrap();
  out.push_str(",\"range\":[0,");
  write!(out, "{}]", program.code_len).unwrap();
  out.push('}');

  // Parse the string back to Value to maintain API compatibility
  // This is still faster than building with json!() because we avoid
  // the massive number of intermediate HashMap allocations
  serde_json::from_str(&out).unwrap_or_else(|_| serde_json::Value::Null)
}

pub(crate) fn serialize_eslint(program: ParsedProgram) -> serde_json::Value {
  let ast = serialize_program(program);
  serde_json::json!({
    "ast": ast,
    "services": {},
    "scopeManager": serde_json::Value::Null,
    "visitorKeys": {
      "Program": ["errors", "body"]
    }
  })
}