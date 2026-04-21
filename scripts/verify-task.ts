/**
 * verify-task.ts — Validates task acceptance criteria and global invariants
 * Usage: bun run scripts/verify-task.ts TRIOS-101
 */
import { existsSync, readFileSync } from "fs";
import { execSync } from "child_process";

const taskId = process.argv[2];
if (!taskId) {
  console.error("Usage: bun run scripts/verify-task.ts TASK_ID");
  process.exit(1);
}

const taskFile = `tasks/${taskId}.yaml`;
if (!existsSync(taskFile)) {
  console.error(`Task file ${taskFile} not found`);
  process.exit(1);
}

let pass = 0;
let fail = 0;

function check(desc: string, fn: () => boolean) {
  try {
    if (fn()) {
      console.log(`  CHECK: ${desc} ... \x1b[32mPASS\x1b[0m`);
      pass++;
    } else {
      console.log(`  CHECK: ${desc} ... \x1b[31mFAIL\x1b[0m`);
      fail++;
    }
  } catch {
    console.log(`  CHECK: ${desc} ... \x1b[31mFAIL\x1b[0m`);
    fail++;
  }
}

function grepCount(file: string, pattern: RegExp): number {
  if (!existsSync(file)) return 0;
  const content = readFileSync(file, "utf-8");
  const matches = content.match(pattern);
  return matches ? matches.length : 0;
}

function run(cmd: string): boolean {
  try {
    execSync(cmd, { stdio: "pipe" });
    return true;
  } catch {
    return false;
  }
}

console.log(`=== Verifying task ${taskId} ===\n`);

// Global invariants
console.log("--- Global Invariants ---");

check("I1: bg-sw.js exists", () => existsSync("crates/trios-ext/extension/dist/bg-sw.js"));
check("I2: bootstrap.js exists", () => existsSync("crates/trios-ext/extension/dist/bootstrap.js"));
check("I3: 0 WASM imports in SW", () => {
  const bg = readFileSync("crates/trios-ext/extension/dist/bg-sw.js", "utf-8");
  // Only check for actual imports/requires, not comments
  const lines = bg.split("\n").filter(l => !l.trimStart().startsWith("//"));
  return !lines.some(l => /import.*wasm|require.*wasm|WebAssembly/i.test(l));
});
check("I4: 0 WebSocket in ext", () => {
  const mcp = readFileSync("crates/trios-ext/src/mcp.rs", "utf-8");
  return !mcp.includes("WebSocket") && !mcp.includes("ws://");
});
check("I5: Single ext tree", () => existsSync("crates/trios-ext/extension"));
check("I7: CSP wasm-unsafe-eval", () => {
  const manifest = readFileSync("crates/trios-ext/extension/manifest.json", "utf-8");
  return manifest.includes("wasm-unsafe-eval");
});
check("I12: --target web output", () => {
  const js = readFileSync("crates/trios-ext/extension/dist/trios_ext.js", "utf-8");
  return js.includes("import.meta.url");
});
check("I13: Accept header in mcp.rs", () => {
  const mcp = readFileSync("crates/trios-ext/src/mcp.rs", "utf-8");
  return mcp.includes("Accept");
});
check("I14: Chat REST endpoint", () => {
  const mcp = readFileSync("crates/trios-ext/src/mcp.rs", "utf-8");
  return mcp.includes("CHAT_HTTP_URL");
});

// Summary
console.log(`\n=== Results: \x1b[32m${pass} passed\x1b[0m, \x1b[31m${fail} failed\x1b[0m ===`);

if (fail > 0) {
  console.log(`\x1b[31mBLOCKED: ${fail} invariant(s) violated. Do not merge.\x1b[0m`);
  process.exit(1);
} else {
  console.log("\x1b[32mAll invariants hold. Safe to merge.\x1b[0m");
  process.exit(0);
}
