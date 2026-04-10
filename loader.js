/* eslint-disable */

const { existsSync } = require('fs')
const { join } = require('path')

function withBindingsDir(fileName) {
  return join(__dirname, 'bindings', fileName)
}

function patchEmptyProgramNaN(program) {
  if (!program || program.type !== 'Program') return program
  if (program.start !== null || program.end !== null) return program

  program.start = Number.NaN
  program.end = Number.NaN
  if (Array.isArray(program.range) && program.range.length >= 2) {
    program.range[0] = Number.NaN
    program.range[1] = Number.NaN
  }

  if (program.loc && program.loc.start && program.loc.end) {
    program.loc.start.line = Number.NaN
    program.loc.start.column = Number.NaN
    program.loc.end.line = Number.NaN
    program.loc.end.column = Number.NaN
  }

  return program
}

let nativeBinding = null
let loadError = null

const localCandidates = [
  `wxml-parser-rs.${process.platform}-${process.arch}.node`,
  'wxml-parser-rs.darwin-universal.node',
]

for (const name of localCandidates) {
  const path = withBindingsDir(name)
  if (!existsSync(path)) continue
  try {
    nativeBinding = require(path)
    break
  } catch (e) {
    loadError = e
  }
}

if (!nativeBinding && loadError) {
  throw loadError
}

if (!nativeBinding) {
  nativeBinding = require('./index.js')
}

const { parse: nativeParse, parseForESLint: nativeParseForESLint } = nativeBinding

function parse(code) {
  const jsonStr = nativeParse(code)
  return patchEmptyProgramNaN(JSON.parse(jsonStr))
}

function parseForESLint(code) {
  const jsonStr = nativeParseForESLint(code)
  const result = JSON.parse(jsonStr)
  if (result && result.ast) patchEmptyProgramNaN(result.ast)
  return result
}

module.exports.parse = parse
module.exports.parseForESLint = parseForESLint
