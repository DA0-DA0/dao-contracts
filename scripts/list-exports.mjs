import fs from 'fs'

const wasm = fs.readFileSync(process.argv[2])
const module = await WebAssembly.compile(wasm)

console.log(WebAssembly.Module.exports(module).map(({name}) => name))
