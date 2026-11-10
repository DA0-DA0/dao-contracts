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
// Usage: node scripts/check-wasm-limits.mjs artifacts/*.wasm
// Exit code 1 when any artifact violates a limit.
import { readFileSync } from 'node:fs'
import { basename } from 'node:path'

const MAX_FUNCTION_LOCALS = 100
const MAX_TOTAL_FUNCTION_LOCALS = 10_000

function readU32(bytes, pos) {
  let result = 0
  let shift = 0
  for (;;) {
    const byte = bytes[pos++]
    result |= (byte & 0x7f) << shift
    if ((byte & 0x80) === 0) return [result >>> 0, pos]
    shift += 7
  }
}

// Returns { importedFunctions, locals: number[] } where locals[i] is the
// declared local count of the i-th defined (non-imported) function.
function analyze(bytes) {
  if (bytes.length < 8 || bytes.readUInt32LE(0) !== 0x6d736100) {
    throw new Error('not a wasm binary')
  }
  let pos = 8
  let importedFunctions = 0
  const locals = []
  while (pos < bytes.length) {
    const id = bytes[pos++]
    let size
    ;[size, pos] = readU32(bytes, pos)
    const end = pos + size
    if (id === 2) {
      let count
      ;[count, pos] = readU32(bytes, pos)
      for (let i = 0; i < count; i++) {
        let len
        ;[len, pos] = readU32(bytes, pos)
        pos += len // module name
        ;[len, pos] = readU32(bytes, pos)
        pos += len // field name
        const kind = bytes[pos++]
        if (kind === 0) {
          importedFunctions++
          ;[, pos] = readU32(bytes, pos) // type index
        } else if (kind === 1) {
          pos++ // reftype
          const flags = bytes[pos++]
          ;[, pos] = readU32(bytes, pos)
          if (flags & 1) [, pos] = readU32(bytes, pos)
        } else if (kind === 2) {
          const flags = bytes[pos++]
          ;[, pos] = readU32(bytes, pos)
          if (flags & 1) [, pos] = readU32(bytes, pos)
        } else if (kind === 3) {
          pos += 2 // valtype, mutability
        } else {
          throw new Error(`unknown import kind ${kind}`)
        }
      }
    } else if (id === 10) {
      let count
      ;[count, pos] = readU32(bytes, pos)
      for (let i = 0; i < count; i++) {
        let bodySize
        ;[bodySize, pos] = readU32(bytes, pos)
        const bodyEnd = pos + bodySize
        let groups
        ;[groups, pos] = readU32(bytes, pos)
        let total = 0
        for (let g = 0; g < groups; g++) {
          let n
          ;[n, pos] = readU32(bytes, pos)
          pos++ // valtype
          total += n
        }
        locals.push(total)
        pos = bodyEnd
      }
    }
    pos = end
  }
  return { importedFunctions, locals }
}

const files = process.argv.slice(2)
if (files.length === 0) {
  console.error('usage: check-wasm-limits.mjs <artifact.wasm>...')
  process.exit(2)
}

let failed = false
for (const file of files) {
  const { importedFunctions, locals } = analyze(readFileSync(file))
  const total = locals.reduce((a, b) => a + b, 0)
  const max = Math.max(0, ...locals)
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
    `${status} ${basename(file).padEnd(44)} max locals ${String(max).padStart(3)} (func ${maxIndex}), total ${String(total).padStart(5)}` +
      (problems.length ? `  <-- ${problems.join('; ')}` : ''),
  )
  if (problems.length) failed = true
}
if (failed) {
  console.error(`\nSome artifacts exceed cosmwasm-vm's function local limits; see https://github.com/CosmWasm/cosmwasm/blob/main/packages/vm/src/config.rs`)
  process.exit(1)
}
