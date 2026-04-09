#!/usr/bin/env node
/**
 * Local updater test server for Orbit.
 *
 * Serves a fake latest-{target}.json at http://localhost:8091/latest-{target}.json
 * so you can test the Tauri updater without deploying to GitHub.
 *
 * Usage:
 *   node scripts/updater-test-server.js
 *
 * Then set tauri.conf.json endpoints to:
 *   ["http://localhost:8091/latest-{{target}}.json"]
 *
 * And run:
 *   npm run tauri:dev
 */

import http from 'http';

const PORT = 8091;

// Fake release payload — version is higher than anything real so the
// updater always thinks an update is available.  The signature and URL are
// intentionally fake; `check()` only parses JSON + compares semver.
// `download_and_install()` would fail, but we only need `check()` to work.
const fakeRelease = {
  version: '9.9.9',
  notes: '[TEST] Local updater test — this is not a real release.',
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
  url: 'http://localhost:8091/fake-setup.exe',
  signature:
    'dW50cnVzdGVkIGNvbW1lbnQ6IFRFU1QgZmFrZSBzaWduYXR1cmUgZm9yIGxvY2FsIHRlc3RpbmcK',
};

const server = http.createServer((req, res) => {
  const url = req.url;
  console.log(`[${new Date().toISOString()}] ${req.method} ${url}`);
  console.log(`  Headers: Accept=${req.headers['accept'] ?? '(none)'}`);

  if (url.startsWith('/latest-') && url.endsWith('.json')) {
    const body = JSON.stringify(fakeRelease, null, 2);
    res.writeHead(200, {
      'Content-Type': 'application/json',
      'Content-Length': Buffer.byteLength(body),
    });
    res.end(body);
    console.log(`  → 200 served fake release v${fakeRelease.version}`);
  } else if (url === '/fake-setup.exe') {
    // Respond with dummy bytes so curl/wget don't hang if someone tests download
    res.writeHead(200, { 'Content-Type': 'application/octet-stream' });
    res.end(Buffer.alloc(0));
    console.log('  → 200 served empty fake binary');
  } else {
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    res.end('Not found');
    console.log('  → 404');
  }
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`\nOrbit updater test server running at http://localhost:${PORT}`);
  console.log('\nExpected requests from Tauri ({{target}}-{{arch}} pattern):');
  console.log(`  GET /latest-windows-x86_64.json   (Windows x64)`);
  console.log(`  GET /latest-linux-x86_64.json     (Linux x64)`);
  console.log('\nReturning fake release:');
  console.log(JSON.stringify(fakeRelease, null, 2));
  console.log('\nPress Ctrl+C to stop.\n');
});
