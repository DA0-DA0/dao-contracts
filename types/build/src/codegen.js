"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const dotenv_1 = __importDefault(require("dotenv"));
const fs_1 = __importDefault(require("fs"));
const path_1 = __importDefault(require("path"));
const ts_codegen_1 = require("@cosmwasm/ts-codegen");
var OutputType;
(function (OutputType) {
    OutputType["contracts"] = "contracts";
    OutputType["packages"] = "packages";
})(OutputType || (OutputType = {}));
dotenv_1.default.config();
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
var LogLevels;
(function (LogLevels) {
    LogLevels[LogLevels["Silent"] = -1] = "Silent";
    LogLevels[LogLevels["Verbose"] = 2] = "Verbose";
    LogLevels[LogLevels["Debug"] = 3] = "Debug";
    LogLevels[LogLevels["Normal"] = 1] = "Normal";
})(LogLevels || (LogLevels = {}));
function log(msg, level = LogLevels.Normal) {
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
function generateTs(spec) {
    return __awaiter(this, void 0, void 0, function* () {
        const out = `${spec.outputPath}/${spec.outputType}/${spec.contractName}`;
        const name = spec.contractName;
        const schemas = (0, ts_codegen_1.readSchemas)({ schemaDir: spec.schemaDir, argv: { packed: false } });
        return yield (0, ts_codegen_1.generate)(name, schemas, out);
    });
}
function getSchemaDirectories(rootDir, contracts) {
    return new Promise((resolve, reject) => {
        var _a;
        const contractList = (_a = contracts === null || contracts === void 0 ? void 0 : contracts.split(",").map((dir) => dir.trim())) !== null && _a !== void 0 ? _a : [];
        const directories = [];
        if (contractList.length) {
            // get the schema directory for each contract
            for (const contractName of contractList) {
                const schemaDir = path_1.default.join(rootDir, contractName, "schema");
                directories.push([schemaDir, contractName]);
            }
            resolve(directories);
        }
        else {
            // get all the schema directories in all the contract directories
            fs_1.default.readdir(rootDir, (err, dirEntries) => {
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
                        const schemaDir = path_1.default.resolve(rootDir, entry, "schema");
                        if (fs_1.default.existsSync(schemaDir) &&
                            fs_1.default.lstatSync(schemaDir).isDirectory()) {
                            directories.push([schemaDir, entry]);
                        }
                        else {
                            log(`${schemaDir} is not a directory`, LogLevels.Verbose);
                        }
                    }
                    catch (e) {
                        console.warn(e);
                    }
                });
                resolve(directories);
            });
        }
    });
}
function main() {
    var _a;
    return __awaiter(this, void 0, void 0, function* () {
        let config = Object.assign({}, DEFAULT_CONFIG);
        const compilationSpecs = [];
        log("Calculating generation specs...");
        for (const root of config.schemaRoots) {
            const { name, paths, outputName, outputDir } = root;
            for (const path of paths) {
                const schemaDirectories = yield getSchemaDirectories(path);
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
        log(`code generating for ${(_a = compilationSpecs === null || compilationSpecs === void 0 ? void 0 : compilationSpecs.length) !== null && _a !== void 0 ? _a : 0} specs...`);
        if (CODEGEN_LOG_LEVEL === LogLevels.Debug) {
            console.log("Compilation specs:");
            console.dir(compilationSpecs);
        }
        const codegenResponses = [];
        for (const spec of compilationSpecs) {
            codegenResponses.push(generateTs(spec));
        }
        yield Promise.all(codegenResponses);
        log(`code generation complete`, LogLevels.Normal);
    });
}
main();
