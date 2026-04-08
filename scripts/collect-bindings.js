const fs = require('fs')
const path = require('path')

const root = path.resolve(__dirname, '..')
const bindingsDir = path.join(root, 'bindings')

if (!fs.existsSync(bindingsDir)) {
  fs.mkdirSync(bindingsDir, { recursive: true })
}

function collectFrom(dir) {
  if (!fs.existsSync(dir)) return []
  const entries = fs.readdirSync(dir, { withFileTypes: true })
  const files = []
  for (const entry of entries) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...collectFrom(full))
      continue
    }
    if (entry.isFile() && entry.name.endsWith('.node')) {
      files.push(full)
    }
  }
  return files
}

const sources = [
  root,
  path.join(root, 'npm'),
  path.join(root, 'artifacts')
]

const copied = new Set()
for (const src of sources) {
  const files = src === root
    ? fs.readdirSync(root)
        .filter((name) => name.endsWith('.node'))
        .map((name) => path.join(root, name))
    : collectFrom(src)
  for (const file of files) {
    const base = path.basename(file)
    if (!base.startsWith('wxml-parser-rust.') || !base.endsWith('.node')) continue
    const target = path.join(bindingsDir, base)
    fs.copyFileSync(file, target)
    copied.add(base)
  }
}

if (copied.size === 0) {
  console.log('[collect-bindings] no .node files found')
} else {
  console.log(`[collect-bindings] collected ${copied.size} files into bindings/`)
  for (const name of copied) {
    console.log(`- ${name}`)
  }
}
