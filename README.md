# wxml-parser-rust

`wxml-parser-rust` 是 `@wxml/parser` 的 Rust + `napi-rs` 实现，提供：

- Rust 核心解析库（`wxml-parser`）
- Node.js API（`parse` / `parseForESLint`）

## 安装依赖

```bash
npm install
```

## 构建

### JS/Node 侧（napi 模块）

```bash
npm run build:debug
# 或发布构建
npm run build
```

### 测试

```bash
npm test
```

***

## JS 使用方法

```js
const { parse, parseForESLint } = require('wxml-parser-rust')

const code = `<view wx:if="{{ok}}">hello</view>`

const ast = parse(code)
console.log(ast.type) // Program

const eslintAst = parseForESLint(code)
console.log(eslintAst.ast.type) // Program
```

***

## Rust 使用方法

Rust API 在 `crates/wxml-parser-core` 中（发布到 crates.io 的包名为 `wxml-parser`），当前对外函数：

- `parse_json(code: &str) -> serde_json::Value`
- `parse_for_eslint_json(code: &str) -> serde_json::Value`

### 1) Cargo.toml

```toml
[dependencies]
wxml-parser = "0.1.0"
serde_json = "1"
```

### 2) 示例代码

```rust
use wxml_parser_core::{parse_json, parse_for_eslint_json};

fn main() {
    let code = r#"<view>{{ message }}</view>"#;

    let ast = parse_json(code);
    println!("{}", ast["type"]); // "Program"

    let eslint_ast = parse_for_eslint_json(code);
    println!("{}", eslint_ast["ast"]["type"]); // "Program"
}
```

## Benchmark

使用 `tests/fixtures/bench/complex-mixed-large.wxml` 这份复杂 WXML fixture，对比以下 3 条解析路径：

- `rust-core`：直接调用 `crates/wxml-parser-core` 中的 `parse_json` / `parse_for_eslint_json`
- `napi`：通过当前包入口 `loader.js` 调用 `parse` / `parseForESLint`
- `js-parser`：调用已安装的 `@wxml/parser` 包中的 `parse` / `parseForESLint`

运行方式：

```bash
npm run bench
```

> 前置条件：
>
> - 先在当前仓库执行 `npm install`
> - 先在 `wxml-parser-rust` 下执行 `npm run build`
> - JS baseline 使用 `devDependencies` 中安装的 `@wxml/parser`

### Benchmark methodology

- `parse` 与 `parseForESLint` 分开统计
- 三种实现都使用同一份 fixture 文本
- benchmark fixture 包含少量中文与 emoji 文本，用于同时覆盖复杂 WXML 在 UTF-8 输入下的稳定性
- 文件读取不计入 benchmark
- benchmark 开始前先做 summary smoke check，确认结果结构一致
- 只校验轻量摘要字段，不序列化完整 AST
- 统计口径包含 warmup、多轮采样、`median ms/op`、`avg ms/op` 与 `ops/sec`

### Test environment

- CPU: Apple M2 Pro
- OS: Darwin 24.6.0
- Node: v20.20.1
- npm: 10.8.2
- rustc: 1.94.1
- warmup: 4 rounds
- measured rounds: 8
- iterations per round:
  - `parse`: 200
  - `parseForESLint`: 120

### parse

| Implementation | Median ms/op | Avg ms/op | ops/sec | Relative |
| -------------- | -----------: | --------: | ------: | -------: |
| rust-core      |        7.553 |     7.561 |   132.3 |    0.04x |
| napi           |        8.577 |     8.587 |   116.5 |    0.04x |
| js-parser      |        0.326 |     0.325 |  3073.3 |    1.00x |

Summary check:

```json
{
  "bodyLength": 2,
  "errorsLength": 0,
  "type": "Program"
}
```

### parseForESLint

| Implementation | Median ms/op | Avg ms/op | ops/sec | Relative |
| -------------- | -----------: | --------: | ------: | -------: |
| rust-core      |        8.134 |     8.178 |   122.3 |    0.04x |
| napi           |        9.264 |     9.312 |   107.4 |    0.04x |
| js-parser      |        0.356 |     0.361 |  2767.0 |    1.00x |

Summary check:

```json
{
  "bodyLength": 2,
  "errorsLength": 0,
  "type": "Program",
  "visitorProgramLength": 2
}
```

当前这组数据说明：在这份 fixture 和当前机器上，npm 包版本的 JS parser 明显更快；`napi` 相比直接调用 Rust core 还会额外承担一层 Node/Rust 边界成本。后续如果继续优化 Rust 版本，可以直接复用 `npm run bench` 跟踪趋势。

## 多平台产物聚合（bindings）

编译后的多平台 `.node` 统一放在 `bindings/` 目录。

先构建，再收集：

```bash
npm run build
npm run collect:bindings
```

会把收集到的 `.node` 统一放到 `bindings/` 目录。

或使用一条命令：

```bash
npm run build:multi
```

> 说明：单机通常只能稳定产出当前平台二进制。多平台产物建议通过 CI matrix 在不同平台构建后，再统一收集到 `bindings/`。

## 发布说明

> 发布 npm 包前请先执行 `npm run build` 生成本地 `.node` 产物；当前 `files` 会打包 `*.node` 文件。

发布前检查：

```bash
npm run build
npm test
npm pack --dry-run
```

确认无误后发布：

```bash
npm publish
```

### 发布 cargo 包（`wxml-parser`）

先做 dry-run：

```bash
cargo publish --dry-run -p wxml-parser
```

通过后正式发布：

```bash
cargo publish -p wxml-parser
```

> `wxml-parser-napi` 仅作为 npm 绑定层使用，当前配置为 `publish = false`，不发布到 crates.io。

***

## 包结构

- `crates/wxml-parser-core`：Rust 解析核心（crate 名：`wxml-parser`）
- `crates/wxml-parser-napi`：Node-API 绑定（`napi-rs`）
- `index.js` / `index.d.ts`：Node 导出入口
- `tests/`：兼容性测试

