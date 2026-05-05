# Orbit Compact Panel Header Design

Date: 2026-05-04

## Goal

Standardize Orbit workspace tabs and panel headers around the approved Terminal Premium visual direction, while preserving the information and actions already present in today's panel headers.

## Current Context

Orbit's workspace renders tabs above panel content. `CentralPanel.svelte` currently owns its own header with session identity, status, token count, context usage, model, mute, close, and an optional branch strip. Terminal and Git panels should use the same header anatomy so panels feel like part of one workspace system.

The user selected the "Terminal Premium" direction for tabs and then selected the "Compact Stack" composition: tabs at the top of the pane, a compact panel header directly below, and content below the header.

## Selected Visual Direction

Use a compact developer cockpit style:

- Dark surfaces with subtle green-tinted borders.
- Fine 1px borders instead of heavy outlines.
- Small rounded rectangles, about `5px` to `6px` radius.
- Active tab uses a subtle green border, a soft green-tinted background, and a real status/icon mark.
- Inactive tabs stay visible with low-contrast borders and muted text, even without hover.
- Header uses the same visual language as tabs but is less prominent than the active tab.
- No emojis or improvised glyphs. Use real `lucide-svelte` icons everywhere.

## Composition

The pane chrome is ordered from top to bottom:

1. `TabBar`: workspace tabs and the add-tab button.
2. `PanelHeader`: standardized header for the active tab content.
3. Optional secondary strip, such as the current Git branch strip for agent panels.
4. Panel body content.

The selected density is Compact Stack:

- Tab bar height: about `34px`.
- Panel header height: about `36px`.
- Optional branch strip height: about `20px` to `22px`.

The target is to keep total chrome small enough for split panes while making active tab, panel identity, and actions readable.

## Tab Bar Design

Tabs use the approved Terminal Premium treatment:

- Active tab is a compact chip with soft green tint and green border.
- Active tab may show a small status dot for agent tabs or a Lucide icon for non-agent tabs.
- Inactive tabs are still legible, with a faint border and muted label.
- Close button is always visible, but lower opacity on inactive tabs.
- Add button is a square icon button at the right edge using `Plus` from Lucide.

Lucide icons:

- Agent: `Bot`
- Terminal: `Terminal`
- Git: `GitBranch`
- Close: `X`
- Add: `Plus`

## Panel Header Design

The header should keep the current `CentralPanel` information model:

- Left side:
  - Status indicator or leading Lucide icon.
  - Primary title, such as session name, `Terminal`, or `Git`.
  - Status pill, such as `running`, `ready`, or change count.
- Right side:
  - Compact metrics, such as tokens.
  - Context usage bar and percentage when applicable.
  - Model pill when applicable.
  - Action icon buttons.
  - Close pane action when available.

For agent panels, preserve today's visible data:

- Session status dot and status text.
- Session name fallback behavior.
- Token count.
- Context usage bar and percentage.
- Model short name.
- Mute action.
- Close action.
- Branch strip when branch data exists.

For terminal panels:

- Leading `Terminal` icon.
- Title: `Terminal`.
- Subtitle or compact text: current `cwd`.
- Status pill: `starting`, `ready`, or `error`.
- Close action.

For Git panels:

- Leading `GitBranch` icon.
- Title: `Git` or `Git Overview`.
- Status pill or metric: number of changed files.
- Compact branch name or repository path.
- Refresh/action buttons.
- Close action if the panel is closable.

## Shared Component

Create a shared `PanelHeader.svelte` under `ui/components/workspace/`.

The component should be presentational. It should not know about sessions, Git state, terminal PTYs, or stores. Consumers pass the title, subtitle/status, optional leading slot, optional metrics slot, actions slot, and close callback.

The component should support drag payloads so panel headers can still be dragged when the active panel supports pane/tab drag behavior.

## Branch Strip

Keep the branch strip as a separate secondary row below the main panel header for agent panels.

Visual treatment:

- Height around `20px` to `22px`.
- Darker than the main header.
- Very subtle border-bottom.
- Use `GitBranch` or another appropriate Lucide icon instead of the current text symbol.
- Muted text that truncates cleanly in narrow panes.

## Accessibility And Interaction

- Use real buttons for close, mute, refresh, and add actions.
- Every icon-only button must have an `aria-label` and `title` where helpful.
- Preserve keyboard activation for tabs.
- Preserve drag behavior, but do not make non-draggable controls interfere with button clicks.
- Maintain visible focus styles consistent with existing theme variables.

## Non-Goals

- No Sidebar redesign.
- No Feed redesign.
- No MetaPanel redesign.
- No new icon package.
- No animation-heavy tab system.
- No change to workspace data persistence solely for this visual work.

## Acceptance Criteria

- Tabs sit above the panel header in every pane.
- Agent, Terminal, and Git panels use the shared compact header anatomy.
- Agent panel retains the same operational information and actions available today.
- Headers and tabs use Lucide icons, not emojis or improvised text glyphs.
- Inactive tabs remain readable without hover.
- Narrow panes truncate labels instead of overflowing.
- `npm run check` passes after implementation.

## Self-Review

- Placeholder scan: no placeholder sections or unfinished requirements remain.
- Internal consistency: the composition consistently places tabs above `PanelHeader`, and the branch strip remains a secondary row.
- Scope check: this spec is limited to visual standardization of workspace tabs and panel headers.
- Ambiguity check: icon usage explicitly requires Lucide and forbids emojis/improvised glyphs.
