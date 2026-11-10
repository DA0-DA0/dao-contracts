#!/usr/bin/env node
// Checks optimized contract artifacts against the per-function and total
// local-variable limits that cosmwasm-vm enforces during static validation
// when a contract is stored (`check_wasm` -> `check_wasm_functions`).
//
// cosmwasm-vm counts, for every function body in the code section, the sum of
// the `count` fields of its local declarations. Parameters are NOT counted.
// Defaults (cosmwasm-vm `WasmLimits`, since 2.2.10 / wasmvm 2.2.9):
//   max_function_locals = 100, max_total_function_locals = 10_000.
//
// Usage:
//   node scripts/check-wasm-limits.mjs --checksums artifacts/checksums.txt
//     checks every artifact listed in the optimizer's checksums.txt
//   node scripts/check-wasm-limits.mjs <artifact.wasm>...
//     checks the given files
// Exit code 1 when any artifact violates a limit or cannot be parsed, or when
// checksums.txt is missing, malformed or empty; 2 on invalid arguments.
import { readFileSync } from 'node:fs'
import { basename, dirname, join } from 'node:path'

const MAX_FUNCTION_LOCALS = 100
const MAX_TOTAL_FUNCTION_LOCALS = 10_000

function readU32(bytes, pos, limit = bytes.length) {
  let result = 0
  for (let shift = 0; shift < 35; shift += 7) {
    if (pos >= limit) throw new Error(`unexpected end of data at offset ${pos}`)
    const byte = bytes[pos++]
    result |= (byte & 0x7f) << shift
    if ((byte & 0x80) === 0) return [result >>> 0, pos]
  }
  throw new Error(`invalid LEB128 integer before offset ${pos}`)
}

function skip(pos, n, limit) {
  if (pos + n > limit) throw new Error(`unexpected end of data at offset ${limit}`)
  return pos + n
}

// Returns { importedFunctions, locals: number[] } where locals[i] is the
// declared local count of the i-th defined (non-imported) function. Throws on
// any malformed or truncated input.
function analyze(bytes) {
  if (bytes.length < 8 || bytes.readUInt32LE(0) !== 0x6d736100) {
    throw new Error('not a wasm binary')
  }
  let pos = 8
  let importedFunctions = 0
  let declaredFunctions
  let locals
  while (pos < bytes.length) {
    const id = bytes[pos++]
    let size
    ;[size, pos] = readU32(bytes, pos)
    const end = pos + size
    if (end > bytes.length) {
      throw new Error(`section ${id} at offset ${pos} ends past the end of the file`)
    }
    if (id === 2) {
      let count
      ;[count, pos] = readU32(bytes, pos, end)
      for (let i = 0; i < count; i++) {
        let len
        ;[len, pos] = readU32(bytes, pos, end)
        pos = skip(pos, len, end) // module name
        ;[len, pos] = readU32(bytes, pos, end)
        pos = skip(pos, len, end) // field name
        pos = skip(pos, 1, end)
        const kind = bytes[pos - 1]
        if (kind === 0) {
          importedFunctions++
          ;[, pos] = readU32(bytes, pos, end) // type index
        } else if (kind === 1) {
          pos = skip(pos, 2, end) // reftype, flags
          const flags = bytes[pos - 1]
          ;[, pos] = readU32(bytes, pos, end)
          if (flags & 1) [, pos] = readU32(bytes, pos, end)
        } else if (kind === 2) {
          pos = skip(pos, 1, end)
          const flags = bytes[pos - 1]
          ;[, pos] = readU32(bytes, pos, end)
          if (flags & 1) [, pos] = readU32(bytes, pos, end)
        } else if (kind === 3) {
          pos = skip(pos, 2, end) // valtype, mutability
        } else {
          throw new Error(`unknown import kind ${kind}`)
        }
      }
    } else if (id === 3) {
      ;[declaredFunctions] = readU32(bytes, pos, end)
    } else if (id === 10) {
      let count
      ;[count, pos] = readU32(bytes, pos, end)
      locals = []
      for (let i = 0; i < count; i++) {
        let bodySize
        ;[bodySize, pos] = readU32(bytes, pos, end)
        const bodyEnd = pos + bodySize
        if (bodyEnd > end) {
          throw new Error(`function body ${i} ends past the end of the code section`)
        }
        let groups
        ;[groups, pos] = readU32(bytes, pos, bodyEnd)
        let total = 0
        for (let g = 0; g < groups; g++) {
          let n
          ;[n, pos] = readU32(bytes, pos, bodyEnd)
          pos = skip(pos, 1, bodyEnd) // valtype
          total += n
        }
        locals.push(total)
        pos = bodyEnd
      }
    }
    pos = end
  }
  if (locals === undefined) throw new Error('no code section')
  if (locals.length === 0) throw new Error('code section has no function bodies')
  if (declaredFunctions !== undefined && declaredFunctions !== locals.length) {
    throw new Error(
      `function section declares ${declaredFunctions} functions but the code section has ${locals.length} bodies`,
    )
  }
  return { importedFunctions, locals }
}

function fail(message) {
  console.error(`error: ${message}`)
  process.exit(1)
}

function usage() {
  console.error(
    'usage: check-wasm-limits.mjs --checksums <checksums.txt>\n' +
      '       check-wasm-limits.mjs <artifact.wasm>...',
  )
  process.exit(2)
}

// Lists the artifacts named in the optimizer's checksums.txt
// (`sha256sum -- *.wasm` output), resolved relative to that file.
function filesFromChecksums(path) {
  let text
  try {
    text = readFileSync(path, 'utf8')
  } catch (error) {
    fail(`cannot read ${path} (${error.code ?? error.message}); run \`just workspace-optimize\` first`)
  }
  const files = text
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const match = /^[0-9a-f]{64} [ *](.+\.wasm)$/.exec(line)
      if (!match) fail(`unexpected line in ${path}: ${line}`)
      return join(dirname(path), match[1])
    })
  if (files.length === 0) fail(`${path} lists no .wasm artifacts`)
  return files
}

const args = process.argv.slice(2)
let files
if (args[0] === '--checksums') {
  if (args.length !== 2) usage()
  files = filesFromChecksums(args[1])
} else {
  files = args
}
if (files.length === 0) usage()

let failed = false
for (const file of files) {
  const name = basename(file).padEnd(44)
  let result
  try {
    result = analyze(readFileSync(file))
  } catch (error) {
    console.log(`FAIL ${name} cannot be checked: ${error.code === 'ENOENT' ? 'file not found' : error.message}`)
    failed = true
    continue
  }
  const { importedFunctions, locals } = result
  const total = locals.reduce((a, b) => a + b, 0)
  const max = Math.max(...locals)
  const maxIndex = importedFunctions + locals.indexOf(max)
  const problems = []
  if (max > MAX_FUNCTION_LOCALS) {
    problems.push(`function ${maxIndex} has ${max} locals (limit ${MAX_FUNCTION_LOCALS})`)
  }
  if (total > MAX_TOTAL_FUNCTION_LOCALS) {
    problems.push(`${total} locals in total (limit ${MAX_TOTAL_FUNCTION_LOCALS})`)
  }
  const status = problems.length ? 'FAIL' : 'ok  '
  console.log(
    `${status} ${name} max locals ${String(max).padStart(3)} (func ${maxIndex}), total ${String(total).padStart(5)}` +
      (problems.length ? `  <-- ${problems.join('; ')}` : ''),
  )
  if (problems.length) failed = true
}
if (failed) {
  console.error(
    `\nSome artifacts exceed cosmwasm-vm's function local limits or could not be parsed; see https://github.com/CosmWasm/cosmwasm/blob/main/packages/vm/src/config.rs`,
  )
  process.exit(1)
}
