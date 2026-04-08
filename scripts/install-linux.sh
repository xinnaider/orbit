#!/usr/bin/env bash
# Orbit — Linux Installer
# Downloads the latest AppImage, installs to ~/.local/share/orbit/,
# creates a desktop entry and icon so Orbit appears in the app launcher.
# The installed AppImage self-updates via the built-in updater.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-linux.sh | bash
set -euo pipefail

REPO="xinnaider/orbit"
INSTALL_DIR="$HOME/.local/share/orbit"
APP_PATH="$INSTALL_DIR/orbit.AppImage"
DESKTOP_DIR="$HOME/.local/share/applications"
DESKTOP_FILE="$DESKTOP_DIR/orbit.desktop"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
ICON_PATH="$ICON_DIR/orbit.png"

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

step()    { printf "  ${G}◆${N} %s\n" "$1"; }
info()    { printf "    ${D}%s${N}\n" "$1"; }
success() { printf "  ${BG}✓${N} %s\n" "$1"; }
fail()    { printf "  ${R}✗ ERROR:${N} %s\n" "$1" >&2; exit 1; }
sep()     { printf "  ${D}───────────────────────────────────${N}\n"; }

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
            cur=$(stat -c%s "$dest" 2>/dev/null || echo 0)
            local pct=$(( cur * 100 / total_bytes ))
            [ "$pct" -gt 100 ] && pct=100
            local filled=$(( pct / 5 ))
            [ "$filled" -gt 20 ] && filled=20
            local empty=$(( 20 - filled ))
            local bar
            bar=$(printf '%*s' "$filled" '' | tr ' ' '█')$(printf '%*s' "$empty" '' | tr ' ' '░')
            local dl_mb total_mb
            dl_mb=$(( cur / 1048576 ))
            total_mb=$(( total_bytes / 1048576 ))
            printf "\r    ${G}[%s] %d%%  %d MB / %d MB  ${N}" \
                "$bar" "$pct" "$dl_mb" "$total_mb"
        fi
        sleep 0.2
    done
    wait "$pid"

    local final_mb
    final_mb=$(( $(stat -c%s "$dest") / 1048576 ))
    printf "\r    ${G}[%s] 100%%  %d MB / %d MB  ${N}\n" \
        "$(printf '%*s' 20 '' | tr ' ' '█')" "$final_mb" "$final_mb"
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
printf "${W}  Claude Code Agent Dashboard${N}\n"
printf '\n'
sep
printf '\n'

# ── Fetch release ─────────────────────────────────────────────────────────────
step 'Fetching latest release...'
API_JSON=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")
VERSION=$(printf '%s' "$API_JSON" \
    | grep '"tag_name"' | head -1 \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
DOWNLOAD_URL=$(printf '%s' "$API_JSON" \
    | grep '"browser_download_url"' \
    | grep '\.AppImage"' | grep -v '\.sig"' \
    | head -1 \
    | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')

[ -z "$DOWNLOAD_URL" ] && fail "No AppImage found. Visit github.com/$REPO/releases"

info "-> Orbit $VERSION found"
printf '\n'

# ── Prepare dirs ──────────────────────────────────────────────────────────────
mkdir -p "$INSTALL_DIR" "$DESKTOP_DIR" "$ICON_DIR"

# ── Download ──────────────────────────────────────────────────────────────────
FILENAME=$(basename "$DOWNLOAD_URL")
step "Downloading $FILENAME"
download_with_progress "$DOWNLOAD_URL" "$APP_PATH"
chmod +x "$APP_PATH"
printf '\n'

# ── Desktop integration ───────────────────────────────────────────────────────
step 'Setting up desktop integration...'

EXTRACT_DIR=$(mktemp -d)
(
    cd "$EXTRACT_DIR"
    "$APP_PATH" --appimage-extract usr/share/icons >/dev/null 2>&1 || true
)
for SIZE in 256x256 128x128 32x32; do
    CANDIDATE="$EXTRACT_DIR/squashfs-root/usr/share/icons/hicolor/$SIZE/apps/orbit.png"
    if [ -f "$CANDIDATE" ]; then
        cp "$CANDIDATE" "$ICON_PATH"
        break
    fi
done
rm -rf "$EXTRACT_DIR"

cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Name=Orbit
Comment=Claude Code agent dashboard
Exec=$APP_PATH
Icon=orbit
Type=Application
Categories=Development;
StartupWMClass=orbit
EOF

update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

# ── Done ─────────────────────────────────────────────────────────────────────
printf '\n'
sep
printf '\n'
success 'Orbit installed successfully.'
printf '\n'
info "App     -> $APP_PATH"
info 'Launcher-> already added to your app menu'
info 'Updates -> automatic via built-in updater'
info 'Docs    -> github.com/xinnaider/orbit'
printf '\n'
sep
printf '\n'
