# Orbit Tab Redesign Design

Date: 2026-05-04

## Goal

Redesign Orbit workspace tabs with a minimal, modern "Soft Minimal" direction and make the surrounding panel experience coherent. The implementation should also fix the terminal tab path so a local shell can be opened from the workspace and replace improvised text symbols in the tab dropdown with real icons from the existing `lucide-svelte` dependency.

## Current Context

Orbit already has a pane-based workspace:

- `ui/lib/stores/workspace.ts` stores panes, tabs, active tab ids, split tree state, focus, drag/reorder, and persistence.
- `ui/components/workspace/TabBar.svelte` renders the tab row and add-tab button.
- `ui/components/workspace/TabItem.svelte` renders individual tabs.
- `ui/components/workspace/TabAddMenu.svelte` renders the add-tab dropdown.
- `ui/components/workspace/PaneContainer.svelte` chooses the active tab content.
- `ui/components/CentralPanel.svelte` renders agent sessions.
- `ui/components/GitPanel.svelte` renders Git overview.
- `ui/components/TerminalPanel.svelte` already imports `@xterm/xterm`, `@xterm/addon-fit`, and the Tauri PTY bridge.

The terminal issue is primarily integration-level: `PaneContainer.svelte` creates terminal tabs but does not render a `TerminalPanel` branch for `activeTab.kind === 'terminal'`. `TerminalPanel.svelte` also needs the active tab `cwd` instead of using a fixed `"."`.

## Selected Approach

Use approach 2 from brainstorming:

1. Apply the Soft Minimal visual treatment to workspace tabs.
2. Add a shared panel header template used by Agent, Git, and Terminal panels.
3. Render terminal tabs correctly and pass tab-specific `terminalId` and `cwd`.
4. Replace broken/manual glyphs with `lucide-svelte` icons.

This balances visible polish with a bounded technical change. It avoids a larger terminal manager refactor for this cycle.

## Visual Direction

The selected direction is "Soft Minimal":

- Tab bar height around 39px.
- Active tab appears as a quiet chip with subtle background and border.
- Inactive tabs remain flat and low contrast.
- Labels truncate with ellipsis in narrow panes.
- Close and add actions use icons instead of text glyphs.
- Focused pane keeps the existing clear focus signal through a thin accent line at the pane top.
- Panel surfaces stay dense and utilitarian, with slightly more breathing room than the current implementation.

The design should remain compatible with Orbit's current theme variables and not introduce a new palette.

## Panel Header Template

Create `ui/components/workspace/PanelHeader.svelte`.

The header provides one reusable anatomy:

- Left side:
  - Optional leading icon or status dot.
  - Primary title.
  - Secondary subtitle for status, cwd, branch, or context.
- Right side:
  - Optional metric pills.
  - Optional icon actions.
  - Optional close-pane action.

Expected consumers:

- `CentralPanel.svelte`: session name, status, token count, context percent, model, thinking toggle, mute, close.
- `GitPanel.svelte`: "Git overview", current branch/change summary, pull/push/stash actions.
- `TerminalPanel.svelte`: shell name, cwd, xterm/local shell state, close.

The template should be intentionally dumb: it receives display data and action slots/props, but does not know session, git, or terminal store semantics.

## Tabs And Dropdown

### `TabItem.svelte`

Replace current symbols with `lucide-svelte` icons:

- Agent tab: `Bot`.
- Git tab: `GitBranch`.
- Terminal tab: `Terminal`.
- Close: `X`.

Keep existing behavior:

- Click activates tab.
- Close dispatches close event.
- Drag start includes `{ tabId, sourcePaneId }`.
- Label lookup for agent tabs still uses `sessions`.

### `TabBar.svelte`

Apply Soft Minimal styling:

- Use the current `TabItem` rendering loop.
- Keep same reorder/drop behavior.
- Use `Plus` icon for add.
- Keep `TabAddMenu` positioning based on add button rect.

### `TabAddMenu.svelte`

Replace current text glyphs with icons:

- `GitBranch` for Git overview.
- `Terminal` for New terminal.
- `Bot` for New session.
- `FolderOpen` for Open session.

Improve menu spacing and hover states while preserving events:

- Existing event contract remains `select: { action: 'terminal' | 'session' | 'open' | 'git' }`.
- Existing close behavior remains click-outside based.

## Terminal Behavior

When the user selects `+ > New terminal`:

1. `PaneContainer.svelte` derives `cwd` from the active agent session in the pane.
2. It creates a `terminal` tab with label `Terminal`, `cwd`, and a stable tab id.
3. The new tab becomes active.
4. `PaneContainer.svelte` renders `TerminalPanel` for `activeTab.kind === 'terminal'`.
5. `TerminalPanel.svelte` opens a local shell in that `cwd`.
6. Input/output streams through the existing Tauri PTY bridge.
7. When the tab is closed and the component unmounts, the PTY created by that panel is killed.

`TerminalPanel.svelte` should accept:

- `terminalId: string`
- `cwd: string`
- Optional `sessionId?: number` only when an agent-backed PTY needs to be reused.

For this redesign, standalone terminal tabs should use the `terminalId` path. A stable numeric PTY id can continue to be derived from the tab id, as the current component already does with string hashing.

The shell selection remains platform-aware:

- Windows: `powershell.exe`
- Non-Windows: `/bin/bash`

User-configurable shell preferences are out of scope for this redesign.

## Xterm Integration

Use the official xterm pattern:

- Import `Terminal` from `@xterm/xterm`.
- Import `@xterm/xterm/css/xterm.css`.
- Create a visible container element.
- Call `term.open(container)` after the container exists and has dimensions.
- Load `FitAddon` with `term.loadAddon(fitAddon)`.
- Call `fitAddon.fit()` after opening and on container resize.

Official references:

- https://xtermjs.org/
- https://xtermjs.org/docs/api/terminal/classes/terminal/
- https://xtermjs.org/docs/guides/using-addons/

## Data Flow

```text
TabAddMenu select("terminal")
  -> PaneContainer.handleAddAction
  -> createTab(paneId, { kind: "terminal", label: "Terminal", cwd })
  -> workspace.activeTabId becomes new terminal tab
  -> PaneContainer renders TerminalPanel
  -> TerminalPanel creates xterm instance and Tauri PTY
  -> onData -> ptyWrite
  -> onPtyOutput -> terminal.write
  -> ResizeObserver/FitAddon -> ptyResize
  -> onDestroy -> ptyKill if owned
```

## Error Handling

- If there is no `cwd`, `PaneContainer` does not create Git or Terminal tabs. The dropdown items may remain visible; selecting one simply leaves the workspace unchanged.
- If `ptyCreate` fails, `TerminalPanel` shows an inline error state with a retry button.
- If `ptyWrite`, `ptyResize`, or `ptyKill` fails after startup, log the error and keep the UI stable.
- If xterm opens before dimensions are available, retry fitting on the next animation frame and through `ResizeObserver`.
- Closing a tab should not affect unrelated tabs or panes.

## Testing

Run:

```bash
npm run check
```

Manual verification:

- Open an agent session.
- Click `+ > New terminal`.
- Confirm a Terminal tab appears and becomes active.
- Confirm the terminal starts in the agent session `cwd`.
- Type a simple command and confirm output appears.
- Resize a split pane and confirm xterm refits.
- Close the terminal tab and confirm no visible workspace error.
- Open `+` menu and confirm icons render correctly.
- Check narrow panes for label truncation and no button text overflow.
- Open Git overview and confirm the new header does not regress existing Git actions.
- Open an agent panel and confirm status, tokens, model, mute, thinking toggle, and close actions remain available.

## Non-Goals

- No new workspace persistence model.
- No terminal multiplexing manager.
- No user-configurable shell preference in this cycle.
- No visual redesign of Sidebar, Feed entries, MetaPanel content, or Git body rows beyond header compatibility.
- No new icon package, because `lucide-svelte` is already installed.

## Acceptance Criteria

- Workspace tabs match the approved Soft Minimal direction.
- Agent, Git, and Terminal panels share a consistent header anatomy.
- The add-tab dropdown uses real icons from `lucide-svelte`.
- Terminal tabs render `TerminalPanel` and start a usable local shell in the current session `cwd`.
- Existing agent and Git tab behavior continues to work.
- `npm run check` passes.
