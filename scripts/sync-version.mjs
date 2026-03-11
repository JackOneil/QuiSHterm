import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const rootDir = resolve(import.meta.dirname, "..");
const packageJsonPath = resolve(rootDir, "package.json");
const cargoTomlPath = resolve(rootDir, "src-tauri", "Cargo.toml");
const tauriConfigPath = resolve(rootDir, "src-tauri", "tauri.conf.json");

const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
const version = String(packageJson.version || "").trim();

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`package.json version \"${version}\" is not valid Semantic Versioning.`);
}

const cargoToml = readFileSync(cargoTomlPath, "utf8");
const cargoVersionPattern = /(^version\s*=\s*")([^"]+)("\s*$)/m;

if (!cargoVersionPattern.test(cargoToml)) {
  throw new Error("Failed to locate the package version field in src-tauri/Cargo.toml.");
}

const nextCargoToml = cargoToml.replace(cargoVersionPattern, `$1${version}$3`);

const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, "utf8"));
tauriConfig.version = version;

writeFileSync(cargoTomlPath, nextCargoToml);
writeFileSync(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

console.log(`Synchronized Tauri build version to ${version}.`);