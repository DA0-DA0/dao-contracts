import { exec } from "child_process";
import dotenv from "dotenv";
import fs from "fs";
import path from "path";

enum OutputType {
  contracts = "contracts",
  packages = "packages"
}

export type CompilationSpec = {
  contractName: string;
  schemaDir: string;
  outputPath: string;
  outputType: OutputType;
};

dotenv.config();

const CONTRACTS_OUTPUT_DIR = ".";

const CODEGEN_LOG_LEVEL = (() => {
  const logLevel = process.env.CODEGEN_LOG_LEVEL || "";
  if (logLevel === "verbose") {
    return 2;
  }
  if (logLevel === "debug") {
    return 3;
  }
  if (logLevel === "silent") {
    return -1;
  }
  return 1;
})();

enum LogLevels {
  Silent = -1,
  Verbose = 2,
  Debug = 3,
  Normal = 1,
}

function log(msg: string, level = LogLevels.Normal) {
  if (CODEGEN_LOG_LEVEL < level) {
    return;
  }
  console.log(msg);
}

const DEFAULT_CONFIG = {
  schemaRoots: [
    {
      name: OutputType.contracts,
      paths: [`../${OutputType.contracts}`],
      outputName: OutputType.contracts,
      outputDir: CONTRACTS_OUTPUT_DIR,
    },
    {
      name: OutputType.packages,
      paths: [`../${OutputType.packages}`],
      outputName: OutputType.packages,
      outputDir: CONTRACTS_OUTPUT_DIR,
    },
  ]
};

async function run(cmd: string): Promise<boolean> {
  log(cmd, LogLevels.Verbose);
  return new Promise((resolve, reject) => {
    exec(cmd, (error, stdout, stderr) => {
      if (error) {
        console.error(`error: ${error.message}`);
        reject(error);
      }
      if (stderr) {
        console.error(`stderr: ${stderr}`);
        reject(stderr);
      }
      resolve(true);
    });
  });
}

async function generateTs(spec: CompilationSpec): Promise<boolean> {
  const script = "node_modules/cosmwasm-typescript-gen/main/cosmwasm-typescript-gen.js";
  return run(`${script} generate \
    --schema ${spec.schemaDir} \
    --out ${spec.outputPath}/${spec.outputType}/${spec.contractName} \
    --name ${spec.contractName}`)
}


function getSchemaDirectories(
  rootDir: string,
  contracts?: string
): Promise<string[][]> {
  return new Promise((resolve, reject) => {
    const contractList = contracts?.split(",").map((dir) => dir.trim()) ?? [];
    const directories: string[][] = [];
    if (contractList.length) {
      // get the schema directory for each contract
      for (const contractName of contractList) {
        const schemaDir = path.join(rootDir, contractName, "schema");
        directories.push([schemaDir, contractName]);
      }
      resolve(directories);
    } else {
      // get all the schema directories in all the contract directories
      fs.readdir(rootDir, (err, dirEntries) => {
        if (err) {
          console.error(err);
          return;
        }
        if (!dirEntries) {
          console.warn(`no entries found in ${rootDir}`);
          resolve([]);
          return;
        }
        dirEntries.forEach((entry) => {
          try {
            const schemaDir = path.resolve(rootDir, entry, "schema");
            if (
              fs.existsSync(schemaDir) &&
              fs.lstatSync(schemaDir).isDirectory()
            ) {
              directories.push([schemaDir, entry]);
            } else {
              log(`${schemaDir} is not a directory`, LogLevels.Verbose);
            }
          } catch (e) {
            console.warn(e);
          }
        });
        resolve(directories);
      });
    }
  });
}

async function main() {
  let config = {
    ...DEFAULT_CONFIG,
  };

  const compilationSpecs: CompilationSpec[] = [];
  log("Calculating generation specs...");
  for (const root of config.schemaRoots) {
    const { name, paths, outputName, outputDir } = root;
    for (const path of paths) {
      const schemaDirectories = await getSchemaDirectories(path);
      for (const [directory, contractName] of schemaDirectories) {
        compilationSpecs.push({
          contractName: contractName,
          schemaDir: directory,
          outputPath: outputDir,
          outputType: outputName,
        });
      }
    }
  }
  log(`code generating for ${compilationSpecs?.length ?? 0} specs...`);
  if (CODEGEN_LOG_LEVEL === LogLevels.Debug) {
    console.log("Compilation specs:");
    console.dir(compilationSpecs);
  }

  const codegenResponses = [];
  for (const spec of compilationSpecs) {
    codegenResponses.push(generateTs(spec));
  }
  await Promise.all(codegenResponses);

  log(`code generation complete`, LogLevels.Normal);
}

main();
