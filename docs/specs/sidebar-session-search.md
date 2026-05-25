# Spec: Sidebar session search (ORB-7)

## Objective

Make the sidebar "Search sessions" control filter the pinned and recent session lists in real time.

## Problem

The Quiet Console sidebar shows a non-interactive placeholder (`<div>`) styled like a search field. Users cannot type or filter sessions.

## Expected behavior

1. The control is a text `<input>` with placeholder "Search sessions…".
2. Typing filters both **Pinned** and **Recent sessions** lists (case-insensitive substring match).
3. A session matches if any of these fields contain the query (trimmed): display name, project name, folder name from `cwd`, branch (`branchName` or `gitBranch`), model, provider id, status.
4. Empty query shows all sessions (current behavior).
5. When the query is non-empty and no root session matches, show **No matching sessions** in the recent section (pinned section hidden if empty).
6. Pinned section is hidden when there are no pinned matches (same as today when no pinned sessions).

## Edge cases

| Case | Result |
|------|--------|
| Whitespace-only query | Treated as empty (show all) |
| Child sessions (`parentSessionId` set) | Not listed in sidebar; search only applies to root sessions |
| All roots pinned, search matches none | "No matching sessions" |
| Search matches only pinned | Recent area shows "No matching sessions" or empty recent list message |

## Acceptance criteria

- [ ] Input accepts keyboard focus and text entry
- [ ] Filtering updates lists as the user types
- [ ] Clearing the input restores the full list
- [ ] Vitest covers filter behavior via `data-testid="session-search-input"`

## Test points (manual)

1. Create 3+ sessions with distinct names; type part of one name — only matching cards remain.
2. Search by git branch label shown in subline — matching session appears.
3. Clear search — all sessions return.
4. Search gibberish — "No matching sessions" appears.
5. Pin a session; search a non-pinned name — pinned section hides if no pin matches.
