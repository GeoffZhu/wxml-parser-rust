#!/usr/bin/env node

/**
 * Bump version across all package manifests.
 *
 * Usage:
 *   node scripts/bump-version.js <new-version>
 *
 * Updates:
 *   - package.json
 *   - crates/wxml-parser-core/Cargo.toml
 *   - crates/wxml-parser-napi/Cargo.toml
 */

const fs = require('fs')
const path = require('path')

const rootDir = path.join(__dirname, '..')
const newVersion = process.argv[2]

if (!newVersion) {
  console.error('Usage: node scripts/bump-version.js <new-version>')
  process.exit(1)
}

const semverRegex = /^\d+\.\d+\.\d+(-[\w.]+)?$/
if (!semverRegex.test(newVersion)) {
  console.error(`Invalid version: ${newVersion}`)
  process.exit(1)
}

const files = [
  {
    path: path.join(rootDir, 'package.json'),
    update(content) {
      const pkg = JSON.parse(content)
      pkg.version = newVersion
      return JSON.stringify(pkg, null, 2) + '\n'
    },
  },
  {
    path: path.join(rootDir, 'crates/wxml-parser-core/Cargo.toml'),
    update(content) {
      return content.replace(
        /^version\s*=\s*"\d+\.\d+\.\d+(-[\w.]+)?"/m,
        `version = "${newVersion}"`,
      )
    },
  },
  {
    path: path.join(rootDir, 'crates/wxml-parser-napi/Cargo.toml'),
    update(content) {
      return content
        .replace(
          /^version\s*=\s*"\d+\.\d+\.\d+(-[\w.]+)?"/m,
          `version = "${newVersion}"`,
        )
        .replace(
          /wxml_parser_rs\s*=\s*\{\s*package\s*=\s*"wxml-parser-rs",\s*version\s*=\s*"\d+\.\d+\.\d+(-[\w.]+)?"/,
          `wxml_parser_rs = { package = "wxml-parser-rs", version = "${newVersion}"`,
        )
    },
  },
]

for (const file of files) {
  const relPath = path.relative(rootDir, file.path)
  const original = fs.readFileSync(file.path, 'utf8')
  const updated = file.update(original)
  if (original === updated) {
    console.warn(`  [skip] ${relPath} (version not found or already up to date)`)
    continue
  }
  fs.writeFileSync(file.path, updated, 'utf8')
  console.log(`  [ok]   ${relPath} → ${newVersion}`)
}

console.log(`\nVersion bumped to ${newVersion}. Remember to update CHANGELOG.md.`)
