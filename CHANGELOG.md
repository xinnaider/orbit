# Changelog

---

## April 2026

### 04/08 · New — Linux/Ubuntu support
Orbit now runs on Ubuntu 22.04+ and other Linux distributions with a desktop environment (GNOME, KDE Plasma, and others). Install with a single command that downloads the app, adds it to your application launcher, and sets up automatic updates — no manual steps required. Once installed, Orbit updates itself in the background just like on Windows.

### 04/07 · New — Meta panel toggle
The stats panel on the right can now be hidden or shown with a single click. Use the `›` button in the panel header to collapse it; a thin `‹` strip appears on the right edge so you can bring it back at any time. The preference is saved and restored between sessions.

### 04/07 · Fix — Button positioning in split panes
The scroll-to-bottom button now stays anchored to the bottom-right of its own panel and no longer overlaps the adjacent panel when two panes are open side by side. The split and close buttons moved into the panel header so they always sit in the top-right corner regardless of whether the branch strip is visible, and no longer clash with each other.

### 04/07 · Improvement — Rename session modal
Renaming a session now opens a modal with two separate fields — the agent codename and the project suffix — pre-filled from the current name. A live preview shows the final result before saving.

### 04/07 · New — In-app changelog
The app now shows a changelog modal on first launch after an update. You can also reopen it anytime by clicking the version badge in the sidebar.

### 04/07 · Fix — Branch badge not updating
The active branch shown in each panel now stays in sync as you switch branches, instead of being frozen at the value from when the session was created.

### 04/07 · Improvement — Truncated panel name and branch in separate strip
When a session name is long, the panel header now shows `…` instead of growing in height. The full name appears on hover. The active branch was moved to a thin strip below the header, easier to read and no longer conflicting with other elements.

### 04/07 · Improvement — Session nickname with two separate fields
When creating a session, the nickname is now composed of two fields: the agent name (auto-generated as a codename) and the project suffix (filled with the folder name). The final result — like `raven · agent-dashboard-v2` — is shown as a preview before saving.

### 04/07 · Improvement — Active branch shown in panel header
Each panel header now correctly shows the branch Claude is working on. For sessions with an isolated worktree, it shows the worktree branch (`orbit/<name>`); for normal sessions, it shows the repository branch.

### 04/07 · New — Split panes
Orbit now lets you view up to 4 Claude Code sessions simultaneously.
Drag any session from the sidebar to the edge of a panel to open a side-by-side split. Up to 4 panels in a 2×2 grid. Click a panel to focus it — the MetaPanel follows the focused panel.

### 04/06 · New — Session nickname and worktree on creation
When creating a new session, you can now give it a custom nickname for easy identification. If left blank, the app automatically suggests a name based on Android device codenames combined with the project name.

There is also a new option to create the session inside an isolated **git worktree**. When enabled, Claude works on a separate branch (automatically created as `orbit/<session-name>`), keeping the main branch intact during the work.

### 04/06 · Improvement — Uninterrupted command execution
Terminal commands now run automatically without asking for confirmation at each step. The agent workflow is smoother and free of unnecessary pauses.

### 04/06 · Improvement — Real-time output
During long-running commands, results appear progressively on screen — no need to wait for the command to finish before seeing what's happening.

### 04/06 · New — API rate limit warning
When the Claude API rate limit is reached, the app shows a clear message on screen instead of silently stopping. The warning automatically dismisses after 30 seconds.

### 04/06 · New — Automatic updates
The app automatically checks for a new version on launch. When one is available, a banner appears with a button to install and restart — no manual download needed.

### 04/06 · Adjustment — Stopped session indicator
Stopped sessions now show a "stopped" label in the sidebar, making it easier to identify the state of each session.

---
