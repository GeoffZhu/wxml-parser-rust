# wxml-parser-rust

`wxml-parser-rust` 是 [`@wxml/parser`](https://www.npmjs.com/package/@wxml/parser) 的 Rust + `napi-rs` 实现，提供：

- Rust 核心解析库（[`wxml-parser`](https://crates.io/crates/wxml-parser)）
- Node.js API（`parse` / `parseForESLint`），附带多平台预编译二进制

## 安装

```bash
npm install wxml-parser-rust
```

或 Rust 侧：

```bash
cargo add wxml-parser
```

## 使用方法

### Node.js

```js
const { parse, parseForESLint } = require('wxml-parser-rust')

const code = `<view wx:if="{{ok}}">hello</view>`

const ast = parse(code)
console.log(ast.type) // Program

const eslintAst = parseForESLint(code)
console.log(eslintAst.ast.type) // Program
```

### Rust

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

## 性能

基于 `tests/fixtures/bench/complex-mixed-large.wxml`，对比三种实现：

- `rust-core`：直接调用 Rust `parse_json` / `parse_for_eslint_json`
- `napi`：通过 `loader.js` 调用 `parse` / `parseForESLint`
- `js-parser`：`@wxml/parser` 的 JS 实现

> 环境：Apple M2 Pro / macOS / Node v20.20.1 / rustc 1.94.1 · warmup 4 轮 · 采样 8 轮

### parse

| Implementation | Median ms/op | ops/sec | Relative |
| --- | ---: | ---: | ---: |
| rust-core | 0.049 | 19,836.6 | 6.69x |
| napi | 0.423 | 2,364.5 | 0.78x |
| js-parser | 0.330 | 2,812.0 | 1.00x |

### parseForESLint

| Implementation | Median ms/op | ops/sec | Relative |
| --- | ---: | ---: | ---: |
| rust-core | 0.054 | 18,377.7 | 6.74x |
| napi | 0.444 | 2,255.2 | 0.83x |
| js-parser | 0.366 | 2,750.8 | 1.00x |

Rust 核心解析速度约为 JS 实现的 **6.7 倍**；通过 napi 调用因跨边界开销，相比 JS 实现约慢 17-20%。可直接运行 `npm run bench` 复现。

## 开发

```bash
# 安装依赖
npm install

# 构建（debug）
npm run build:debug

# 构建（release）
npm run build

# 测试
npm test

# Rust 侧测试
cargo test -p wxml-parser

# 基准测试
npm run bench
```

## 版本管理

发布新版本前，使用版本同步脚本更新所有 manifest：

```bash
npm run version:bump <new-version>
```

该脚本会同步更新 `package.json`、`crates/wxml-parser-core/Cargo.toml`、`crates/wxml-parser-napi/Cargo.toml` 中的版本号。更新后请一并更新 `CHANGELOG.md`。

## 多平台产物

编译后的 `.node` 文件统一放在 `bindings/` 目录。本地构建 + 收集：

```bash
npm run build:multi
```

多平台产物通过 CI matrix 在不同平台构建后自动收集。

## 发布

### npm

发布流程由 CI 自动完成：推送 `v*` tag 即可触发构建、测试和发布。

手动发布（需先构建）：

```bash
npm run build
npm test
npm publish
```

### crates.io

```bash
cargo publish --dry-run -p wxml-parser
cargo publish -p wxml-parser
```

> `wxml-parser-napi` 仅作为 npm 绑定层，不发布到 crates.io。

## 项目结构

```
├── crates/
│   ├── wxml-parser-core/   # Rust 解析核心（crate 名：wxml-parser）
│   └── wxml-parser-napi/   # Node-API 绑定层（napi-rs）
├── tests/                  # 兼容性测试
├── scripts/                # 构建/版本管理脚本
├── bindings/               # 多平台预编译 .node 产物
├── loader.js               # Node.js 加载入口
├── index.js                # napi 生成入口
└── index.d.ts              # TypeScript 类型声明
```

## License

[MIT](./LICENSE)
