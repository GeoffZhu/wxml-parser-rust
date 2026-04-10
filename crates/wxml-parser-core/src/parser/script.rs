use oxc_allocator::Allocator;
use oxc_ast::ast::{
  Expression,
  Statement,
};
use oxc_parser::Parser as OxcParser;
use oxc_span::{GetSpan, SourceType};

use super::{
  Parser,
  cursor::Cursor,
  ir::{
    EndTagIr,
    NodeIr,
    ScriptBodyIr,
    ScriptCommentIr,
    ScriptLocIr,
    ScriptNodeIr,
    ScriptProgramIr,
    StartTagIr,
  },
};

impl<'a> Parser<'a> {
  pub(crate) fn enrich_script_node(&self, node: &mut ScriptNodeIr<'a>) {
    let value = match node.value {
      Some(v) => v.to_string(),
      None => String::new(),
    };
    let (base_line, base_col) = if let Some(start_tag) = &node.start_tag {
      // end_col is 1-indexed exclusive; convert to 0-indexed for ESTree
      (start_tag.span.end_line, start_tag.span.end_col.saturating_sub(1))
    } else {
      (1, 0)
    };

    match parse_inline_js_program(&value, base_line, base_col) {
      Ok(program) => {
        node.body = Some(program);
        node.error = None;
      }
      Err((message, line, column)) => {
        node.error = Some(self.make_script_error_ir(node.span, message, line, column));
        node.body = None;
      }
    }
  }

  pub(crate) fn parse_wxs_node(
    &mut self,
    start: Cursor,
    start_tag: Option<StartTagIr<'a>>,
    self_closing: bool,
  ) -> NodeIr<'a> {
    if self_closing {
      let end = self.pos();
      let mut node = self.make_script_node(start, end, None, start_tag, None);
      if self.emit_script_program {
        if let NodeIr::Script(script) = &mut node {
          self.enrich_script_node(script);
        }
      }
      return node;
    }

    let content_start = self.i;
    let mut end_tag: Option<EndTagIr<'a>> = None;
    let mut value: Option<&'a str> = None;

    while self.i < self.bytes.len() {
      if self.starts_with(b"</") {
        let saved = self.pos();
        self.bump_ascii(b"</");
        self.skip_ws();
        let n = self.parse_name().unwrap_or("");
        self.skip_ws();
        if n == "wxs" && self.i < self.bytes.len() && unsafe { *self.bytes.get_unchecked(self.i) } == b'>' {
          let value_end = saved.idx;
          value = Some(self.safe_slice(content_start, value_end));
          if let Some(v) = value {
            if v.contains("</wxs") {
              self.push_parse_error("MismatchedTokenException", "wxs element missing slash open '</wxs>'", saved);
            }
          }
          self.i += 1;
          self.col += 1;
          let end = self.pos();
          end_tag = Some(self.make_end_tag_ir(saved, end, "wxs"));
          break;
        }
        // Not a wxs close tag - restore position and continue scanning as wxs content
        self.i = saved.idx;
        self.line = saved.line;
        self.col = saved.col;
      }
      let _ = self.bump();
    }

    if value.is_none() && end_tag.is_none() {
      value = Some(self.safe_slice(content_start, self.i));
      self.push_parse_error("MismatchedTokenException", "wxs element missing slash open '</wxs>'", self.pos());
      self.push_parse_error("MismatchedTokenException", "Expecting token of type --> WXS_SLASH_CLOSE <-- but found --> EOF <--", self.pos());
    }

    let end = self.pos();
    let mut node = self.make_script_node(start, end, value, start_tag, end_tag);
    if self.emit_script_program {
      if let NodeIr::Script(script) = &mut node {
        self.enrich_script_node(script);
      }
    }
    node
  }
}

fn byte_to_line_col(s: &str, byte_pos: usize) -> (usize, usize) {
  let mut line = 1usize;
  let mut col = 0usize;
  for (i, ch) in s.char_indices() {
    if i >= byte_pos {
      break;
    }
    if ch == '\n' {
      line += 1;
      col = 0;
    } else {
      col += 1;
    }
  }
  (line, col)
}

fn to_abs_loc(base_line: usize, base_col: usize, rel_line: usize, rel_col: usize) -> (usize, usize) {
  if rel_line <= 1 {
    (base_line, base_col + rel_col)
  } else {
    (base_line + rel_line - 1, rel_col)
  }
}

fn collect_member_expression_spans_from_expr(expr: &Expression<'_>, out: &mut Vec<(u32, u32)>) {
  if let Some(me) = expr.as_member_expression() {
    let span = me.span();
    out.push((span.start, span.end));
    return;
  }

  if let Expression::CallExpression(call) = expr {
    collect_member_expression_spans_from_expr(&call.callee, out);
    for arg in &call.arguments {
      if let Some(e) = arg.as_expression() {
        collect_member_expression_spans_from_expr(e, out);
      }
    }
    return;
  }

  if let Expression::AssignmentExpression(assign) = expr {
    if let Some(me) = assign.left.as_member_expression() {
      let span = me.span();
      out.push((span.start, span.end));
    }
    collect_member_expression_spans_from_expr(&assign.right, out);
  }
}

fn parse_inline_js_program(
  value: &str,
  base_line: usize,
  base_col: usize,
) -> Result<ScriptProgramIr, (String, usize, usize)> {
  if value.is_empty() {
    return Ok(ScriptProgramIr {
      body: vec![],
      comments: vec![],
      loc: ScriptLocIr {
        start_line: base_line,
        start_col: base_col,
        end_line: base_line,
        end_col: base_col,
      },
    });
  }

  let allocator = Allocator::default();
  let source_type = SourceType::default();
  let parsed = OxcParser::new(&allocator, value, source_type).parse();

  if !parsed.errors.is_empty() {
    return Err(("Unexpected token".to_string(), base_line, base_col));
  }

  let mut body = vec![];
  let mut member_spans = vec![];

  for stmt in &parsed.program.body {
    if let Statement::ExpressionStatement(es) = stmt {
      collect_member_expression_spans_from_expr(&es.expression, &mut member_spans);
    }
  }

  for (s, e) in member_spans {
    let (sl_rel, sc_rel) = byte_to_line_col(value, s as usize);
    let (el_rel, ec_rel) = byte_to_line_col(value, e as usize);
    let (sl, sc) = to_abs_loc(base_line, base_col, sl_rel, sc_rel);
    let (el, ec) = to_abs_loc(base_line, base_col, el_rel, ec_rel);
    body.push(ScriptBodyIr::MemberExpression {
      loc: ScriptLocIr {
        start_line: sl,
        start_col: sc,
        end_line: el,
        end_col: ec,
      },
    });
  }

  let mut comments = vec![];
  for c in parsed.program.comments.iter() {
    let span = c.span;
    let (sl_rel, sc_rel) = byte_to_line_col(value, span.start as usize);
    let (el_rel, ec_rel) = byte_to_line_col(value, span.end as usize);
    let (sl, sc) = to_abs_loc(base_line, base_col, sl_rel, sc_rel);
    let (el, ec) = to_abs_loc(base_line, base_col, el_rel, ec_rel);

    let typ = if c.is_line() { "Line" } else { "Block" }.to_string();

    comments.push(ScriptCommentIr {
      typ,
      loc: ScriptLocIr {
        start_line: sl,
        start_col: sc,
        end_line: el,
        end_col: ec,
      },
    });
  }

  // WXScriptProgram loc spans from first statement start to last statement end
  let (prog_start_line, prog_start_col) = if let Some(first) = parsed.program.body.first() {
    let span = first.span();
    let (rl, rc) = byte_to_line_col(value, span.start as usize);
    to_abs_loc(base_line, base_col, rl, rc)
  } else {
    (base_line, base_col)
  };

  let (prog_end_line, prog_end_col) = if let Some(last) = parsed.program.body.last() {
    let span = last.span();
    let (rl, rc) = byte_to_line_col(value, span.end as usize);
    to_abs_loc(base_line, base_col, rl, rc)
  } else {
    (base_line, base_col)
  };

  Ok(ScriptProgramIr {
    body,
    comments,
    loc: ScriptLocIr {
      start_line: prog_start_line,
      start_col: prog_start_col,
      end_line: prog_end_line,
      end_col: prog_end_col,
    },
  })
}