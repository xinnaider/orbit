<div align="center">

<img src="front/icons/orbit-source.svg" width="88" height="88" alt="Orbit"/>

# Orbit

Run multiple Claude Code agents simultaneously — each in its own session, all in one place.

[orbit.jfernando.dev](https://orbit.jfernando.dev) · [Download](https://github.com/xinnaider/orbit/releases/latest) · [![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

</div>

---

**Multi-session** — parallel agents across different projects  
**Real-time feed** — streaming output with thinking blocks and tool calls  
**Cost tracking** — per-session token usage and USD cost  
**Persistent history** — sessions survive restarts  
**Slash commands & @ files** — full Claude Code plugin support

---

## Install

Requires **Windows 10 1903+** and [Claude Code](https://github.com/anthropics/claude-code) installed and logged in.

1. Download the `.exe` from [Releases](https://github.com/xinnaider/orbit/releases/latest)
2. Run the installer
3. Open Orbit and click **+** to start a session

---

## Development

**Requirements:** Node.js ≥ 20, Rust stable, npm ≥ 10

```bash
git clone https://github.com/xinnaider/orbit.git
cd orbit
npm install
npm run tauri:dev
```

```bash
npm test            # Vitest
npm run test:rust   # cargo test
npm run lint        # ESLint + clippy
npm run format      # Prettier + rustfmt
```

---

MIT © josefernando
