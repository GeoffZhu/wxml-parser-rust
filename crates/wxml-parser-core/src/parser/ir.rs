#[derive(Clone, Copy, Debug)]
pub struct Span {
  pub start_idx: usize,
  pub end_idx: usize,
  pub start_line: usize,
  pub start_col: usize,
  pub end_line: usize,
  pub end_col: usize,
}

#[derive(Clone, Debug)]
pub struct ParseErrorIr {
  pub typ: &'static str,
  pub raw_type: Option<&'static str>,
  pub value: String,
  pub span: Span,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct LocIr {
  pub start_line: usize,
  pub start_col: usize,
  pub end_line: usize,
  pub end_col: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct InterpolationIr<'a> {
  pub typ: &'a str,
  pub raw_value: &'a str,
  pub value: &'a str,
  pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AttributeIr<'a> {
  pub key: &'a str,
  pub quote: Option<&'a str>,
  pub value: Option<String>,
  pub raw_value: Option<String>,
  pub children: Vec<NodeIr<'a>>,
  pub interpolations: Vec<InterpolationIr<'a>>,
  pub span: Span,
}

#[derive(Clone, Debug)]
pub struct StartTagIr<'a> {
  pub name: &'a str,
  pub attributes: Vec<AttributeIr<'a>>,
  pub self_closing: bool,
  pub span: Span,
}

#[derive(Clone, Debug)]
pub struct EndTagIr<'a> {
  pub name: &'a str,
  pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ScriptErrorIr {
  pub value: String,
  pub span: Span,
  pub line: usize,
  pub column: usize,
}

#[derive(Clone, Debug)]
pub struct ScriptNodeIr<'a> {
  pub name: &'static str,
  pub value: Option<&'a str>,
  pub start_tag: Option<StartTagIr<'a>>,
  pub end_tag: Option<EndTagIr<'a>>,
  pub body: Option<ScriptProgramIr>,
  pub error: Option<ScriptErrorIr>,
  pub span: Span,
}

#[derive(Clone, Debug)]
pub enum NodeIr<'a> {
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
pub struct ParsedProgram<'a> {
  pub body: Vec<NodeIr<'a>>,
  pub comment_indices: Vec<usize>,
  pub errors: Vec<ParseErrorIr>,
  pub end_line: usize,
  pub end_col: usize,
  pub code_len: usize,
}

#[derive(Clone, Debug)]
pub struct ScriptLocIr {
  pub start_line: usize,
  pub start_col: usize,
  pub end_line: usize,
  pub end_col: usize,
}

#[derive(Clone, Debug)]
pub enum ScriptBodyIr {
  MemberExpression { loc: ScriptLocIr },
}

#[derive(Clone, Debug)]
pub struct ScriptCommentIr {
  pub typ: String,
  pub loc: ScriptLocIr,
}

#[derive(Clone, Debug)]
pub struct ScriptProgramIr {
  pub body: Vec<ScriptBodyIr>,
  pub comments: Vec<ScriptCommentIr>,
  pub loc: ScriptLocIr,
}
