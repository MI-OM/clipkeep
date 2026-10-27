import { readFile } from "node:fs/promises";

const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const tauriConfig = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const cargoToml = await readFile(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const versions = {
  "package.json": packageJson.version,
  "src-tauri/tauri.conf.json": tauriConfig.version,
  "src-tauri/Cargo.toml": cargoVersion,
};

if (Object.values(versions).some((version) => !version) || new Set(Object.values(versions)).size !== 1) {
  console.error("Release versions must be present and match:");
  for (const [file, version] of Object.entries(versions)) {
    console.error(`  ${file}: ${version ?? "missing"}`);
  }
  process.exit(1);
}

const refType = process.env.GITHUB_REF_TYPE;
const refName = process.env.GITHUB_REF_NAME;
if (refType === "tag" && refName !== `v${packageJson.version}`) {
  console.error(`Release tag ${refName} does not match version v${packageJson.version}.`);
  process.exit(1);
}

console.log(`Release version v${packageJson.version} is consistent across package metadata.`);
