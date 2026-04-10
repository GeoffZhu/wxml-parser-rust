mod parser;

pub use parser::{
  parse_for_eslint_json,
  parse_for_eslint_json_string,
  parse_json,
  parse_json_string,
  parse_program_with_mode,
};

pub mod ir {
  pub use crate::parser::ir::*;
}
