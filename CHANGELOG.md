# Changelog

---

## April 2026

### 04/15 · New — macOS support
Orbit is now available for macOS (Intel and Apple Silicon). Download the .dmg from the releases page, open it, and drag Orbit to your Applications folder.

### 04/14 · New — SSH remote sessions for all providers

You can now run sessions on remote servers via SSH. When creating a session,
switch to "ssh remote" mode, enter the host and user (with optional password),
and test the connection before starting. Works with all providers — Claude Code,
Codex, and OpenCode. SSH credentials are held in memory only and never saved
to disk.

### 04/12 · New — Multi-provider support (OpenCode + Codex)

You can now create sessions with any AI provider — not just Claude Code.
The new session dialog shows a provider selector with favorites (Claude Code,
Codex, OpenRouter, Anthropic, OpenAI, Google, DeepSeek) and a search bar for
100+ additional providers. Models populate automatically for each provider.
API keys are read from environment variables when available, with an option to
override in the dialog. Sessions powered by OpenCode or Codex stream output
in real time, just like Claude Code sessions.

### 04/09 · New — Sub-agents monitor tab

When a session spawns sub-agents, they now appear in the "Sub-agents" tab on the right panel, grouped by Running and Completed. Click any agent to inspect its full conversation log in a modal. The list updates automatically as the session progresses, and a refresh button fetches the latest state on demand.

### 04/09 · Improvement — Redesigned app icon with bolder orbital rings
The app icon now features thicker, more legible orbital rings that stand out at small sizes (taskbar, title bar). The icon generation now uses Chrome renderer for crisp antialiasing across all platform variants.

### 04/09 · Improvement — Light theme now has warmer, easier-on-the-eyes colors
The light theme received a visual refresh with a warmer color palette that improves readability and reduces eye strain in bright environments.

### 04/09 · Improvement — Context menu items now have SVG icons
All context menu options (Rename, Mute, Force Stop, Delete) now display consistent SVG icons that match the header button styling, making the menu visually cohesive.

### 04/09 · Fix — Mute button moved to header actions
The mute button is now grouped with split and close controls in the panel header for a cleaner, more intuitive layout.

### 04/09 · Fix — "Stopped" badge removed from stats panel
The redundant "stopped" label that appeared in the stats panel header has been removed. The session's stopped state is still visible in the sidebar.

### 04/09 · Improvement — Mute option now in context menu
The mute/unmute action is now available directly in the session context menu. Right-click the session name in the sidebar to quickly silence notifications without opening the session header.

### 04/09 · Improvement — Toast notifications now have action buttons
Toasts have been repositioned and enhanced with support for action buttons. Update notifications have distinct styling to stand out from other messages.

### 04/09 · Fix — Markdown tables and code blocks now style correctly
Code blocks and tables in markdown now render with proper theme colors and consistent styling across light and dark modes.

### 04/09 · Fix — Auto-scroll continues following new messages
The session feed's auto-scroll now maintains follow-mode reliably when new content arrives, preventing unexpected jumps to old messages.

### 04/08 · Fix — Rate limit warning no longer triggers incorrectly
The "rate limit reached" banner was sometimes shown when Claude's response simply
mentioned rate limits or server load in its text — without an actual API error occurring.
The detection is now precise: it only fires when the API returns a real rate limit or
overloaded error, so the banner appears only when it matters.

### 04/08 · Improvement — Smoother performance with long sessions
The session feed now renders only the entries currently visible on screen instead of mounting everything at once. Sessions with hundreds of messages open faster, scroll smoothly, and use less memory — without any change to how messages look or behave.

### 04/08 · New — Per-session mute button
Each session panel now has a speaker icon button in the header. Click it to silence the notification sound for that session only — useful when you want to monitor one session quietly while staying alerted by others. A small muted indicator also appears next to the session name in the sidebar. The preference is saved across app restarts.

### 04/08 · Improvement — Compact toast notifications replace banners
Error, warning, and info messages now appear as small stacked toasts in the bottom-right corner instead of full-width banners that blocked the top of the screen. Toasts with a required action (rate limit, update available, session error) stay visible until manually dismissed; informational notices disappear automatically after five seconds.

### 04/08 · Fix — Auto-updater now works on Windows and Linux
The "check for updates" feature was silently failing on all platforms because the update server URL was missing the architecture suffix. Users on v0.3.0 will now correctly receive an update prompt when opening the app.

### 04/08 · Fix — Message display delay and working indicator
Sending a message now shows it in the chat immediately and without flickering. The "working" indicator (animated dots) now appears as soon as Claude starts processing a request and stays visible throughout — previously it could disappear briefly or not show at all right after sending. The fix also reduces unnecessary background work that was slowing down the feed during active sessions.

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
