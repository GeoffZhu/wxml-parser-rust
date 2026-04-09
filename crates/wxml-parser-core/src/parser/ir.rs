#[derive(Clone, Copy, Debug)]
pub(crate) struct Span {
  pub(crate) start_idx: usize,
  pub(crate) end_idx: usize,
  pub(crate) start_line: usize,
  pub(crate) start_col: usize,
  pub(crate) end_line: usize,
  pub(crate) end_col: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ParseErrorIr {
  pub(crate) typ: &'static str,
  pub(crate) raw_type: Option<&'static str>,
  pub(crate) value: String,
  pub(crate) span: Span,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LocIr {
  pub(crate) start_line: usize,
  pub(crate) start_col: usize,
  pub(crate) end_line: usize,
  pub(crate) end_col: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct InterpolationIr<'a> {
  pub(crate) typ: &'a str,
  pub(crate) raw_value: String,
  pub(crate) value: &'a str,
  pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) struct AttributeIr<'a> {
  pub(crate) key: &'a str,
  pub(crate) quote: Option<&'a str>,
  pub(crate) value: Option<String>,
  pub(crate) raw_value: Option<String>,
  pub(crate) children: Vec<NodeIr<'a>>,
  pub(crate) interpolations: Vec<InterpolationIr<'a>>,
  pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) struct StartTagIr<'a> {
  pub(crate) name: &'a str,
  pub(crate) attributes: Vec<AttributeIr<'a>>,
  pub(crate) self_closing: bool,
  pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) struct EndTagIr<'a> {
  pub(crate) name: &'a str,
  pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) struct ScriptErrorIr {
  pub(crate) value: String,
  pub(crate) span: Span,
  pub(crate) line: usize,
  pub(crate) column: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ScriptNodeIr<'a> {
  pub(crate) name: &'static str,
  pub(crate) value: Option<&'a str>,
  pub(crate) start_tag: Option<StartTagIr<'a>>,
  pub(crate) end_tag: Option<EndTagIr<'a>>,
  pub(crate) body: Option<ScriptProgramIr>,
  pub(crate) error: Option<ScriptErrorIr>,
  pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) enum NodeIr<'a> {
  Text { value: &'a str, span: Span },
  Comment { value: &'a str, span: Span },
  Interpolation(InterpolationIr<'a>),
  Element {
    name: &'a str,
    children: Vec<NodeIr<'a>>,
    start_tag: Option<StartTagIr<'a>>,
    end_tag: Option<EndTagIr<'a>>,
    span: Span,
  },
  Script(ScriptNodeIr<'a>),
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedProgram<'a> {
  pub(crate) body: Vec<NodeIr<'a>>,
  pub(crate) comment_indices: Vec<usize>,
  pub(crate) errors: Vec<ParseErrorIr>,
  pub(crate) end_line: usize,
  pub(crate) end_col: usize,
  pub(crate) code_len: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ScriptLocIr {
  pub(crate) start_line: usize,
  pub(crate) start_col: usize,
  pub(crate) end_line: usize,
  pub(crate) end_col: usize,
}

#[derive(Clone, Debug)]
pub(crate) enum ScriptBodyIr {
  MemberExpression { loc: ScriptLocIr },
}

#[derive(Clone, Debug)]
pub(crate) struct ScriptCommentIr {
  pub(crate) typ: String,
  pub(crate) loc: ScriptLocIr,
}

#[derive(Clone, Debug)]
pub(crate) struct ScriptProgramIr {
  pub(crate) body: Vec<ScriptBodyIr>,
  pub(crate) comments: Vec<ScriptCommentIr>,
  pub(crate) loc: ScriptLocIr,
}
