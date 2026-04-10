use serde_json::Value;

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

/// Write a usize value to the output string using itoa (faster than `write!`).
#[inline(always)]
fn write_u64(n: usize, out: &mut String) {
  let mut buf = itoa::Buffer::new();
  out.push_str(buf.format(n));
}

/// Escape and write a string as JSON string value (including surrounding quotes).
#[inline(always)]
fn write_json_str(s: &str, out: &mut String) {
  out.push('"');
  let mut start = 0;
  let bytes = s.as_bytes();
  for (i, &b) in bytes.iter().enumerate() {
    match b {
      b'"' | b'\\' | b'\n' | b'\r' | b'\t' => {
        if start < i {
          out.push_str(&s[start..i]);
        }
        match b {
          b'"' => out.push_str("\\\""),
          b'\\' => out.push_str("\\\\"),
          b'\n' => out.push_str("\\n"),
          b'\r' => out.push_str("\\r"),
          b'\t' => out.push_str("\\t"),
          _ => unreachable!(),
        }
        start = i + 1;
      }
      0x00..=0x1F => {
        if start < i {
          out.push_str(&s[start..i]);
        }
        // Control characters: write as \uXXXX
        let hex = match b {
          0x00 => "0000", 0x01 => "0001", 0x02 => "0002", 0x03 => "0003",
          0x04 => "0004", 0x05 => "0005", 0x06 => "0006", 0x07 => "0007",
          0x08 => "0008", 0x0B => "000b", 0x0C => "000c", 0x0E => "000e",
          0x0F => "000f", 0x10 => "0010", 0x11 => "0011", 0x12 => "0012",
          0x13 => "0013", 0x14 => "0014", 0x15 => "0015", 0x16 => "0016",
          0x17 => "0017", 0x18 => "0018", 0x19 => "0019", 0x1A => "001a",
          0x1B => "001b", 0x1C => "001c", 0x1D => "001d", 0x1E => "001e",
          0x1F => "001f",
          _ => unreachable!(),
        };
        out.push_str("\\u");
        out.push_str(hex);
        start = i + 1;
      }
      _ => {}
    }
  }
  if start < s.len() {
    out.push_str(&s[start..]);
  }
  out.push('"');
}

/// Write start, end, loc, range — the common position block shared by all AST nodes.
/// This reduces the number of push_str calls by writing the entire tail at once.
#[inline(always)]
fn write_pos(start: usize, end: usize, span: &Span, out: &mut String) {
  out.push_str(",\"start\":");
  write_u64(start, out);
  out.push_str(",\"end\":");
  write_u64(end, out);
  out.push_str(",\"loc\":");
  write_loc(span, out);
  out.push_str(",\"range\":[");
  write_u64(start, out);
  out.push(',');
  write_u64(end, out);
  out.push(']');
}

#[inline(always)]
fn write_loc(span: &Span, out: &mut String) {
  let mut end_col = span.end_col;
  if span.start_idx != span.end_idx {
    end_col = end_col.saturating_sub(1);
  }
  out.push_str("{\"start\":{\"line\":");
  write_u64(span.start_line, out);
  out.push_str(",\"column\":");
  write_u64(span.start_col, out);
  out.push_str("},\"end\":{\"line\":");
  write_u64(span.end_line, out);
  out.push_str(",\"column\":");
  write_u64(end_col, out);
  out.push_str("}}");
}

fn write_script_loc(loc: &ScriptLocIr, out: &mut String) {
  out.push_str("{\"start\":{\"line\":");
  write_u64(loc.start_line, out);
  out.push_str(",\"column\":");
  write_u64(loc.start_col, out);
  out.push_str("},\"end\":{\"line\":");
  write_u64(loc.end_line, out);
  out.push_str(",\"column\":");
  write_u64(loc.end_col, out);
  out.push_str("}}");
}

fn write_script_body(body: &ScriptBodyIr, out: &mut String) {
  match body {
    ScriptBodyIr::MemberExpression { loc } => {
      out.push_str("{\"type\":\"MemberExpression\",\"loc\":");
      write_script_loc(loc, out);
      out.push_str(",\"range\":[0,0]}");
    }
  }
}

fn write_script_comment(comment: &ScriptCommentIr, out: &mut String) {
  out.push_str("{\"type\":");
  write_json_str(&comment.typ, out);
  out.push_str(",\"loc\":");
  write_script_loc(&comment.loc, out);
  out.push_str(",\"range\":[0,0]}");
}

fn write_script_program(program: &ScriptProgramIr, out: &mut String) {
  out.push_str("{\"type\":\"WXScriptProgram\",\"offset\":[],\"body\":[");
  let mut first = true;
  for b in &program.body {
    if !first { out.push(','); }
    first = false;
    write_script_body(b, out);
  }
  out.push_str("],\"comments\":[");
  let mut first = true;
  for c in &program.comments {
    if !first { out.push(','); }
    first = false;
    write_script_comment(c, out);
  }
  out.push_str("],\"loc\":");
  write_script_loc(&program.loc, out);
  out.push_str(",\"range\":[0,0]}");
}

fn write_interpolation(interp: &InterpolationIr, out: &mut String) {
  out.push_str("{\"type\":");
  write_json_str(interp.typ, out);
  out.push_str(",\"rawValue\":");
  write_json_str(&interp.raw_value, out);
  out.push_str(",\"value\":");
  write_json_str(interp.value, out);
  write_pos(interp.span.start_idx, interp.span.end_idx, &interp.span, out);
  out.push('}');
}

fn write_attribute(attr: &AttributeIr, out: &mut String) {
  out.push_str("{\"type\":\"WXAttribute\",\"key\":");
  write_json_str(attr.key, out);
  out.push_str(",\"quote\":");
  match attr.quote {
    Some(q) => write_json_str(q, out),
    None => out.push_str("null"),
  }
  out.push_str(",\"value\":");
  match &attr.value {
    Some(v) => write_json_str(v, out),
    None => out.push_str("null"),
  }
  out.push_str(",\"rawValue\":");
  match &attr.raw_value {
    Some(v) => write_json_str(v, out),
    None => out.push_str("null"),
  }
  out.push_str(",\"children\":[");
  let mut first = true;
  for c in &attr.children {
    if !first { out.push(','); }
    first = false;
    write_node(c, out);
  }
  out.push_str("],\"interpolations\":[");
  let mut first = true;
  for i in &attr.interpolations {
    if !first { out.push(','); }
    first = false;
    out.push_str("{\"type\":\"WXInterpolation\",\"rawValue\":");
    write_json_str(&i.raw_value, out);
    out.push_str(",\"value\":");
    write_json_str(i.value, out);
    write_pos(i.span.start_idx, i.span.end_idx, &i.span, out);
    out.push_str("}");
  }
  out.push(']');
  write_pos(attr.span.start_idx, attr.span.end_idx, &attr.span, out);
  out.push('}');
}

fn write_start_tag(tag: &StartTagIr, out: &mut String) {
  out.push_str("{\"type\":\"WXStartTag\",\"name\":");
  write_json_str(tag.name, out);
  out.push_str(",\"attributes\":[");
  let mut first = true;
  for a in &tag.attributes {
    if !first { out.push(','); }
    first = false;
    write_attribute(a, out);
  }
  out.push_str("],\"selfClosing\":");
  out.push_str(if tag.self_closing { "true" } else { "false" });
  write_pos(tag.span.start_idx, tag.span.end_idx, &tag.span, out);
  out.push('}');
}

fn write_end_tag(tag: &EndTagIr, out: &mut String) {
  out.push_str("{\"type\":\"WXEndTag\",\"name\":");
  write_json_str(tag.name, out);
  write_pos(tag.span.start_idx, tag.span.end_idx, &tag.span, out);
  out.push('}');
}

fn write_script_error(err: &ScriptErrorIr, out: &mut String) {
  out.push_str("{\"type\":\"WXScriptError\",\"value\":");
  write_json_str(&err.value, out);
  out.push_str(",\"start\":");
  write_u64(err.span.start_idx, out);
  out.push_str(",\"end\":");
  write_u64(err.span.end_idx, out);
  out.push_str(",\"loc\":{\"start\":{\"line\":");
  write_u64(err.line, out);
  out.push_str(",\"column\":");
  write_u64(err.column, out);
  out.push_str("},\"end\":{\"line\":");
  write_u64(err.line, out);
  out.push_str(",\"column\":");
  write_u64(err.column, out);
  out.push_str("}},\"range\":[");
  write_u64(err.span.start_idx, out);
  out.push(',');
  write_u64(err.span.end_idx, out);
  out.push_str("]}");
}

fn write_script_node(script: &ScriptNodeIr, out: &mut String) {
  out.push_str("{\"type\":\"WXScript\",\"name\":");
  write_json_str(script.name, out);
  out.push_str(",\"value\":");
  match script.value {
    Some(v) => write_json_str(v, out),
    None => out.push_str("null"),
  }
  out.push_str(",\"startTag\":");
  match &script.start_tag {
    Some(t) => write_start_tag(t, out),
    None => out.push_str("null"),
  }
  out.push_str(",\"endTag\":");
  match &script.end_tag {
    Some(t) => write_end_tag(t, out),
    None => out.push_str("null"),
  }
  if let Some(body) = &script.body {
    out.push_str(",\"body\":[");
    write_script_program(body, out);
    out.push(']');
  }
  if let Some(error) = &script.error {
    out.push_str(",\"error\":");
    write_script_error(error, out);
  }
  write_pos(script.span.start_idx, script.span.end_idx, &script.span, out);
  out.push('}');
}

fn write_node(node: &NodeIr, out: &mut String) {
  match node {
    NodeIr::Text { value, span } => {
      out.push_str("{\"type\":\"WXText\",\"value\":");
      write_json_str(value, out);
      write_pos(span.start_idx, span.end_idx, span, out);
      out.push('}');
    }
    NodeIr::Comment { value, span } => {
      out.push_str("{\"type\":\"WXComment\",\"value\":");
      write_json_str(value, out);
      write_pos(span.start_idx, span.end_idx, span, out);
      out.push('}');
    }
    NodeIr::Interpolation(interp) => write_interpolation(interp, out),
    NodeIr::Element { name, children, start_tag, end_tag, span } => {
      out.push_str("{\"type\":\"WXElement\",\"name\":");
      write_json_str(name, out);
      out.push_str(",\"children\":[");
      let mut first = true;
      for c in children {
        if !first { out.push(','); }
        first = false;
        write_node(c, out);
      }
      out.push_str("],\"startTag\":");
      match start_tag {
        Some(t) => write_start_tag(t, out),
        None => out.push_str("null"),
      }
      out.push_str(",\"endTag\":");
      match end_tag {
        Some(t) => write_end_tag(t, out),
        None => out.push_str("null"),
      }
      write_pos(span.start_idx, span.end_idx, span, out);
      out.push('}');
    }
    NodeIr::Script(script) => write_script_node(script, out),
  }
}

fn write_error(err: &ParseErrorIr, out: &mut String) {
  out.push_str("{\"type\":");
  write_json_str(err.typ, out);
  if let Some(raw_type) = err.raw_type {
    out.push_str(",\"rawType\":");
    write_json_str(raw_type, out);
  }
  out.push_str(",\"value\":");
  write_json_str(&err.value, out);
  write_pos(err.span.start_idx, err.span.end_idx, &err.span, out);
  out.push('}');
}

/// Build the JSON string for the program (no serde_json::Value construction).
pub(crate) fn serialize_program_to_string(program: &ParsedProgram) -> String {
  if program.code_len == 0 {
    return r#"{"type":"Program","body":[],"comments":[],"errors":[],"tokens":[],"start":null,"end":null,"loc":{"start":{"line":null,"column":null},"end":{"line":null,"column":null}},"range":[null,null]}"#.to_string();
  }

  let mut out = String::with_capacity(program.code_len * 3);
  out.push_str("{\"type\":\"Program\",\"body\":[");
  let mut first = true;
  for node in &program.body {
    if !first { out.push(','); }
    first = false;
    write_node(node, &mut out);
  }
  out.push_str("],\"comments\":[");
  let mut first = true;
  for idx in &program.comment_indices {
    if let Some(node) = program.body.get(*idx) {
      if !first { out.push(','); }
      first = false;
      write_node(node, &mut out);
    }
  }
  out.push_str("],\"errors\":[");
  let mut first = true;
  for err in &program.errors {
    if !first { out.push(','); }
    first = false;
    write_error(err, &mut out);
  }
  out.push_str("],\"tokens\":[],\"start\":0,\"end\":");
  write_u64(program.code_len, &mut out);
  out.push_str(",\"loc\":{\"start\":{\"line\":1,\"column\":1},\"end\":{\"line\":");
  write_u64(program.end_line, &mut out);
  out.push_str(",\"column\":");
  write_u64(program.end_col, &mut out);
  out.push_str("}},\"range\":[0,");
  write_u64(program.code_len, &mut out);
  out.push_str("]}");

  out
}

/// Build the JSON string for eslint output (no serde_json::Value construction).
pub(crate) fn serialize_eslint_to_string(program: &ParsedProgram) -> String {
  let ast_json = serialize_program_to_string(program);
  let mut out = String::with_capacity(ast_json.len() + 128);
  out.push_str("{\"ast\":");
  out.push_str(&ast_json);
  out.push_str(",\"services\":{},\"scopeManager\":null,\"visitorKeys\":{\"Program\":[\"errors\",\"body\"]}}");
  out
}

pub(crate) fn serialize_program(program: ParsedProgram) -> Value {
  let json_str = serialize_program_to_string(&program);
  serde_json::from_str(&json_str).unwrap_or_else(|_| {
    let errors: Vec<Value> = program.errors.iter().map(|e| serde_json::json!({
      "type": e.typ, "rawType": e.raw_type, "value": e.value,
      "start": e.span.start_idx, "end": e.span.end_idx,
      "loc": span_to_json_loc(&e.span),
      "range": [e.span.start_idx, e.span.end_idx]
    })).collect();
    let body: Vec<Value> = program.body.iter().map(|_| serde_json::Value::Null).collect();
    serde_json::json!({"type":"Program","body":body,"errors":errors})
  })
}

fn span_to_json_loc(span: &Span) -> Value {
  let mut end_col = span.end_col;
  if span.start_idx != span.end_idx {
    end_col = end_col.saturating_sub(1);
  }
  serde_json::json!({
    "start": { "line": span.start_line, "column": span.start_col },
    "end": { "line": span.end_line, "column": end_col }
  })
}

pub(crate) fn serialize_eslint(program: ParsedProgram) -> Value {
  let ast = serialize_program(program);
  serde_json::json!({
    "ast": ast,
    "services": {},
    "scopeManager": Value::Null,
    "visitorKeys": {
      "Program": ["errors", "body"]
    }
  })
}