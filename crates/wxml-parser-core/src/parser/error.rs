use super::{
  Parser,
  cursor::Cursor,
  ir::{ParseErrorIr, Span},
};

impl<'a> Parser<'a> {
  pub(crate) fn push_parse_error(&mut self, raw_type: &'static str, value: &'static str, at: Cursor) {
    self.errors.push(ParseErrorIr {
      typ: "WXParseError",
      raw_type: Some(raw_type),
      value: value.to_string(),
      span: Span {
        start_idx: at.idx,
        end_idx: at.idx,
        start_line: at.line,
        start_col: at.col,
        end_line: at.line,
        end_col: at.col,
      },
    });
  }

  pub(crate) fn patch_last_lexer_error_type(&mut self) {
    if let Some(last) = self.errors.last_mut() {
      last.typ = "WXLexerError";
      last.raw_type = None;
    }
  }
}
