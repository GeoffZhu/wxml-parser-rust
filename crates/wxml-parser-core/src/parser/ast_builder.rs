use super::{
  Parser,
  cursor::Cursor,
  ir::{
    AttributeIr,
    EndTagIr,
    InterpolationIr,
    LocIr,
    NodeIr,
    ScriptErrorIr,
    ScriptNodeIr,
    Span,
    StartTagIr,
  },
};

pub(crate) struct AttrValueParts<'a> {
  pub(crate) quote: &'a str,
  pub(crate) value: String,
  pub(crate) raw_value: String,
  pub(crate) children: Vec<NodeIr<'a>>,
  pub(crate) interpolations: Vec<InterpolationIr<'a>>,
}

impl<'a> Parser<'a> {
  #[inline(always)]
  pub(crate) fn make_span(&self, start: Cursor, end: Cursor) -> Span {
    Span {
      start_idx: start.idx,
      end_idx: end.idx,
      start_line: start.line,
      start_col: start.col,
      end_line: end.line,
      end_col: end.col,
    }
  }

  pub(crate) fn span_to_loc(&self, span: &Span) -> LocIr {
    let mut end_col = span.end_col;
    if span.start_idx != span.end_idx {
      end_col = end_col.saturating_sub(1);
    }
    LocIr {
      start_line: span.start_line,
      start_col: span.start_col,
      end_line: span.end_line,
      end_col,
    }
  }

  #[inline(always)]
  pub(crate) fn make_text_node(&self, start: Cursor, end: Cursor, value: &'a str) -> NodeIr<'a> {
    NodeIr::Text {
      value,
      span: self.make_span(start, end),
    }
  }

  #[inline(always)]
  pub(crate) fn make_comment_node(&self, start: Cursor, end: Cursor, value: &'a str) -> NodeIr<'a> {
    NodeIr::Comment {
      value,
      span: self.make_span(start, end),
    }
  }

  pub(crate) fn make_interpolation_node(&self, typ: &'a str, start: Cursor, end: Cursor, value: &'a str) -> NodeIr<'a> {
    NodeIr::Interpolation(InterpolationIr {
      typ,
      raw_value: format!("{{{{{}}}}}", value),
      value,
      span: self.make_span(start, end),
    })
  }

  pub(crate) fn make_end_tag_ir(&self, start: Cursor, end: Cursor, name: &'a str) -> EndTagIr<'a> {
    EndTagIr {
      name,
      span: self.make_span(start, end),
    }
  }

  pub(crate) fn make_attribute_ir(
    &self,
    start: Cursor,
    end: Cursor,
    key: &'a str,
    quote: Option<&'a str>,
    value: Option<String>,
    raw_value: Option<String>,
    children: Vec<NodeIr<'a>>,
    interpolations: Vec<InterpolationIr<'a>>,
  ) -> AttributeIr<'a> {
    AttributeIr {
      key,
      quote,
      value,
      raw_value,
      children,
      interpolations,
      span: self.make_span(start, end),
    }
  }

  pub(crate) fn make_start_tag_ir(
    &self,
    start: Cursor,
    end: Cursor,
    name: &'a str,
    attributes: Vec<AttributeIr<'a>>,
    self_closing: bool,
  ) -> StartTagIr<'a> {
    StartTagIr {
      name,
      attributes,
      self_closing,
      span: self.make_span(start, end),
    }
  }

  pub(crate) fn make_element_node(
    &self,
    start: Cursor,
    end: Cursor,
    name: &'a str,
    children: Vec<NodeIr<'a>>,
    start_tag: Option<StartTagIr<'a>>,
    end_tag: Option<EndTagIr<'a>>,
  ) -> NodeIr<'a> {
    NodeIr::Element {
      name,
      children,
      start_tag,
      end_tag,
      span: self.make_span(start, end),
    }
  }

  pub(crate) fn make_script_node(
    &self,
    start: Cursor,
    end: Cursor,
    value: Option<&'a str>,
    start_tag: Option<StartTagIr<'a>>,
    end_tag: Option<EndTagIr<'a>>,
  ) -> NodeIr<'a> {
    NodeIr::Script(ScriptNodeIr {
      name: "wxs",
      value,
      start_tag,
      end_tag,
      body: None,
      error: None,
      span: self.make_span(start, end),
    })
  }

  pub(crate) fn make_script_error_ir(&self, span: Span, value: String, line: usize, column: usize) -> ScriptErrorIr {
    ScriptErrorIr {
      value,
      span,
      line,
      column,
    }
  }
}
