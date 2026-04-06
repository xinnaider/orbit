# Orbit

Desktop application for managing multiple [Claude Code](https://github.com/anthropics/claude-code) sessions simultaneously.

[![Build](https://img.shields.io/github/actions/workflow/status/xinnaider/orbit/build.yml?branch=master)](https://github.com/xinnaider/orbit/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows-blue.svg)](#installation)

---

```
┌─ orbit ──────────────────────────────────────────────────────────────┐
│  sidebar        │  session feed              │  meta panel           │
│  ─────────────  │  ──────────────────────    │  ───────────────────  │
│  ● api-server   │  you · 14:32               │  tokens               │
│    running      │    fix the auth bug        │  24.8K                │
│    sonnet · 14K │                            │  input    19.2K       │
│  ● dashboard    │  claude · 14:33            │  output    5.6K       │
│    waiting      │    I'll look at auth.ts…   │                       │
│    opus · 89K   │                            │  cost  $0.48          │
│  ● utils-lib    │  run · bash                │                       │
│    idle         │    $ git status            │  context  14.2%       │
└─────────────────┴────────────────────────────┴───────────────────────┘
```

---

## Features

- **Multi-session** — run multiple Claude Code agents in parallel across different projects
- **Real-time feed** — streaming output with thinking blocks, tool calls, and responses
- **Persistent history** — sessions survive app restarts; conversations resume automatically
- **Cost tracking** — per-session token usage and estimated cost in USD
- **Slash commands** — `/` autocomplete from installed Claude Code plugins
- **@ file picker** — reference files inline with `@filename`
- **Context menu** — right-click to rename, stop, or delete sessions

---

## Installation

### Requirements

- **Windows 10 1903+**
- **[Claude Code CLI](https://github.com/anthropics/claude-code)** installed and logged in:
  ```bash
  npm install -g @anthropic-ai/claude-code
  claude login
  ```

### Download

1. Go to [Releases](https://github.com/xinnaider/orbit/releases/latest)
2. Download the `.exe` installer
3. Run the installer
4. Open Orbit, click **+** to create your first session

---

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development setup

**Requirements:** Node.js ≥ 20, Rust stable ([rustup](https://rustup.rs)), npm ≥ 10

```bash
git clone https://github.com/xinnaider/orbit.git
cd orbit
npm install
npm run tauri:dev
```

### Testing

```bash
npm test           # Frontend (Vitest)
npm run test:rust  # Backend (cargo test)
```

### Linting & formatting

```bash
npm run lint       # ESLint + clippy
npm run format     # Prettier + rustfmt
```

---

## License

MIT © josefernando
