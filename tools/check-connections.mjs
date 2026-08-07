#!/usr/bin/env node
// Guard against CONNECTIONS.md drifting from the compose files.
//
// The doc is what people copy credentials out of, so a stale port there is
// worse than no doc at all: it sends someone debugging a connection that was
// never going to work. This compares the published port of every
// `<engine>/compose.yaml` against the table in CONNECTIONS.md.
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const doc = readFileSync(resolve(root, "CONNECTIONS.md"), "utf8");

// `| engine | 55432 | ...`
const documented = new Map(
  [...doc.matchAll(/^\| (\w+) \| (\d+) \|/gm)].map((m) => [m[1], m[2]]),
);

const problems = [];
for (const entry of readdirSync(root, { withFileTypes: true })) {
  if (!entry.isDirectory()) continue;
  const compose = resolve(root, entry.name, "compose.yaml");
  if (!existsSync(compose)) continue;

  // First published port wins; the extras are web consoles.
  const published = readFileSync(compose, "utf8").match(/"(\d+):(\d+)"/);
  if (!published) continue;

  const expected = documented.get(entry.name);
  if (!expected) {
    problems.push(`${entry.name}: has a compose.yaml but no row in CONNECTIONS.md`);
  } else if (expected !== published[1]) {
    problems.push(
      `${entry.name}: CONNECTIONS.md says ${expected}, compose publishes ${published[1]}`,
    );
  }
}

if (problems.length > 0) {
  throw new Error(`CONNECTIONS.md is out of date:\n  ${problems.join("\n  ")}`);
}
console.log(`connection reference matches ${documented.size} compose files`);
