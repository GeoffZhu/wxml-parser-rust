use serde_json::{json, Value};
use std::env;
use std::fs;
use std::time::Instant;
use wxml_parser_core::{parse_for_eslint_json, parse_json};

fn summarize_parse(result: &Value) -> Value {
  json!({
    "type": result.get("type").and_then(Value::as_str).unwrap_or(""),
    "bodyLength": result.get("body").and_then(Value::as_array).map_or(0, |v| v.len()),
    "errorsLength": result.get("errors").and_then(Value::as_array).map_or(0, |v| v.len()),
  })
}

fn summarize_parse_for_eslint(result: &Value) -> Value {
  let ast = result.get("ast").unwrap_or(result);
  let visitor_program = result
    .get("visitorKeys")
    .and_then(|v| v.get("Program"))
    .and_then(Value::as_array)
    .map_or(0, |v| v.len());

  json!({
    "type": ast.get("type").and_then(Value::as_str).unwrap_or(""),
    "bodyLength": ast.get("body").and_then(Value::as_array).map_or(0, |v| v.len()),
    "errorsLength": ast.get("errors").and_then(Value::as_array).map_or(0, |v| v.len()),
    "visitorProgramLength": visitor_program,
  })
}

fn parse_args() -> Result<(String, String, usize, usize, usize), String> {
  let mut args = env::args().skip(1);
  let mode = args.next().ok_or("missing mode")?;
  let fixture_path = args.next().ok_or("missing fixture path")?;
  let warmup = args
    .next()
    .ok_or("missing warmup")?
    .parse::<usize>()
    .map_err(|_| "invalid warmup")?;
  let rounds = args
    .next()
    .ok_or("missing rounds")?
    .parse::<usize>()
    .map_err(|_| "invalid rounds")?;
  let iterations = args
    .next()
    .ok_or("missing iterations")?
    .parse::<usize>()
    .map_err(|_| "invalid iterations")?;
  Ok((mode, fixture_path, warmup, rounds, iterations))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let (mode, fixture_path, warmup, rounds, iterations) = parse_args().map_err(|msg| {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, msg)
  })?;

  let code = fs::read_to_string(&fixture_path)?;

  let summarize: fn(&Value) -> Value = match mode.as_str() {
    "parse" => summarize_parse,
    "parseForESLint" => summarize_parse_for_eslint,
    _ => {
      return Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("unsupported mode: {}", mode),
      )
      .into())
    }
  };

  let runner: fn(&str) -> Value = match mode.as_str() {
    "parse" => parse_json,
    "parseForESLint" => parse_for_eslint_json,
    _ => unreachable!(),
  };

  let baseline = runner(&code);
  let summary = summarize(&baseline);

  for _ in 0..warmup {
    for _ in 0..iterations {
      let result = runner(&code);
      std::hint::black_box(result);
    }
  }

  let mut round_ns = Vec::with_capacity(rounds);
  for _ in 0..rounds {
    let start = Instant::now();
    for _ in 0..iterations {
      let result = runner(&code);
      std::hint::black_box(result);
    }
    round_ns.push(start.elapsed().as_nanos() as f64);
  }

  let total_ns: f64 = round_ns.iter().sum();
  let avg_ns_per_round = total_ns / rounds as f64;
  let avg_ns_per_op = avg_ns_per_round / iterations as f64;

  let mut sorted = round_ns.clone();
  sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
  let median_ns_per_round = if rounds % 2 == 0 {
    (sorted[rounds / 2 - 1] + sorted[rounds / 2]) / 2.0
  } else {
    sorted[rounds / 2]
  };
  let median_ns_per_op = median_ns_per_round / iterations as f64;
  let ops_per_sec = 1_000_000_000.0 / avg_ns_per_op;

  let output = json!({
    "implementation": "rust-core",
    "mode": mode,
    "fixturePath": fixture_path,
    "summary": summary,
    "stats": {
      "warmup": warmup,
      "rounds": rounds,
      "iterations": iterations,
      "avgMsPerOp": avg_ns_per_op / 1_000_000.0,
      "medianMsPerOp": median_ns_per_op / 1_000_000.0,
      "opsPerSec": ops_per_sec,
    }
  });

  println!("{}", serde_json::to_string(&output)?);
  Ok(())
}
