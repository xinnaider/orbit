# SSH Remote Sessions

## Objective

Allow Orbit to create and manage Claude Code sessions on remote machines via SSH, without requiring the user to install Orbit on that machine. The remote machine only needs `claude` CLI and either key-based or password-based SSH access.

## Expected behavior

1. In the New Session modal, the user toggles "ssh remote" mode.
2. The user enters the remote **host** (e.g. `vps.example.com`) and **user** (e.g. `ubuntu`).
3. The user optionally enters a **password**. If left empty, key-based auth is used.
4. The user clicks **test connection** to verify SSH access before creating the session.
5. The user enters the **remote path** and starts the session.
6. Orbit SSHes into `user@host` and runs `claude` on the remote machine, piping all output back.
7. The session feed, token counts, tool calls, and follow-up messages all work identically to a local session.

## Auth modes

| Mode | How it works |
|------|--------------|
| Key-based | `BatchMode=yes`, no password — uses SSH agent or `~/.ssh/id_*` |
| Password | `SSH_ASKPASS` + temp helper script echoes password; `SSH_ASKPASS_REQUIRE=force`; works without a TTY |

**Security notes:**
- Passwords are **never persisted** to the SQLite database or disk beyond session lifetime.
- The password is held in `ActiveSession` (in-memory) so follow-up messages can reconnect.
- An `AskpassGuard` (RAII) cleans up temp helper files when the `SpawnHandle` drops.
- The askpass helper script reads from a separate temp password file (restricted perms on Unix: 0o600/0o700), so the password is never embedded as a literal in an executable script.

## Test connection

A dedicated `test_ssh_connection(host, user, password?)` Tauri command:
1. Validates host/user format.
2. Runs `ssh ... "echo __orbit_ok__"` with a 5-second timeout.
3. Returns `{ ok: true, latencyMs: N }` or `{ ok: false, error: "..." }`.
4. The button in the modal shows a spinner → green (latency) or red (error message).

## Constraints

- Host and user are validated to contain only safe characters (alphanumeric, dots, hyphens, underscores).
- The first connection auto-accepts the host key (`StrictHostKeyChecking=accept-new`).
- Worktree creation is always skipped for SSH sessions.
- `--model`, `--resume`, `--prompt`, and `cwd` are all POSIX-escaped before being passed to the remote shell.

## Cases

| Scenario | Behavior |
|----------|----------|
| SSH mode, key auth, valid host/user/path | Session spawns on remote |
| SSH mode, password auth | SSH_ASKPASS helper echoes password; session spawns |
| Test connection, key auth success | Green: "connected · 42ms" |
| Test connection, wrong password | Red: "Permission denied (publickey,password)" |
| SSH mode, empty host | Submit blocked by frontend validation |
| SSH mode, invalid host chars | Rejected by backend, session enters error |
| SSH mode, `useWorktree: true` (API-level) | Overridden to `false` server-side |
| Follow-up message | `--resume` + password reused from ActiveSession memory |
| SSH connection drops mid-session | EOF detected; session stops |
| Remote `claude` not found | SSH exits non-zero; session enters error |

## Acceptance criteria

- [x] New Session modal shows host/user fields in SSH mode
- [x] Worktree checkbox hidden in SSH mode
- [ ] Optional password field in SSH mode with show/hide toggle
- [ ] "Test connection" button with spinner + result inline
- [x] Sessions with ssh_host/ssh_user spawn via SSH
- [x] Invalid host/user rejected before SSH call
- [x] posix_escape applied to all user-supplied values
- [ ] Password auth via SSH_ASKPASS works on Windows and Linux
- [ ] Temp askpass files cleaned up when session ends
- [ ] Follow-up messages reuse the in-memory password
