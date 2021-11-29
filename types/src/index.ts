import { exec } from 'child_process'
import dotenv from 'dotenv'
import fs from 'fs'
import path from 'path'
import { dedupe } from 'ts-dedupe'
import {
  compileFromFile,
  Options,
  DEFAULT_OPTIONS,
} from 'json-schema-to-typescript'
import { IDeDupeOptions } from 'ts-dedupe/dist/contracts'

export type CompilationSpec = {
  contractName: string
  schemaDir: string
  schemaFiles: string[]
  outputPath: string
  options: Options
}

dotenv.config()

const CONTRACTS_OUTPUT_DIR = 'build'
const TSCONFIG_DEFAULT = `{
  "compilerOptions": {
    "target": "es2017",
    "lib": ["esnext"],
    "baseUrl": ".",
    "sourceMap": true
  },
  "include": ["*.ts"],
  "exclude": ["node_modules"]
}    
`

const CODEGEN_NO_DEDUP = !!process.env.NO_DEDUP
const CODEGEN_LOG_LEVEL = (() => {
  const logLevel = process.env.CODEGEN_LOG_LEVEL || ''
  if (logLevel === 'verbose') {
    return 2
  }
  if (logLevel === 'debug') {
    return 3
  }
  if (logLevel === 'silent') {
    return -1
  }
  return 1
})()

enum LogLevels {
  Silent = -1,
  Verbose = 2,
  Debug = 3,
  Normal = 1,
}

function log(msg: string, level = LogLevels.Normal) {
  if (CODEGEN_LOG_LEVEL < level) {
    return
  }
  console.log(msg)
}

const DEFAULT_CONFIG = {
  schemaRoots: [
    {
      name: 'contracts',
      paths: [process.env.CONTRACTS_ROOT || '../contracts'],
      outputName: 'contracts',
      outputDir: CONTRACTS_OUTPUT_DIR,
    }
  ],
  tsconfig: TSCONFIG_DEFAULT,
}

async function getSchemaFiles(schemaDir: string): Promise<string[]> {
  return new Promise((resove, reject) => {
    const schemaFiles: string[] = []
    fs.readdir(schemaDir, (err, dirEntries) => {
      if (err) {
        console.error(err)
      }

      dirEntries.forEach((entry) => {
        const fullPath = path.join(schemaDir, entry)
        if (entry.endsWith('.json') && fs.existsSync(fullPath)) {
          schemaFiles.push(fullPath)
        }
      })

      resove(schemaFiles)
    })
  })
}

async function schemaCompileOptions(
  contractName: string,
  contractRoot: string,
  outputDir: string,
  schemaDir: string
): Promise<CompilationSpec> {
  const schemaFiles = await getSchemaFiles(schemaDir)
  const outputPath = path.join(outputDir, contractRoot, contractName)
  const options: Options = {
    ...DEFAULT_OPTIONS,
    bannerComment: '',
    format: false,
  }
  return {
    contractName,
    schemaDir,
    schemaFiles,
    outputPath,
    options,
  }
}

function deleteFile(filePath?: string) {
  if (!filePath) {
    return
  }
  if (fs.existsSync(filePath)) {
    try {
      fs.unlinkSync(filePath)
    } catch (e) {
      console.error(e)
    }
  }
}

function removeDirectory(dir: string) {
  try {
    fs.rmdirSync(dir, { recursive: true })
  } catch (err) {
    console.error(`Error while deleting ${dir}.`)
  }
}

function writeTsconfig(outputPath: string, tsconfig = TSCONFIG_DEFAULT) {
  fs.writeFileSync(path.join(outputPath, 'tsconfig.json'), tsconfig)
}

async function run(cmd: string): Promise<boolean> {
  log(cmd, LogLevels.Verbose)
  return new Promise((resolve, reject) => {
    exec(cmd, (error, stdout, stderr) => {
      if (error) {
        console.error(`error: ${error.message}`)
        reject(error)
      }
      if (stderr) {
        console.error(`stderr: ${stderr}`)
        reject(stderr)
      }
      resolve(true)
    })
  })
}

function getSchemaDirectories(
  rootDir: string,
  contracts?: string
): Promise<string[][]> {
  return new Promise((resolve, reject) => {
    const contractList = contracts?.split(',').map((dir) => dir.trim()) ?? []
    const directories: string[][] = []
    if (contractList.length) {
      // get the schema directory for each contract
      for (const contractName of contractList) {
        const schemaDir = path.join(rootDir, contractName, 'schema')
        directories.push([schemaDir, contractName])
      }
      resolve(directories)
    } else {
      // get all the schema directories in all the contract directories
      fs.readdir(rootDir, (err, dirEntries) => {
        if (err) {
          console.error(err)
          return
        }
        if (!dirEntries) {
          console.warn(`no entries found in ${rootDir}`)
          resolve([])
          return
        }
        dirEntries.forEach((entry) => {
          try {
            const schemaDir = path.resolve(rootDir, entry, 'schema')
            if (
              fs.existsSync(schemaDir) &&
              fs.lstatSync(schemaDir).isDirectory()
            ) {
              directories.push([schemaDir, entry])
            } else {
              log(`${schemaDir} is not a directory`, LogLevels.Verbose)
            }
          } catch (e) {
            console.warn(e)
          }
        })
        resolve(directories)
      })
    }
  })
}

function isEmptyFile(filename: string) {
  const contents = fs.readFileSync(filename, 'utf8').trim()
  return !contents
}

async function findEmptyFiles(directory: string): Promise<string[]> {
  return new Promise((resolve, reject) => {
    const emptyFiles: string[] = []
    fs.readdir(directory, (err, dirEntries) => {
      if (err) {
        console.error(err)
        return
      }
      if (!dirEntries) {
        console.warn(`no entries found in ${directory}`)
        resolve([])
        return
      }
      dirEntries.forEach((entry) => {
        try {
          const filename = path.resolve(directory, entry)
          if (
            fs.existsSync(filename) &&
            !fs.lstatSync(filename).isDirectory()
          ) {
            if (isEmptyFile(filename)) {
              emptyFiles.push(entry.replace('.d.ts', ''))
            }
          }
        } catch (e) {
          console.warn(e)
        }
      })
      resolve(emptyFiles)
    })
  })
}

function removeEmptyItems(barrelFile: string, emptyFiles: string[]) {
  const emptyFileSet = new Set<string>(
    emptyFiles.map((emptyName) => `export * from "./${emptyName}";`)
  )
  const contents = fs.readFileSync(barrelFile, 'utf-8')
  const lines = contents.split('\n')
  const outputLines = []
  for (const line of lines) {
    if (emptyFileSet.has(line)) {
      outputLines.push(`// dedup emptied this file\n// ${line}`)
    } else {
      outputLines.push(line)
    }
  }
  fs.writeFileSync(barrelFile, outputLines.join('\n'))
}

async function dedup(inputPath: string, outputPath?: string): Promise<void> {
  if (!outputPath) {
    outputPath = inputPath
  }
  log(`starting dedup files in ${inputPath}...`, LogLevels.Verbose)
  const options: IDeDupeOptions = {
    project: path.join(inputPath, 'tsconfig.json'),
    duplicatesFile: path.join(outputPath, 'shared-types.d.ts'),
    barrelFile: path.join(outputPath, 'index.ts'),
    retainEmptyFiles: true,
  }
  if (CODEGEN_LOG_LEVEL === LogLevels.Debug) {
    options.logger = console
  }
  deleteFile(options.barrelFile)
  deleteFile(options.duplicatesFile)
  await dedupe(options)
  log(`dedup complete for ${outputPath}`, LogLevels.Verbose)
  // Now, remove any files fully emptied by dedup'ing
  // from the index file
  const emptyFiles = await findEmptyFiles(outputPath)
  if (emptyFiles.length && options.barrelFile) {
    log(`emptyFiles in ${outputPath}: ${emptyFiles}`, LogLevels.Verbose)
    removeEmptyItems(options.barrelFile, emptyFiles)
  }
}

function ensurePath(outputPath: string) {
  if (fs.existsSync(outputPath)) {
    return
  }
  try {
    fs.mkdirSync(outputPath, { recursive: true })
  } catch (e) {
    console.log(e)
  }
}

async function compileSchemaFile(schemaFile: string, spec: CompilationSpec) {
  const outputFile = path.join(
    spec.outputPath,
    path.basename(schemaFile).replace('.json', '.d.ts')
  )
  const ts = await compileFromFile(schemaFile, spec.options)
  ensurePath(path.dirname(outputFile))
  fs.writeFileSync(outputFile, ts)
}

async function main() {
  let config = {
    ...DEFAULT_CONFIG,
  }

  const compilationSpecs = []
  let fileCount = 0
  log('Calculating generation specs...')
  for (const root of config.schemaRoots) {
    const { name, paths, outputName, outputDir } = root
    const contractOutputPath = path.join(outputDir, outputName)
    log(`Clearing output path ${contractOutputPath}`)
    removeDirectory(contractOutputPath)
      ensurePath(contractOutputPath)
    for (const path of paths) {
      const schemaDirectories = await getSchemaDirectories(path)
      for (const [directory, contractName] of schemaDirectories) {
        const compilationOptions = await schemaCompileOptions(
          contractName,
          name,
          outputDir,
          directory
        )
        fileCount += compilationOptions?.schemaFiles?.length ?? 0
        compilationSpecs.push(compilationOptions)
      }
    }
  }
  log(
    `code generating for ${fileCount} files in ${
      compilationSpecs?.length ?? 0
    } specs...`
  )
  if (CODEGEN_LOG_LEVEL === LogLevels.Debug) {
    console.log('Compilation specs:')
    console.dir(compilationSpecs)
  }

  const compilationPromises = []
  for (const spec of compilationSpecs) {
    for (const schemaFile of spec.schemaFiles) {
      compilationPromises.push(compileSchemaFile(schemaFile, spec))
    }
  }
  await Promise.all(compilationPromises)
  if (CODEGEN_NO_DEDUP) {
    log(`Skipping dedup step`, LogLevels.Verbose)
  } else {
    const dedupPromises = []
    for (const spec of compilationSpecs) {
      writeTsconfig(spec.outputPath, config.tsconfig)
      dedupPromises.push(dedup(spec.outputPath))
    }
    await Promise.all(dedupPromises)
  }
  log(`code generation complete`, LogLevels.Normal)
}

main()
