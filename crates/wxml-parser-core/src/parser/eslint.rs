use serde_json::Value;

use super::{parse_program_with_mode, serialize::serialize_eslint};

pub fn parse_for_eslint_json(code: &str) -> Value {
  serialize_eslint(parse_program_with_mode(code, true))
}