#!/usr/bin/env bash
# Orbit — macOS Installer
# Downloads the latest .dmg, mounts it, copies Orbit to /Applications,
# and cleans up. Works on Intel and Apple Silicon.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-macos.sh | bash
set -euo pipefail

REPO="xinnaider/orbit"

# ── Colors ────────────────────────────────────────────────────────────────────
if [ -t 1 ]; then
  G='\033[0;32m'
  BG='\033[1;32m'
  W='\033[1;37m'
  D='\033[0;90m'
  R='\033[0;31m'
  N='\033[0m'
else
  G=''; BG=''; W=''; D=''; R=''; N=''
fi

SYM_STEP=$'\u25C6'   # ◆
SYM_OK=$'\u2713'     # ✓
SYM_ERR=$'\u2717'    # ✗
SYM_SEP=$'\u2500'    # ─
SYM_BLOCK=$'\u2588'  # █
SYM_LIGHT=$'\u2591'  # ░

step()    { printf "  ${G}%s${N} %s\n"       "$SYM_STEP" "$1"; }
info()    { printf "    ${D}%s${N}\n"         "$1"; }
success() { printf "  ${BG}%s${N} %s\n"      "$SYM_OK"   "$1"; }
fail()    { printf "  ${R}%s ERROR:${N} %s\n" "$SYM_ERR"  "$1" >&2; exit 1; }
sep()     { printf "  ${D}%s${N}\n" "$(printf '%0.s'"$SYM_SEP" $(seq 1 35))"; }

make_bar() {
    local filled=$1 empty=$2 bar='' i
    for ((i = 0; i < filled; i++)); do bar+="$SYM_BLOCK"; done
    for ((i = 0; i < empty;  i++)); do bar+="$SYM_LIGHT"; done
    printf '%s' "$bar"
}

download_with_progress() {
    local url="$1" dest="$2"

    local total_bytes
    total_bytes=$(curl -fsSLI "$url" \
        | grep -i '^content-length' | tail -1 \
        | awk '{print $2}' | tr -d '\r')
    [ -z "$total_bytes" ] && total_bytes=0

    curl -fsSL "$url" -o "$dest" &
    local pid=$!

    while kill -0 "$pid" 2>/dev/null; do
        if [ -f "$dest" ] && [ "$total_bytes" -gt 0 ]; then
            local cur
            cur=$(stat -f%z "$dest" 2>/dev/null || echo 0)
            local pct=$(( cur * 100 / total_bytes ))
            [ "$pct" -gt 100 ] && pct=100
            local filled=$(( pct / 5 ))
            [ "$filled" -gt 20 ] && filled=20
            local empty=$(( 20 - filled ))
            local dl_mb total_mb
            dl_mb=$(( cur / 1048576 ))
            total_mb=$(( total_bytes / 1048576 ))
            printf "\r    ${G}[%s] %d%%  %d MB / %d MB  ${N}" \
                "$(make_bar "$filled" "$empty")" "$pct" "$dl_mb" "$total_mb"
        fi
        sleep 0.2
    done
    wait "$pid"

    local final_mb
    final_mb=$(( $(stat -f%z "$dest") / 1048576 ))
    printf "\r    ${G}[%s] 100%%  %d MB / %d MB  ${N}\n" \
        "$(make_bar 20 0)" "$final_mb" "$final_mb"
}

# ── Header ────────────────────────────────────────────────────────────────────
clear
printf '\n'
printf "${G}  ██████╗ ██████╗ ██████╗ ██╗████████╗${N}\n"
printf "${G} ██╔═══██╗██╔══██╗██╔══██╗██║╚══██╔══╝${N}\n"
printf "${G} ██║   ██║██████╔╝██████╔╝██║   ██║   ${N}\n"
printf "${G} ██║   ██║██╔══██╗██╔══██╗██║   ██║   ${N}\n"
printf "${G} ╚██████╔╝██║  ██╗██████╔╝██║   ██║   ${N}\n"
printf "${G}  ╚═════╝ ╚═╝  ╚═╝╚═════╝ ╚═╝   ╚═╝  ${N}\n"
printf '\n'
printf "${W}  AI Agent Dashboard${N}\n"
printf '\n'
sep
printf '\n'

# ── Detect architecture ───────────────────────────────────────────────────────
ARCH=$(uname -m)
case "$ARCH" in
  arm64|aarch64) PATTERN='aarch64.*\.dmg"' ;;
  x86_64)        PATTERN='x64.*\.dmg"'     ;;
  *)             fail "Unsupported architecture: $ARCH" ;;
esac

# ── Fetch release ─────────────────────────────────────────────────────────────
step 'Fetching latest release...'
API_JSON=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")
VERSION=$(printf '%s' "$API_JSON" \
    | grep '"tag_name"' | head -1 \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
DOWNLOAD_URL=$(printf '%s' "$API_JSON" \
    | grep '"browser_download_url"' \
    | grep "$PATTERN" | grep -v '\.sig"' \
    | head -1 \
    | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')

[ -z "$DOWNLOAD_URL" ] && fail "No .dmg found for $ARCH. Visit github.com/$REPO/releases"

info "-> Orbit $VERSION ($ARCH)"
printf '\n'

# ── Download ──────────────────────────────────────────────────────────────────
DMG_PATH=$(mktemp /tmp/orbit-XXXXXX.dmg)
FILENAME=$(basename "$DOWNLOAD_URL")
step "Downloading $FILENAME"
download_with_progress "$DOWNLOAD_URL" "$DMG_PATH"
printf '\n'

# ── Mount and copy ────────────────────────────────────────────────────────────
step 'Installing to /Applications...'
MOUNT_DIR=$(mktemp -d /tmp/orbit-mount-XXXXXX)
hdiutil attach -nobrowse -mountpoint "$MOUNT_DIR" "$DMG_PATH" -quiet

APP_SRC="$MOUNT_DIR/Orbit.app"
APP_DEST="/Applications/Orbit.app"

if [ ! -d "$APP_SRC" ]; then
    # Some DMGs put the .app at a different level
    APP_SRC=$(find "$MOUNT_DIR" -maxdepth 2 -name "*.app" -type d | head -1)
fi

[ -z "$APP_SRC" ] && { hdiutil detach "$MOUNT_DIR" -quiet 2>/dev/null; fail "No .app found in DMG"; }

# Remove existing installation if present
[ -d "$APP_DEST" ] && rm -rf "$APP_DEST"

cp -R "$APP_SRC" "$APP_DEST"

# ── Cleanup ───────────────────────────────────────────────────────────────────
hdiutil detach "$MOUNT_DIR" -quiet 2>/dev/null || true
rm -f "$DMG_PATH"
rmdir "$MOUNT_DIR" 2>/dev/null || true

# ── Remove quarantine attribute ───────────────────────────────────────────────
xattr -rd com.apple.quarantine "$APP_DEST" 2>/dev/null || true

# ── Done ─────────────────────────────────────────────────────────────────────
printf '\n'
sep
printf '\n'
success 'Orbit installed successfully.'
printf '\n'
info "App     -> $APP_DEST"
info 'Launch  -> open from Spotlight or Applications folder'
info 'Updates -> automatic via built-in updater'
info 'Docs    -> github.com/xinnaider/orbit'
printf '\n'
sep
printf '\n'
