use serde_json::Value;

use super::{
  parse_program_with_mode,
  serialize::{serialize_eslint, serialize_eslint_to_string},
};

pub fn parse_for_eslint_json(code: &str) -> Value {
  serialize_eslint(parse_program_with_mode(code, true))
}

/// Parse WXML for ESLint and return the result as a JSON string directly,
/// avoiding the expensive `serde_json::from_str` → Value step.
pub fn parse_for_eslint_json_string(code: &str) -> String {
  serialize_eslint_to_string(&parse_program_with_mode(code, true))
}