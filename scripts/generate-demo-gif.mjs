#!/usr/bin/env node
/**
 * Generates demo.gif — animated screenshot of Orbit running with mock data.
 * Usage: npm run demo:gif
 */

import puppeteer from 'puppeteer';
import sharp from 'sharp';
import gifenc from 'gifenc';
const { GIFEncoder, quantize, applyPalette } = gifenc;
import { spawn, spawnSync } from 'child_process';
import { copyFileSync, writeFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, '..');

const VIEWPORT = { width: 1440, height: 900 };
const GIF_W = 1280;
const GIF_H = 800;
const FPS = 2;
const FRAME_DELAY = Math.round(1000 / FPS); // gifenc expects milliseconds.
const OUT = join(ROOT, 'media', 'demo.gif');
const LANDING_OUT = join(ROOT, 'landing', 'public', 'demo.gif');
const stoppedServers = new WeakSet();

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function stopServer(server) {
  if (!server || server.killed || stoppedServers.has(server)) return;
  stoppedServers.add(server);

  if (process.platform === 'win32' && server.pid) {
    spawnSync('taskkill', ['/pid', String(server.pid), '/T', '/F'], { stdio: 'ignore' });
    return;
  }

  server.kill();
}

async function waitForServer(url, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(url);
      if (res.status < 500) return;
    } catch {}
    await sleep(300);
  }
  throw new Error(`Server at ${url} did not start within ${timeout}ms`);
}

async function encodeFrame(pngBuffer) {
  const { data, info } = await sharp(pngBuffer)
    .resize(GIF_W, GIF_H)
    .flatten({ background: '#090a0a' })
    .ensureAlpha()
    .raw()
    .toBuffer({ resolveWithObject: true });
  return { data: new Uint8Array(data), width: info.width, height: info.height };
}

async function main() {
  // ── 1. Start mock dev server ──────────────────────────────────────────────
  console.log('Starting mock dev server...');
  const server = spawn('npm', ['run', 'dev:mock'], {
    cwd: ROOT,
    shell: true,
    stdio: 'pipe',
  });
  process.on('exit', () => stopServer(server));
  process.on('SIGINT', () => {
    stopServer(server);
    process.exit(0);
  });

  await waitForServer('http://localhost:1420');
  await sleep(600);
  console.log('Server ready. Launching browser...\n');

  // ── 2. Launch Puppeteer ───────────────────────────────────────────────────
  const browser = await puppeteer.launch({
    headless: true,
    defaultViewport: VIEWPORT,
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage'],
  });

  const page = await browser.newPage();
  await page.goto('http://localhost:1420', { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('[data-testid="session-item"], .feed-wrap', { timeout: 20_000 });
  const changelogClose = await page.$('.close-btn');
  if (changelogClose) {
    await changelogClose.click();
    await sleep(300);
  }
  await sleep(1000);

  // ── 3. Capture scenes ────────────────────────────────────────────────────
  const rawFrames = [];

  const snap = async () => {
    rawFrames.push(await page.screenshot({ type: 'png' }));
  };

  const hold = async (ms) => {
    const count = Math.max(1, Math.round((ms / 1000) * FPS));
    for (let i = 0; i < count; i++) {
      await snap();
      await sleep(1000 / FPS);
    }
  };

  // Scene A: app loads — sidebar with 3 sessions (3s)
  console.log('Scene A: sidebar overview');
  await hold(3000);

  // Scene B: click session 1 "fix auth bug" (5s)
  console.log('Scene B: session 1 feed');
  const items = await page.$$('[data-testid="session-item"]');
  if (items[0]) {
    await items[0].click();
    await sleep(600);
  }
  await hold(5000);

  // Scene C: scroll feed down (3s)
  console.log('Scene C: scroll feed');
  await page.evaluate(() => {
    const el = document.querySelector('.feed-scroller') ?? document.querySelector('.feed-wrap');
    if (el) el.scrollTop += 280;
  });
  await sleep(400);
  await hold(3000);

  // Scene D: click session 2 — show feed (4s)
  console.log('Scene D: session 2 feed');
  const items2 = await page.$$('[data-testid="session-item"]');
  if (items2[1]) {
    await items2[1].click();
    await sleep(600);
  }
  await hold(4000);

  // Scene E: focus input and type a message (4s)
  console.log('Scene E: typing a message');
  const textarea = await page.$('textarea');
  if (textarea) {
    await textarea.click();
    await page.keyboard.type('Add unit tests for the Chart component', { delay: 80 });
    await sleep(500);
    await hold(4000);
  }

  await browser.close();
  stopServer(server);

  // ── 4. Encode GIF ─────────────────────────────────────────────────────────
  console.log(`\nEncoding ${rawFrames.length} frames → ${GIF_W}×${GIF_H}...`);
  const encoder = GIFEncoder();

  for (let i = 0; i < rawFrames.length; i++) {
    process.stdout.write(`\r  Frame ${i + 1}/${rawFrames.length}`);
    const { data, width, height } = await encodeFrame(rawFrames[i]);
    const palette = quantize(data, 256, { clearAlphaColor: 255 });
    const index = applyPalette(data, palette);
    encoder.writeFrame(index, width, height, { palette, delay: FRAME_DELAY });
  }

  encoder.finish();
  writeFileSync(OUT, Buffer.from(encoder.bytes()));
  copyFileSync(OUT, LANDING_OUT);

  const sizeKb = Math.round(encoder.bytes().byteLength / 1024);
  const duration = (rawFrames.length / FPS).toFixed(1);
  console.log(`\n\nSaved demo.gif — ${duration}s, ${sizeKb}KB\n`);
  console.log(`Copied to ${LANDING_OUT}\n`);
  process.exit(0);
}

main().catch((e) => {
  console.error('\nError:', e.message);
  process.exit(1);
});
