#!/usr/bin/env bun
/**
 * CI-only: write an increasing semver into the three version files the
 * updater compares. GitHub run_number is monotonic per repo.
 *
 * Usage: bun scripts/stamp-version.mjs 0.1.42
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("usage: bun scripts/stamp-version.mjs 0.1.N");
  process.exit(1);
}

const root = resolve(import.meta.dirname, "..");

const pkgPath = resolve(root, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
pkg.version = version;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");

const tauriPath = resolve(root, "src-tauri/tauri.conf.json");
const tauri = JSON.parse(readFileSync(tauriPath, "utf8"));
tauri.version = version;
writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + "\n");

const cargoPath = resolve(root, "src-tauri/Cargo.toml");
const cargo = readFileSync(cargoPath, "utf8").replace(
  /^version = "[^"]+"/m,
  `version = "${version}"`,
);
writeFileSync(cargoPath, cargo);

console.log(`stamped version ${version}`);
