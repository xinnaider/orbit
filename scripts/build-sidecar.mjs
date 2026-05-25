#!/usr/bin/env node
/**
 * Release/build: compile orbit and link into the externalBin slot.
 * Uses hardlink when possible to avoid duplicating the executable on disk.
 *
 * Usage:
 *   node scripts/build-sidecar.mjs           # debug build
 *   node scripts/build-sidecar.mjs --release  # release build
 */

import { execSync } from 'node:child_process';
import {
  copyFileSync,
  existsSync,
  linkSync,
  mkdirSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';

const isRelease = process.argv.includes('--release');

function getTargetTriple() {
  if (process.env.TAURI_ENV_TARGET_TRIPLE) {
    return process.env.TAURI_ENV_TARGET_TRIPLE;
  }
  const rustcOutput = execSync('rustc -vV', { encoding: 'utf-8' });
  const match = rustcOutput.match(/^host:\s*(.+)$/m);
  if (!match) throw new Error('Could not detect Rust target triple from `rustc -vV`');
  return match[1].trim();
}

function linkOrCopy(src, dest) {
  if (existsSync(dest)) {
    try {
      unlinkSync(dest);
    } catch {
      copyFileSync(src, dest);
      return;
    }
  }
  try {
    linkSync(src, dest);
  } catch {
    copyFileSync(src, dest);
  }
}

const triple = getTargetTriple();
const ext = triple.includes('windows') ? '.exe' : '';

console.log(`[sidecar] target: ${triple}`);
console.log(`[sidecar] mode: ${isRelease ? 'release' : 'debug'} (unified binary, --mcp-stdio)`);

const destDir = join('tauri', 'binaries');
const dest = join(destDir, `orbit-mcp-${triple}${ext}`);

if (!existsSync(destDir)) {
  mkdirSync(destDir, { recursive: true });
}
if (!existsSync(dest)) {
  writeFileSync(dest, '');
  console.log(`[sidecar] created placeholder: ${dest}`);
}

const profile = isRelease ? '--release' : '';
execSync(`cargo build --manifest-path tauri/Cargo.toml ${profile}`.trim(), {
  stdio: 'inherit',
});

const profileDir = isRelease ? 'release' : 'debug';
const mainBin = join('tauri', 'target', profileDir, `orbit${ext}`);

linkOrCopy(mainBin, dest);
console.log(`[sidecar] ${mainBin} -> ${dest}`);
