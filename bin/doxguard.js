#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const require = createRequire(import.meta.url);
const platformKey = `${process.platform}-${process.arch}`;
const packages = {
  "darwin-arm64": ["@ishizakahiroshi/doxguard-darwin-arm64", "doxguard"],
  "darwin-x64": ["@ishizakahiroshi/doxguard-darwin-x64", "doxguard"],
  "linux-arm64": ["@ishizakahiroshi/doxguard-linux-arm64", "doxguard"],
  "linux-x64": ["@ishizakahiroshi/doxguard-linux-x64", "doxguard"],
  "win32-arm64": ["@ishizakahiroshi/doxguard-win32-arm64", "doxguard.exe"],
  "win32-x64": ["@ishizakahiroshi/doxguard-win32-x64", "doxguard.exe"],
};

const selected = packages[platformKey];
if (!selected) {
  process.stderr.write(`doxguard: unsupported platform ${platformKey}\n`);
  process.exit(2);
}

const [packageName, binaryName] = selected;
let binary = process.env.DOXGUARD_BINARY_PATH;
if (!binary) {
  try {
    const manifest = require.resolve(`${packageName}/package.json`);
    binary = join(dirname(manifest), "bin", binaryName);
  } catch {
    process.stderr.write(
      `doxguard: native package ${packageName} is missing. Reinstall doxguard without --no-optional.\n`,
    );
    process.exit(2);
  }
}

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: true,
});
if (result.error) {
  process.stderr.write(`doxguard: failed to start native binary: ${result.error.message}\n`);
  process.exit(2);
}
if (result.signal) {
  process.kill(process.pid, result.signal);
} else {
  process.exit(result.status ?? 2);
}
