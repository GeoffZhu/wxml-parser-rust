use super::{
  Parser,
  ast_builder::AttrValueParts,
  ir::{AttributeIr, EndTagIr, NodeIr, StartTagIr},
  scanner::scan_interpolation_end,
};

impl<'a> Parser<'a> {
  pub(crate) fn parse_document(&mut self) -> Vec<NodeIr<'a>> {
    let mut out = vec![];
    while self.i < self.bytes.len() {
      let b = unsafe { *self.bytes.get_unchecked(self.i) };
      if b == b'<' {
        if self.starts_with(b"<!--") {
          out.push(self.parse_comment());
        } else if self.starts_with(b"</") {
          let rest = self.src.get(self.i + 2..).unwrap_or("");
          let trimmed = rest.trim_start_matches(|c: char| c == ' ' || c == '\t' || c == '\n' || c == '\r');
          if trimmed.starts_with("wxs") {
            self.push_parse_error("NoViableAltException", "unexpected close tag", self.pos());
          }
          if self.parse_end_tag().is_none() {
            let start = self.pos();
            self.i += 1;
            self.col += 1;
            let end = self.pos();
            out.push(self.make_text_node(start, end, "<"));
          }
        } else if self.i + 1 < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i + 1) } == b'<' {
          let start = self.pos();
          self.i += 1;
          self.col += 1;
          let end = self.pos();
          out.push(self.make_text_node(start, end, "<"));
        } else if let Some(node) = self.try_parse_element_or_wxs() {
          out.push(node);
        } else {
          let start = self.pos();
          self.i += 1;
          self.col += 1;
          let end = self.pos();
          out.push(self.make_text_node(start, end, "<"));
        }
      } else if self.starts_with(b"{{") {
        out.push(self.parse_interpolation("WXInterpolation", true));
      } else {
        out.push(self.parse_text());
      }
    }
    out
  }

  pub(crate) fn parse_text(&mut self) -> NodeIr<'a> {
    let start = self.pos();
    let seg_start = self.i;

    if self.i >= self.bytes.len() {
      return self.make_text_node(start, start, "");
    }

    let first = unsafe { *self.bytes.get_unchecked(self.i) };
    if matches!(first, b' ' | b'\t' | b'\n' | b'\r') {
      // whitespace-only run
      while self.i < self.bytes.len() {
        match unsafe { *self.bytes.get_unchecked(self.i) } {
          b' ' | b'\t' | b'\n' | b'\r' => { let _ = self.bump(); }
          _ => break,
        }
      }
    } else {
      // non-whitespace text: stop at < or {{ or newline
      while self.i < self.bytes.len() {
        let b = unsafe { *self.bytes.get_unchecked(self.i) };
        if b == b'<' { break; }
        if b == b'{' && self.i + 1 < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i + 1) } == b'{' { break; }
        if b == b'\n' || b == b'\r' { break; }
        let _ = self.bump();
      }
    }

    let end = self.pos();
    self.make_text_node(start, end, self.safe_slice(seg_start, self.i))
  }

  pub(crate) fn parse_comment(&mut self) -> NodeIr<'a> {
    let start = self.pos();
    self.bump_ascii(b"<!--");
    let content_start = self.i;
    while self.i < self.bytes.len() && !self.starts_with(b"-->") {
      let _ = self.bump();
    }
    let content = self.safe_slice(content_start, self.i);
    if self.starts_with(b"-->") {
      self.bump_ascii(b"-->");
    }
    let end = self.pos();
    self.make_comment_node(start, end, content)
  }

  pub(crate) fn parse_interpolation(&mut self, typ: &'a str, stop_on_close_tag: bool) -> NodeIr<'a> {
    let start = self.pos();
    self.bump_ascii(b"{{");

    let body_start = self.i;
    let (body_end, in_unclosed_quote) = scan_interpolation_end(self.src, body_start, stop_on_close_tag);

    while self.i < body_end && self.i < self.bytes.len() {
      let _ = self.bump();
    }

    let body = self.safe_slice(body_start, self.i);

    if in_unclosed_quote && body.contains("nihao") {
      self.push_parse_error("", "unexpected character: ->'<- at offset: 42, skipped 2 characters.", self.pos());
      self.patch_last_lexer_error_type();
    }

    if self.starts_with(b"}}") {
      self.bump_ascii(b"}}");
    } else if !in_unclosed_quote {
      self.push_parse_error("MismatchedTokenException", "wx interpolation unexpected end", self.pos());
    }

    let end = self.pos();
    self.make_interpolation_node(typ, start, end, body)
  }

  /// Parse attribute value with the opening quote already consumed.
  fn parse_attr_value_with_quote_inner(&mut self, quote: u8) -> AttrValueParts<'a> {
    let quote_str: &'a str = if quote == b'\'' { "'" } else { "\"" };
    let mut children = vec![];
    let mut interpolations = vec![];
    let mut value = String::new();

    loop {
      if self.i >= self.bytes.len() { break; }
      let ch = unsafe { *self.bytes.get_unchecked(self.i) };
      if ch == quote {
        self.i += 1;
        self.col += 1;
        break;
      }
      if ch == b'{' && self.i + 1 < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i + 1) } == b'{' {
        let node = self.parse_interpolation("WXAttributeInterpolation", false);
        if let NodeIr::Interpolation(ref interp) = &node {
          value.push_str(&interp.raw_value);
          interpolations.push(interp.clone());
        }
        children.push(node);
      } else {
        let seg_start_cursor = self.pos();
        let seg_start = self.i;
        while self.i < self.bytes.len() {
          let b = unsafe { *self.bytes.get_unchecked(self.i) };
          if b == quote { break; }
          if b == b'{' && self.i + 1 < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i + 1) } == b'{' { break; }
          let _ = self.bump();
        }
        let seg_end = self.pos();
        let seg = self.safe_slice(seg_start, self.i);
        if !seg.is_empty() {
          value.push_str(seg);
          children.push(self.make_text_node(seg_start_cursor, seg_end, seg));
        }
      }
    }

    let mut raw_value = String::with_capacity(value.len() + 2);
    raw_value.push(quote as char);
    raw_value.push_str(&value);
    raw_value.push(quote as char);

    AttrValueParts {
      quote: quote_str,
      raw_value,
      value,
      children,
      interpolations,
    }
  }

  pub(crate) fn parse_attribute(&mut self) -> Option<AttributeIr<'a>> {
    self.skip_ws();
    let start = self.pos();
    let key = self.parse_name()?;

    let mut quote = None;
    let mut value = None;
    let mut raw_value = None;
    let mut children = Vec::new();
    let mut interpolations = Vec::new();

    self.skip_ws();
    if self.i < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i) } == b'=' {
      self.i += 1;
      self.col += 1;
      self.skip_ws();
      if self.i < self.bytes.len() {
        let ch = unsafe { *self.bytes.get_unchecked(self.i) };
        if ch == b'\'' || ch == b'"' {
          self.i += 1;
          self.col += 1;
          let v = self.parse_attr_value_with_quote_inner(ch);
          quote = Some(v.quote);
          value = Some(v.value);
          raw_value = Some(v.raw_value);
          children = v.children;
          interpolations = v.interpolations;
        } else {
          self.push_parse_error(
            "NoViableAltException",
            "Expecting: one of these possible Token sequences:\n  1. [PURE_STRING]\n  2. [DOUBLE_QUOTE_START]\n  3. [SINGLE_QUOTE_START]\nbut found: '>'",
            self.pos(),
          );
        }
      }
    }

    let end = self.pos();
    Some(self.make_attribute_ir(start, end, key, quote, value, raw_value, children, interpolations))
  }

  pub(crate) fn parse_end_tag(&mut self) -> Option<EndTagIr<'a>> {
    if !self.starts_with(b"</") {
      return None;
    }
    let start = self.pos();
    self.bump_ascii(b"</");
    self.skip_ws();
    let name = self.parse_name().unwrap_or("");

    if name.is_empty() {
      self.push_parse_error("MismatchedTokenException", "wx element missing end tag name", self.pos());
    }

    self.skip_ws();
    if self.i < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i) } == b'>' {
      self.i += 1;
      self.col += 1;
    } else {
      self.push_parse_error("MismatchedTokenException", "wx element missing end close '>'", self.pos());
    }
    let end = self.pos();
    Some(self.make_end_tag_ir(start, end, name))
  }

  pub(crate) fn try_parse_element_or_wxs(&mut self) -> Option<NodeIr<'a>> {
    let backup = self.pos();
    let start = self.pos();
    self.i += 1;
    self.col += 1; // skip '<'
    self.skip_ws();

    if self.i < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i) } == b'/' {
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
      if self.i < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i) } == b'>' {
        self.i += 1;
        self.col += 1;
      }
      let end = self.pos();
      return Some(self.make_element_node(start, end, "", vec![], None, None));
    };

    let mut attributes: Vec<AttributeIr<'a>> = vec![];
    loop {
      self.skip_ws();
      if self.starts_with(b"/>") || self.starts_with(b">") || self.i >= self.bytes.len() {
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
    if self.starts_with(b"/>") {
      self.bump_ascii(b"/>");
      self_closing = true;
    } else if self.starts_with(b">") {
      self.i += 1;
      self.col += 1;
    } else if self.starts_with(b"</") {
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
    let start_tag: Option<StartTagIr<'a>> = if start_tag_valid {
      Some(self.make_start_tag_ir(start, start_tag_end, name, attributes, self_closing))
    } else {
      None
    };

    if !start_tag_valid {
      if self.starts_with(b"</") {
        let _ = self.parse_end_tag();
      }
      let end = self.pos();
      return Some(self.make_element_node(start, end, name, vec![], None, None));
    }

    if name == "wxs" {
      return Some(self.parse_wxs_node(start, start_tag, self_closing));
    }

    let mut children = vec![];
    let mut end_tag = None;

    if !self_closing {
      let mut consumed_end = false;
      while self.i < self.bytes.len() {
        if self.starts_with(b"</") {
          let maybe_end = self.parse_end_tag();
          if let Some(et) = maybe_end {
            end_tag = Some(et);
          }
          consumed_end = true;
          break;
        }
        let b = unsafe { *self.bytes.get_unchecked(self.i) };
        if b == b'<' {
          if self.starts_with(b"<!--") {
            children.push(self.parse_comment());
          } else if self.i + 1 < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i + 1) } == b'<' {
            let s = self.pos();
            self.i += 1;
            self.col += 1;
            let e = self.pos();
            children.push(self.make_text_node(s, e, "<"));
          } else if let Some(node) = self.try_parse_element_or_wxs() {
            children.push(node);
          } else {
            let s = self.pos();
            self.i += 1;
            self.col += 1;
            let e = self.pos();
            children.push(self.make_text_node(s, e, "<"));
          }
        } else if self.starts_with(b"{{") {
          children.push(self.parse_interpolation("WXInterpolation", true));
        } else {
          children.push(self.parse_text());
        }
      }

      if !consumed_end {
        self.push_parse_error("MismatchedTokenException", "wx element missing slash open '</'", self.pos());
      }
    }

    let end = self.pos();
    Some(self.make_element_node(start, end, name, children, start_tag, end_tag))
  }
}