# SSH Remote Sessions

## Objective

Allow Orbit to create and manage Claude Code sessions on remote machines via SSH, without requiring the user to install Orbit on that machine. The remote machine only needs `claude` CLI and SSH key-based access.

## Expected behavior

1. In the New Session modal, the user toggles "ssh remote" mode.
2. The user enters the remote **host** (e.g. `vps.example.com`) and **user** (e.g. `ubuntu`).
3. The user enters the **remote path** where Claude should run (e.g. `/home/ubuntu/project`).
4. Orbit SSHes into `user@host` and runs `claude` on the remote machine, piping all output back through the SSH connection.
5. The session feed, token counts, tool calls, and follow-up messages all work identically to a local session.

## Constraints

- Host and user are validated to contain only safe characters (alphanumeric, dots, hyphens, underscores). Invalid values are rejected before any SSH call.
- SSH uses key-based authentication only (`BatchMode=yes`). Password prompts are not supported.
- The first connection auto-accepts the host key (`StrictHostKeyChecking=accept-new`). Subsequent connections verify the persisted key.
- Worktree creation is skipped for SSH sessions (the remote path is used directly).
- The `--model` and `--resume` arguments are POSIX-escaped before being passed to the remote shell.

## Cases

| Scenario | Behavior |
|----------|----------|
| SSH mode, valid host+user+path | Session spawns on remote, output streams back |
| SSH mode, empty host | Submit blocked by frontend validation |
| SSH mode, invalid host chars (e.g. `-o...`) | Rejected by backend before SSH call, session enters error state |
| SSH mode, `useWorktree: true` (API-level) | Silently overridden to `false` on server side |
| SSH mode + follow-up message | `--resume` with posix-escaped session ID is sent |
| SSH connection drops mid-session | Session stops; stdout EOF detected |
| Remote `claude` not found | SSH exits non-zero; session enters error state |

## Acceptance criteria

- [ ] New Session modal shows host/user fields when SSH mode is selected
- [ ] Worktree checkbox is hidden in SSH mode
- [ ] Sessions with `ssh_host`/`ssh_user` set in the DB spawn via SSH
- [ ] Invalid host/user values are rejected before spawning
- [ ] `posix_escape` is applied to all user-supplied values in the remote command
- [ ] Follow-up messages resume the correct session on the remote machine
