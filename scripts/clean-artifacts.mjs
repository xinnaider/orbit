#!/usr/bin/env node
/**
 * Remove local build artifacts that consume disk space.
 *
 * Usage:
 *   node scripts/clean-artifacts.mjs          # rust + frontend + test output
 *   node scripts/clean-artifacts.mjs --all    # also node_modules (re-run npm ci)
 */

import { execSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';
import { join } from 'node:path';

const all = process.argv.includes('--all');

const paths = [
  'tauri/target',
  'tauri/binaries',
  'build',
  '.svelte-kit',
  'test-results',
  'playwright-report',
  'blob-report',
];

if (all) {
  paths.push('node_modules');
}

const hadRustTarget = existsSync(join(process.cwd(), 'tauri', 'target'));
if (hadRustTarget) {
  try {
    execSync('cargo clean --manifest-path tauri/Cargo.toml', { stdio: 'inherit' });
  } catch {
    /* ignore */
  }
}

let removed = 0;
for (const rel of paths) {
  const p = join(process.cwd(), rel);
  if (!existsSync(p)) continue;
  rmSync(p, { recursive: true, force: true });
  console.log(`removed ${rel}`);
  removed++;
}

if (removed === 0) {
  console.log('nothing to clean');
} else if (all) {
  console.log('done — run npm ci to restore dependencies');
} else {
  console.log('done — next tauri:dev will do one fresh cargo build');
}
