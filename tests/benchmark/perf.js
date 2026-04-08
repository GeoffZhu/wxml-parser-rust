const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const rustPackageRoot = path.resolve(__dirname, "..", "..");
const fixturePath = path.join(rustPackageRoot, "tests", "fixtures", "bench", "complex-mixed-large.wxml");

const napiParser = require("../..");
const jsParser = require("@wxml/parser");

const warmup = 4;
const rounds = 8;
const iterationsByMode = {
  parse: 200,
  parseForESLint: 120,
};

function readFixture() {
  return fs.readFileSync(fixturePath, { encoding: "utf8" });
}

function summarizeParse(result) {
  return {
    type: result?.type ?? "",
    bodyLength: Array.isArray(result?.body) ? result.body.length : 0,
    errorsLength: Array.isArray(result?.errors) ? result.errors.length : 0,
  };
}

function summarizeParseForESLint(result) {
  return {
    type: result?.ast?.type ?? "",
    bodyLength: Array.isArray(result?.ast?.body) ? result.ast.body.length : 0,
    errorsLength: Array.isArray(result?.ast?.errors) ? result.ast.errors.length : 0,
    visitorProgramLength: Array.isArray(result?.visitorKeys?.Program)
      ? result.visitorKeys.Program.length
      : 0,
  };
}

function summarize(mode, result) {
  return mode === "parse" ? summarizeParse(result) : summarizeParseForESLint(result);
}

function benchmarkImplementation(name, fn, mode, code) {
  const iterations = iterationsByMode[mode];
  const summary = summarize(mode, fn(code));

  for (let i = 0; i < warmup; i += 1) {
    for (let j = 0; j < iterations; j += 1) {
      fn(code);
    }
  }

  const roundDurationsNs = [];
  for (let i = 0; i < rounds; i += 1) {
    const start = process.hrtime.bigint();
    for (let j = 0; j < iterations; j += 1) {
      fn(code);
    }
    const end = process.hrtime.bigint();
    roundDurationsNs.push(Number(end - start));
  }

  const avgNsPerRound = roundDurationsNs.reduce((sum, value) => sum + value, 0) / rounds;
  const avgNsPerOp = avgNsPerRound / iterations;
  const sorted = [...roundDurationsNs].sort((a, b) => a - b);
  const medianNsPerRound =
    sorted.length % 2 === 0
      ? (sorted[sorted.length / 2 - 1] + sorted[sorted.length / 2]) / 2
      : sorted[Math.floor(sorted.length / 2)];
  const medianNsPerOp = medianNsPerRound / iterations;

  return {
    implementation: name,
    summary,
    stats: {
      warmup,
      rounds,
      iterations,
      avgMsPerOp: avgNsPerOp / 1e6,
      medianMsPerOp: medianNsPerOp / 1e6,
      opsPerSec: 1e9 / avgNsPerOp,
    },
  };
}

function runRustBenchmark(mode) {
  const iterations = String(iterationsByMode[mode]);
  const output = execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--release",
      "-p",
      "wxml-parser",
      "--example",
      "bench_runner",
      "--",
      mode,
      fixturePath,
      String(warmup),
      String(rounds),
      iterations,
    ],
    {
      cwd: rustPackageRoot,
      encoding: "utf8",
    }
  );

  return JSON.parse(output.trim());
}

function stableStringify(value) {
  if (Array.isArray(value)) {
    return `[${value.map(stableStringify).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function assertSummaries(mode, results) {
  const [first, ...rest] = results;
  const baseline = stableStringify(first.summary);
  for (const result of rest) {
    if (stableStringify(result.summary) !== baseline) {
      throw new Error(
        `summary mismatch for ${mode}: ${first.implementation}=${baseline}, ${result.implementation}=${stableStringify(result.summary)}`
      );
    }
  }
}

function formatNumber(value, digits = 3) {
  return Number(value).toFixed(digits);
}

function renderTable(results) {
  const jsBaseline = results.find((item) => item.implementation === "js-parser");
  const baselineMedian = jsBaseline.stats.medianMsPerOp;
  const lines = [
    "| Implementation | Median ms/op | Avg ms/op | ops/sec | Relative |",
    "| --- | ---: | ---: | ---: | ---: |",
  ];

  for (const result of results) {
    lines.push(
      `| ${result.implementation} | ${formatNumber(result.stats.medianMsPerOp)} | ${formatNumber(result.stats.avgMsPerOp)} | ${formatNumber(result.stats.opsPerSec, 1)} | ${formatNumber(baselineMedian / result.stats.medianMsPerOp, 2)}x |`
    );
  }

  return lines.join("\n");
}

function renderSummaryBlock(mode, results) {
  return [
    `### ${mode}`,
    "",
    renderTable(results),
    "",
    "Summary check:",
    "",
    "```json",
    JSON.stringify(results[0].summary, null, 2),
    "```",
    "",
  ].join("\n");
}

function main() {
  const code = readFixture();
  const environment = {
    fixture: path.relative(rustPackageRoot, fixturePath),
    node: process.version,
    platform: process.platform,
    arch: process.arch,
    warmup,
    rounds,
    iterationsByMode,
  };

  console.log("# Benchmark Environment");
  console.log(JSON.stringify(environment, null, 2));
  console.log("");

  const suites = [
    ["parse", (source) => napiParser.parse(source), (source) => jsParser.parse(source)],
    ["parseForESLint", (source) => napiParser.parseForESLint(source), (source) => jsParser.parseForESLint(source)],
  ];

  for (const [mode, napiFn, jsFn] of suites) {
    const rustResult = runRustBenchmark(mode);
    const napiResult = benchmarkImplementation("napi", napiFn, mode, code);
    const jsResult = benchmarkImplementation("js-parser", jsFn, mode, code);
    const results = [rustResult, napiResult, jsResult];

    assertSummaries(mode, results);
    console.log(renderSummaryBlock(mode, results));
  }
}

main();
