<div align="center">

<img src="tauri/icons/orbit-source.svg" width="96" height="96" alt="Orbit logo"/>

# Orbit

**Desktop app for running multiple [Claude Code](https://github.com/anthropics/claude-code) agents simultaneously.**

[![Build](https://img.shields.io/github/actions/workflow/status/xinnaider/orbit/build.yml?branch=master)](https://github.com/xinnaider/orbit/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-yellow.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows-blue.svg)](#installation)
[![Platform: Linux](https://img.shields.io/badge/platform-Linux-orange.svg)](#installation)

[orbit.jfernando.dev](https://orbit.jfernando.dev)

</div>

![Orbit demo](media/demo.gif)

## Features

- **Multi-session** — run multiple Claude Code agents in parallel across different projects
- **Real-time feed** — streaming output with thinking blocks, tool calls, and responses
- **Persistent history** — sessions survive app restarts; conversations resume automatically
- **Cost tracking** — per-session token usage and estimated cost in USD
- **Slash commands** — `/` autocomplete from installed Claude Code plugins
- **@ file picker** — reference files inline with `@filename`
- **Context menu** — right-click to rename, stop, or delete sessions

## Installation

### Requirements

- **[Claude Code CLI](https://github.com/anthropics/claude-code)** installed and logged in:
  ```bash
  npm install -g @anthropic-ai/claude-code
  claude login
  ```

---

### Windows (10 1903+)

**One-line installer** — open PowerShell and run:

```powershell
irm https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-windows.ps1 | iex
```

This downloads the latest release, shows a progress bar, and launches the installer automatically. Open Orbit from the Start Menu when done.

Orbit updates itself automatically when a new version is available.

---

### Linux (Ubuntu 22.04+ · Debian · KDE · other distros)

**One-line installer** — downloads the AppImage, creates a desktop entry (shows up in your app launcher), and sets up auto-updates:

```bash
curl -fsSL https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-linux.sh | bash
```

Open Orbit from your application menu or run:

```bash
~/.local/share/orbit/orbit.AppImage
```

Orbit updates itself automatically when a new version is available.

> **Requirements:** `curl`, `fuse2` (pre-installed on most desktop distros).
> On Ubuntu: `sudo apt install fuse libfuse2` if not present.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development setup

**Requirements:** Node.js ≥ 20, Rust stable ([rustup](https://rustup.rs)), npm ≥ 10

```bash
git clone https://github.com/xinnaider/orbit.git
cd orbit
npm install
npm run tauri:dev   # starts frontend + backend together
```

`tauri:dev` runs the Vite dev server and the Rust backend in one command, with hot reload on frontend changes.

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

## License

MIT © josefernando
