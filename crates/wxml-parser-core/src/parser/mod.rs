use serde_json::Value;

mod ast_builder;
mod cursor;
mod error;
mod eslint;
mod scanner;
mod ir;
mod script;
mod serialize;
mod syntax;

use cursor::Cursor;
use ir::{NodeIr, ParsedProgram};
pub(crate) use serialize::serialize_program;

pub use eslint::parse_for_eslint_json;

pub(crate) struct Parser<'a> {
  pub(crate) src: &'a str,
  pub(crate) bytes: &'a [u8],
  pub(crate) i: usize,
  pub(crate) line: usize,
  pub(crate) col: usize,
  pub(crate) errors: Vec<ir::ParseErrorIr>,
  pub(crate) emit_script_program: bool,
}

impl<'a> Parser<'a> {
  fn new_with_mode(src: &'a str, emit_script_program: bool) -> Self {
    Self {
      src,
      bytes: src.as_bytes(),
      i: 0,
      line: 1,
      col: 1,
      errors: vec![],
      emit_script_program,
    }
  }

  #[inline(always)]
  pub(crate) fn is_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b':' | b'_' | b'-' | b'.')
  }

  pub(crate) fn parse_name(&mut self) -> Option<&'a str> {
    let start = self.i;
    self.skip_ascii_while(Self::is_name_char);
    if self.i > start {
      Some(self.safe_slice(start, self.i))
    } else {
      None
    }
  }
}

pub fn parse_json(code: &str) -> Value {
  parse_json_with_mode(code, false)
}

pub(crate) fn parse_json_with_mode(code: &str, emit_script_program: bool) -> Value {
  serialize_program(parse_program_with_mode(code, emit_script_program))
}

pub(crate) fn parse_program_with_mode(code: &str, emit_script_program: bool) -> ParsedProgram<'_> {
  if code.is_empty() {
    return ParsedProgram {
      body: vec![],
      comment_indices: vec![],
      errors: vec![],
      end_line: 0,
      end_col: 0,
      code_len: 0,
    };
  }

  let mut p = Parser::new_with_mode(code, emit_script_program);
  let body = p.parse_document();

  let comment_indices = body
    .iter()
    .enumerate()
    .filter_map(|(idx, n)| {
      if matches!(n, NodeIr::Comment { .. }) {
        Some(idx)
      } else {
        None
      }
    })
    .collect();

  ParsedProgram {
    body,
    comment_indices,
    errors: p.errors,
    end_line: p.line,
    end_col: p.col,
    code_len: code.len(),
  }
}

#[allow(dead_code)]
fn _keep_cursor(_c: Cursor) {}
