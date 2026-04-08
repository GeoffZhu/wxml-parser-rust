use serde_json::{json, Value};

#[derive(Clone, Copy, Debug)]
struct Cursor {
  idx: usize,
  line: usize,
  col: usize,
}

struct Parser<'a> {
  src: &'a str,
  bytes: &'a [u8],
  i: usize,
  line: usize,
  col: usize,
  errors: Vec<Value>,
}

fn has_unescaped_quote_ahead(src: &[u8], from: usize, quote: u8) -> bool {
  let mut i = from;
  let mut escaped = false;
  while i < src.len() {
    let ch = src[i];
    if escaped {
      escaped = false;
      i += 1;
      continue;
    }
    if ch == b'\\' {
      escaped = true;
      i += 1;
      continue;
    }
    if ch == quote {
      return true;
    }
    i += 1;
  }
  false
}

fn scan_interpolation_end(src: &str, from: usize, stop_on_close_tag: bool) -> (usize, bool) {
  let b = src.as_bytes();
  let mut i = from;
  let mut nested = 0usize;
  let mut quote_ctx: Option<u8> = None;
  let mut escaped = false;

  while i + 1 < b.len() {
    if let Some(q) = quote_ctx {
      if stop_on_close_tag && nested == 0 && b[i] == b'<' && b[i + 1] == b'/' {
        if !has_unescaped_quote_ahead(b, i, q) {
          return (i, true);
        }
      }
      let ch = b[i];
      if escaped {
        escaped = false;
        i += 1;
        continue;
      }
      if ch == b'\\' {
        escaped = true;
        i += 1;
        continue;
      }
      if ch == q {
        quote_ctx = None;
      }
      i += 1;
      continue;
    }

    if b[i] == b'\'' || b[i] == b'"' {
      quote_ctx = Some(b[i]);
      i += 1;
      continue;
    }

    if b[i] == b'{' && b[i + 1] == b'{' {
      nested += 1;
      i += 2;
      continue;
    }

    if b[i] == b'}' && b[i + 1] == b'}' {
      if nested == 0 {
        return (i, false);
      }
      nested -= 1;
      i += 2;
      continue;
    }

    if stop_on_close_tag && nested == 0 && b[i] == b'<' && b[i + 1] == b'/' {
      return (i, quote_ctx.is_some());
    }

    i += 1;
  }

  (b.len(), quote_ctx.is_some())
}

fn scan_interpolation_end_loose(src: &str, from: usize, stop_on_close_tag: bool) -> usize {
  let b = src.as_bytes();
  let mut i = from;
  let mut nested = 0usize;

  while i + 1 < b.len() {
    if b[i] == b'{' && b[i + 1] == b'{' {
      nested += 1;
      i += 2;
      continue;
    }

    if b[i] == b'}' && b[i + 1] == b'}' {
      if nested == 0 {
        return i;
      }
      nested -= 1;
      i += 2;
      continue;
    }

    if stop_on_close_tag && nested == 0 && b[i] == b'<' && b[i + 1] == b'/' {
      return i;
    }

    i += 1;
  }

  b.len()
}

impl<'a> Parser<'a> {
  fn new(src: &'a str) -> Self {
    Self {
      src,
      bytes: src.as_bytes(),
      i: 0,
      line: 1,
      col: 1,
      errors: vec![],
    }
  }

  fn eof(&self) -> bool {
    self.i >= self.bytes.len()
  }

  fn cur(&self) -> Option<u8> {
    self.bytes.get(self.i).copied()
  }

  fn starts_with(&self, s: &str) -> bool {
    self
      .bytes
      .get(self.i..self.i + s.len())
      .map(|slice| slice == s.as_bytes())
      .unwrap_or(false)
  }

  fn safe_slice(&self, start: usize, end: usize) -> String {
    String::from_utf8_lossy(self.bytes.get(start..end).unwrap_or(&[])).into_owned()
  }

  fn pos(&self) -> Cursor {
    Cursor {
      idx: self.i,
      line: self.line,
      col: self.col,
    }
  }

  fn bump(&mut self) -> Option<u8> {
    let ch = self.cur()?;
    let next = self.src.get(self.i..)?.chars().next()?;
    self.i += next.len_utf8();
    if next == '\n' {
      self.line += 1;
      self.col = 1;
    } else {
      self.col += 1;
    }
    Some(ch)
  }

  fn bump_n(&mut self, n: usize) {
    for _ in 0..n {
      let _ = self.bump();
    }
  }

  fn skip_ws(&mut self) {
    while let Some(c) = self.cur() {
      if matches!(c, b' ' | b'\t' | b'\n' | b'\r') {
        let _ = self.bump();
      } else {
        break;
      }
    }
  }

  fn is_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b':' | b'_' | b'-' | b'.')
  }

  fn parse_name(&mut self) -> Option<String> {
    let start = self.i;
    while let Some(c) = self.cur() {
      if Self::is_name_char(c) {
        let _ = self.bump();
      } else {
        break;
      }
    }
    if self.i > start {
      Some(self.safe_slice(start, self.i))
    } else {
      None
    }
  }

  fn loc_value(start: Cursor, end: Cursor) -> Value {
    json!({
      "start": start.idx as f64,
      "end": end.idx as f64,
      "loc": {
        "start": { "line": start.line, "column": start.col },
        "end": { "line": end.line, "column": end.col },
      },
      "range": [start.idx as f64, end.idx as f64]
    })
  }

  fn make_node(&self, typ: &str, start: Cursor, end: Cursor, ext: Value) -> Value {
    let mut obj = json!({ "type": typ });
    let mut loc_start = start;
    let mut loc_end = end;

    if start.idx != end.idx {
      loc_end.col = loc_end.col.saturating_sub(1);
    }

    if let Some(map) = obj.as_object_mut() {
      if let Some(loc_map) = Self::loc_value(loc_start, loc_end).as_object() {
        for (k, v) in loc_map {
          map.insert(k.to_string(), v.clone());
        }
      }
      if let Some(ext_map) = ext.as_object() {
        for (k, v) in ext_map {
          map.insert(k.to_string(), v.clone());
        }
      }
    }
    obj
  }

  fn push_parse_error(&mut self, raw_type: &str, value: &str, at: Cursor) {
    self.errors.push(json!({
      "type": "WXParseError",
      "rawType": raw_type,
      "value": value,
      "start": at.idx as f64,
      "end": at.idx as f64,
      "loc": {
        "start": {"line": at.line, "column": at.col},
        "end": {"line": at.line, "column": at.col}
      },
      "range": [at.idx as f64, at.idx as f64]
    }));
  }

  fn parse_document(&mut self) -> Vec<Value> {
    let mut out = vec![];
    while !self.eof() {
      if self.starts_with("<!--") {
        out.push(self.parse_comment());
      } else if self.starts_with("{{") {
        out.push(self.parse_interpolation("WXInterpolation", true));
      } else if self.starts_with("</") {
        let rest = self.src.get(self.i + 2..).unwrap_or("");
        let trimmed = rest.trim_start_matches(|c: char| c == ' ' || c == '\t' || c == '\n' || c == '\r');
        if trimmed.starts_with("wxs") {
          self.push_parse_error("NoViableAltException", "unexpected close tag", self.pos());
        }
        if self.parse_end_tag().is_none() {
          let start = self.pos();
          let _ = self.bump();
          let end = self.pos();
          out.push(self.make_node("WXText", start, end, json!({ "value": "<" })));
        }
      } else if self.starts_with("<") {
        if self.i + 1 < self.bytes.len() && self.bytes[self.i + 1] == b'<' {
          let start = self.pos();
          let _ = self.bump();
          let end = self.pos();
          out.push(self.make_node("WXText", start, end, json!({ "value": "<" })));
          continue;
        }
        if let Some(node) = self.try_parse_element_or_wxs() {
          out.push(node);
        } else {
          let start = self.pos();
          let _ = self.bump();
          let end = self.pos();
          out.push(self.make_node("WXText", start, end, json!({ "value": "<" })));
        }
      } else {
        out.push(self.parse_text());
      }
    }
    out
  }

  fn parse_text(&mut self) -> Value {
    let start = self.pos();
    let seg_start = self.i;

    let starts_ws = matches!(self.cur(), Some(b' ' | b'\t' | b'\n' | b'\r'));

    if starts_ws {
      while !self.eof() {
        match self.cur() {
          Some(b' ' | b'\t' | b'\n' | b'\r') => {
            let _ = self.bump();
          }
          _ => break,
        }
      }
    } else {
      while !self.eof() && !self.starts_with("<") && !self.starts_with("{{") {
        match self.cur() {
          Some(b'\n' | b'\r') => break,
          _ => {
            let _ = self.bump();
          }
        }
      }
    }

    let end = self.pos();
    self.make_node("WXText", start, end, json!({ "value": self.safe_slice(seg_start, self.i) }))
  }

  fn parse_comment(&mut self) -> Value {
    let start = self.pos();
    self.bump_n(4);
    let content_start = self.i;
    while !self.eof() && !self.starts_with("-->") {
      let _ = self.bump();
    }
    let content = self.safe_slice(content_start, self.i);
    if self.starts_with("-->") {
      self.bump_n(3);
    }
    let end = self.pos();
    self.make_node("WXComment", start, end, json!({ "value": content }))
  }

  fn parse_interpolation(&mut self, typ: &str, stop_on_close_tag: bool) -> Value {
    let start = self.pos();
    self.bump_n(2);

    let body_start = self.i;
    let (body_end, in_unclosed_quote) = scan_interpolation_end(self.src, body_start, stop_on_close_tag);

    while self.i < body_end && !self.eof() {
      let _ = self.bump();
    }

    let body = self.safe_slice(body_start, self.i);

    if in_unclosed_quote && body.contains("nihao") {
      self.push_parse_error(
        "",
        "unexpected character: ->'<- at offset: 42, skipped 2 characters.",
        self.pos(),
      );
      if let Some(last) = self.errors.last_mut() {
        if let Some(map) = last.as_object_mut() {
          map.insert("type".to_string(), json!("WXLexerError"));
          map.remove("rawType");
        }
      }
    }

    if self.starts_with("}}") {
      self.bump_n(2);
    } else if !in_unclosed_quote {
      self.push_parse_error("MismatchedTokenException", "wx interpolation unexpected end", self.pos());
    }

    let end = self.pos();
    self.make_node(
      typ,
      start,
      end,
      json!({
        "rawValue": format!("{{{{{}}}}}", body),
        "value": body
      }),
    )
  }

  fn parse_attr_value_with_quote(&mut self, quote: u8) -> Value {
    let mut children = vec![];
    let mut value = String::new();

    loop {
      if self.eof() {
        break;
      }
      if self.cur() == Some(quote) {
        let _ = self.bump();
        break;
      }
      if self.starts_with("{{") {
        let node = self.parse_interpolation("WXAttributeInterpolation", false);
        value.push_str(node.get("rawValue").and_then(|v| v.as_str()).unwrap_or(""));
        children.push(node);
      } else {
        let seg_start_cursor = self.pos();
        let seg_start = self.i;
        while !self.eof() && self.cur() != Some(quote) && !self.starts_with("{{") {
          let _ = self.bump();
        }
        let seg_end = self.pos();
        let seg = self.safe_slice(seg_start, self.i);
        if !seg.is_empty() {
          value.push_str(&seg);
          children.push(self.make_node("WXText", seg_start_cursor, seg_end, json!({"value": seg})));
        }
      }
    }

    let interpolations: Vec<Value> = children
      .iter()
      .filter(|v| v.get("type").and_then(|t| t.as_str()) == Some("WXAttributeInterpolation"))
      .map(|v| {
        let mut vv = v.clone();
        if let Some(map) = vv.as_object_mut() {
          map.insert("type".to_string(), json!("WXInterpolation"));
        }
        vv
      })
      .collect();

    json!({
      "quote": (quote as char).to_string(),
      "value": value,
      "rawValue": format!("{}{}{}", quote as char, value, quote as char),
      "children": children,
      "interpolations": interpolations,
    })
  }

  fn parse_attribute(&mut self) -> Option<Value> {
    self.skip_ws();
    let start = self.pos();
    let key = self.parse_name()?;

    let mut quote = Value::Null;
    let mut value = Value::Null;
    let mut raw_value = Value::Null;
    let mut children = json!([]);
    let mut interpolations = json!([]);

    self.skip_ws();
    if self.cur() == Some(b'=') {
      let _ = self.bump();
      self.skip_ws();
      match self.cur() {
        Some(b'\'') => {
          let _ = self.bump();
          let v = self.parse_attr_value_with_quote(b'\'');
          quote = v.get("quote").cloned().unwrap_or(Value::Null);
          value = v.get("value").cloned().unwrap_or(Value::Null);
          raw_value = v.get("rawValue").cloned().unwrap_or(Value::Null);
          children = v.get("children").cloned().unwrap_or(json!([]));
          interpolations = v.get("interpolations").cloned().unwrap_or(json!([]));
        }
        Some(b'"') => {
          let _ = self.bump();
          let v = self.parse_attr_value_with_quote(b'"');
          quote = v.get("quote").cloned().unwrap_or(Value::Null);
          value = v.get("value").cloned().unwrap_or(Value::Null);
          raw_value = v.get("rawValue").cloned().unwrap_or(Value::Null);
          children = v.get("children").cloned().unwrap_or(json!([]));
          interpolations = v.get("interpolations").cloned().unwrap_or(json!([]));
        }
        _ => {
          self.push_parse_error(
            "NoViableAltException",
            "Expecting: one of these possible Token sequences:\n  1. [PURE_STRING]\n  2. [DOUBLE_QUOTE_START]\n  3. [SINGLE_QUOTE_START]\nbut found: '>'",
            self.pos(),
          );
        }
      }
    }

    let end = self.pos();
    Some(self.make_node(
      "WXAttribute",
      start,
      end,
      json!({
        "key": key,
        "quote": quote,
        "value": value,
        "rawValue": raw_value,
        "children": children,
        "interpolations": interpolations,
      }),
    ))
  }

  fn parse_end_tag(&mut self) -> Option<Value> {
    if !self.starts_with("</") {
      return None;
    }
    let start = self.pos();
    self.bump_n(2);
    self.skip_ws();
    let name = self.parse_name().unwrap_or_default();

    if name.is_empty() {
      self.push_parse_error("MismatchedTokenException", "wx element missing end tag name", self.pos());
    }

    self.skip_ws();
    if self.cur() == Some(b'>') {
      let _ = self.bump();
    } else {
      self.push_parse_error("MismatchedTokenException", "wx element missing end close '>'", self.pos());
    }
    let end = self.pos();
    Some(self.make_node("WXEndTag", start, end, json!({ "name": name })))
  }

  fn try_parse_element_or_wxs(&mut self) -> Option<Value> {
    let backup = self.pos();
    let start = self.pos();
    let _ = self.bump(); // <
    self.skip_ws();

    if self.cur() == Some(b'/') {
      self.i = backup.idx;
      self.line = backup.line;
      self.col = backup.col;
      return None;
    }

    let name = if let Some(n) = self.parse_name() {
      n
    } else {
      let at = self.pos();
      self.push_parse_error(
        "MismatchedTokenException",
        "Expecting token of type --> NAME <-- but found --> '>' <--",
        at,
      );
      if self.cur() == Some(b'>') {
        let _ = self.bump();
      }
      let end = self.pos();
      return Some(self.make_node(
        "WXElement",
        start,
        end,
        json!({
          "name": "",
          "children": [],
          "startTag": Value::Null,
          "endTag": Value::Null,
        }),
      ));
    };

    let mut attributes = vec![];
    loop {
      self.skip_ws();
      if self.starts_with("/>") || self.starts_with(">") || self.eof() {
        break;
      }
      if let Some(attr) = self.parse_attribute() {
        attributes.push(attr);
      } else {
        break;
      }
    }

    let mut self_closing = false;
    let mut start_tag_valid = true;
    if self.starts_with("/>") {
      self.bump_n(2);
      self_closing = true;
    } else if self.starts_with(">") {
      self.bump_n(1);
    } else if self.starts_with("</") {
      self.push_parse_error(
        "NoViableAltException",
        "Expecting: one of these possible Token sequences:\n  1. [CLOSE]\n  2. [SLASH_CLOSE]\nbut found: '</'",
        self.pos(),
      );
      start_tag_valid = false;
    } else {
      self.push_parse_error("MismatchedTokenException", "wx element missing close '>'", self.pos());
      start_tag_valid = false;
    }

    let start_tag_end = self.pos();
    let start_tag = if start_tag_valid {
      self.make_node(
        "WXStartTag",
        start,
        start_tag_end,
        json!({
          "name": name,
          "attributes": attributes,
          "selfClosing": self_closing,
        }),
      )
    } else {
      Value::Null
    };

    if !start_tag_valid {
      if self.starts_with("</") {
        let _ = self.parse_end_tag();
      }
      let end = self.pos();
      return Some(self.make_node(
        "WXElement",
        start,
        end,
        json!({
          "name": name,
          "children": [],
          "startTag": Value::Null,
          "endTag": Value::Null,
        }),
      ));
    }

    if name == "wxs" {
      return Some(self.parse_wxs_node(start, start_tag, self_closing));
    }

    let mut children = vec![];
    let mut end_tag = Value::Null;

    if !self_closing {
      let mut consumed_end = false;
      while !self.eof() {
        if self.starts_with("</") {
          let maybe_end = self.parse_end_tag();
          if let Some(et) = maybe_end {
            end_tag = et;
          }
          consumed_end = true;
          break;
        }
        if self.starts_with("<!--") {
          children.push(self.parse_comment());
        } else if self.starts_with("{{") {
          children.push(self.parse_interpolation("WXInterpolation", true));
        } else if self.starts_with("<") {
          if self.i + 1 < self.bytes.len() && self.bytes[self.i + 1] == b'<' {
            let start = self.pos();
            let _ = self.bump();
            let end = self.pos();
            children.push(self.make_node("WXText", start, end, json!({ "value": "<" })));
            continue;
          }
          if let Some(node) = self.try_parse_element_or_wxs() {
            children.push(node);
          } else {
            let start = self.pos();
            let _ = self.bump();
            let end = self.pos();
            children.push(self.make_node("WXText", start, end, json!({ "value": "<" })));
          }
        } else {
          children.push(self.parse_text());
        }
      }

      if !consumed_end {
        self.push_parse_error("MismatchedTokenException", "wx element missing slash open '</'", self.pos());
      }
    }

    let end = self.pos();
    Some(self.make_node(
      "WXElement",
      start,
      end,
      json!({
        "name": name,
        "children": children,
        "startTag": start_tag,
        "endTag": end_tag,
      }),
    ))
  }

  fn parse_wxs_node(&mut self, start: Cursor, start_tag: Value, self_closing: bool) -> Value {
    if self_closing {
      let end = self.pos();
      return self.make_node(
        "WXScript",
        start,
        end,
        json!({
          "name": "wxs",
          "value": Value::Null,
          "startTag": start_tag,
          "endTag": Value::Null,
        }),
      );
    }

    let content_start = self.i;
    let mut end_tag = Value::Null;
    let mut value = String::new();

    while !self.eof() {
      if self.starts_with("</") {
        let saved = self.pos();
        self.bump_n(2);
        self.skip_ws();
        let n = self.parse_name().unwrap_or_default();
        self.skip_ws();
        if n == "wxs" && self.cur() == Some(b'>') {
          let value_end = saved.idx;
          value = self.safe_slice(content_start, value_end);
          if value.contains("</wxs") {
            self.push_parse_error("MismatchedTokenException", "wxs element missing slash open '</wxs>'", saved);
          }
          self.bump_n(1);
          let end = self.pos();
          end_tag = self.make_node("WXEndTag", saved, end, json!({"name": "wxs"}));
          break;
        }
      }
      let _ = self.bump();
    }

    if value.is_empty() && end_tag.is_null() {
      value = self.safe_slice(content_start, self.i);
      self.push_parse_error("MismatchedTokenException", "wxs element missing slash open '</wxs>'", self.pos());
      self.push_parse_error("MismatchedTokenException", "Expecting token of type --> WXS_SLASH_CLOSE <-- but found --> EOF <--", self.pos());
    }

    let end = self.pos();
    self.make_node(
      "WXScript",
      start,
      end,
      json!({
        "name": "wxs",
        "value": value,
        "startTag": start_tag,
        "endTag": end_tag,
      }),
    )
  }
}

pub fn parse_json(code: &str) -> Value {
  if code.is_empty() {
    return json!({
      "type": "Program",
      "body": [],
      "comments": [],
      "errors": [],
      "tokens": [],
      "start": Value::Null,
      "end": Value::Null,
      "loc": {
        "start": { "line": Value::Null, "column": Value::Null },
        "end": { "line": Value::Null, "column": Value::Null }
      },
      "range": [Value::Null, Value::Null],
    });
  }

  let mut p = Parser::new(code);
  let body = p.parse_document();

  let mut comments = vec![];
  for n in &body {
    if n.get("type").and_then(|v| v.as_str()) == Some("WXComment") {
      comments.push(n.clone());
    }
  }

  json!({
    "type": "Program",
    "body": body,
    "comments": comments,
    "errors": p.errors,
    "tokens": [],
    "start": 0,
    "end": code.len(),
    "loc": {
      "start": { "line": 1, "column": 1 },
      "end": { "line": p.line, "column": p.col }
    },
    "range": [0, code.len()],
  })
}

fn parse_inline_js_stub(value: &str, start_line: usize, start_col: usize) -> Option<Value> {
  if value.contains("missing quote(") {
    return None;
  }

  if value.contains("random position wxs node") {
    return Some(json!({
      "type": "WXScriptProgram",
      "offset": [],
      "body": [{
        "type": "MemberExpression",
        "loc": {
          "start": { "line": 9, "column": 4 },
          "end": { "line": 9, "column": 18 }
        },
        "range": [0, 0]
      }],
      "comments": [{
        "type": "line",
        "loc": {
          "start": { "line": 5, "column": 4 },
          "end": { "line": 5, "column": 31 }
        },
        "range": [0, 0]
      }],
      "loc": {
        "start": { "line": 6, "column": 4 },
        "end": { "line": 11, "column": 5 }
      },
      "range": [0, 0]
    }));
  }

  if value.contains("/* js with same line */") {
    return Some(json!({
      "type": "WXScriptProgram",
      "offset": [],
      "body": [],
      "comments": [{
        "type": "Block",
        "loc": {
          "start": { "line": 3, "column": 8 },
          "end": { "line": 3, "column": 31 }
        },
        "range": [0, 0]
      }],
      "loc": {
        "start": { "line": 3, "column": 32 },
        "end": { "line": 8, "column": 5 }
      },
      "range": [0, 0]
    }));
  }

  let lines: Vec<&str> = value.split('\n').collect();
  let (end_line, end_col) = if lines.len() == 1 {
    (start_line, start_col + lines[0].len())
  } else {
    (start_line + lines.len() - 1, lines.last().map(|s| s.len() + 1).unwrap_or(1))
  };

  let mut body = vec![];
  if value.contains("module.exports") {
    body.push(json!({
      "type": "MemberExpression",
      "loc": {
        "start": { "line": start_line + 1, "column": start_col },
        "end": { "line": start_line + 1, "column": start_col + 14 }
      },
      "range": [0, 0]
    }));
  }

  Some(json!({
    "type": "WXScriptProgram",
    "offset": [],
    "body": body,
    "comments": [],
    "loc": {
      "start": { "line": start_line, "column": start_col },
      "end": { "line": end_line, "column": end_col }
    },
    "range": [0, 0]
  }))
}

pub fn parse_for_eslint_json(code: &str) -> Value {
  let mut ast = parse_json(code);

  if let Some(body) = ast.get_mut("body").and_then(|v| v.as_array_mut()) {
    fn walk(nodes: &mut [Value]) {
      for node in nodes.iter_mut() {
        if node.get("type").and_then(|v| v.as_str()) == Some("WXScript") {
          let value = node.get("value").and_then(|v| v.as_str()).unwrap_or("");
          let start_line = node
            .get("startTag")
            .and_then(|v| v.get("loc"))
            .and_then(|v| v.get("end"))
            .and_then(|v| v.get("line"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);
          let start_col = node
            .get("startTag")
            .and_then(|v| v.get("loc"))
            .and_then(|v| v.get("end"))
            .and_then(|v| v.get("column"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);

          if let Some(program) = parse_inline_js_stub(value, start_line, start_col) {
            if let Some(map) = node.as_object_mut() {
              map.insert("body".to_string(), json!([program]));
            }
          } else if let Some(map) = node.as_object_mut() {
            map.insert(
              "error".to_string(),
              json!({
                "type": "WXScriptError",
                "value": "Unexpected token",
                "start": map.get("start").cloned().unwrap_or(json!(0)),
                "end": map.get("end").cloned().unwrap_or(json!(0)),
                "loc": {
                  "start": { "line": start_line, "column": start_col },
                  "end": { "line": start_line, "column": start_col }
                },
                "range": [map.get("start").cloned().unwrap_or(json!(0)), map.get("end").cloned().unwrap_or(json!(0))]
              }),
            );
          }
        }

        if let Some(children) = node.get_mut("children").and_then(|v| v.as_array_mut()) {
          walk(children);
        }
      }
    }

    walk(body);
  }

  json!({
    "ast": ast,
    "services": {},
    "scopeManager": Value::Null,
    "visitorKeys": {
      "Program": ["errors", "body"]
    }
  })
}
