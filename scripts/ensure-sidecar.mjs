#!/usr/bin/env node
/**
 * Dev-only: satisfy Tauri externalBin without running cargo.
 * - Creates a zero-byte placeholder if the sidecar slot is missing.
 * - If orbit.exe already exists (from a prior tauri dev build), hardlinks it
 *   into binaries/ (same inode — no duplicate disk vs copy).
 *
 * Usage: node scripts/ensure-sidecar.mjs
 */

import { execSync } from 'node:child_process';
import {
  copyFileSync,
  existsSync,
  linkSync,
  mkdirSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';

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
      return false;
    }
  }
  try {
    linkSync(src, dest);
    return true;
  } catch {
    try {
      copyFileSync(src, dest);
      return true;
    } catch {
      return false;
    }
  }
}

const triple = getTargetTriple();
const ext = triple.includes('windows') ? '.exe' : '';
const destDir = join('tauri', 'binaries');
const dest = join(destDir, `orbit-mcp-${triple}${ext}`);
const mainBin = join('tauri', 'target', 'debug', `orbit${ext}`);

if (!existsSync(destDir)) {
  mkdirSync(destDir, { recursive: true });
}

if (existsSync(mainBin)) {
  const srcStat = statSync(mainBin);
  const destStat = existsSync(dest) ? statSync(dest) : null;
  const needsLink =
    !destStat || destStat.size !== srcStat.size || destStat.mtimeMs < srcStat.mtimeMs;

  if (needsLink && linkOrCopy(mainBin, dest)) {
    console.log(`[sidecar] linked ${mainBin} -> ${dest} (no extra disk)`);
  } else if (!existsSync(dest)) {
    writeFileSync(dest, '');
    console.log(`[sidecar] placeholder ${dest}`);
  }
} else if (!existsSync(dest)) {
  writeFileSync(dest, '');
  console.log(`[sidecar] placeholder ${dest} (tauri dev will compile orbit once)`);
} else {
  console.log(`[sidecar] ok ${dest}`);
}
