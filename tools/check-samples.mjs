#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { execFileSync, spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const trackedFiles = execFileSync("git", ["ls-files"], {
  cwd: root,
  encoding: "utf8",
})
  .trim()
  .split("\n")
  .filter(Boolean);
const failures = [];

for (const file of trackedFiles) {
  const absolute = resolve(root, file);
  if (file.endsWith(".json")) {
    try {
      JSON.parse(readFileSync(absolute, "utf8"));
    } catch (error) {
      failures.push(`${file}: invalid JSON: ${error.message}`);
    }
  }
  if (file.endsWith(".js")) {
    run("node", ["--check", absolute], file);
  }
  if (/compose(?:\.[^.]+)?\.ya?ml$/.test(file)) {
    run("docker", ["compose", "-f", absolute, "config", "--quiet"], file);
  }
}

if (failures.length > 0) {
  throw new Error(`sample checks failed:\n${failures.join("\n")}`);
}
console.log(`sample checks passed (${trackedFiles.length} tracked files)`);

function run(command, args, file) {
  const result = spawnSync(command, args, { cwd: root, encoding: "utf8" });
  if (result.status !== 0) {
    failures.push(
      `${file}: ${command} failed: ${(result.stderr || result.stdout).trim()}`,
    );
  }
}
