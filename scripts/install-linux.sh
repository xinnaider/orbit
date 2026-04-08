#!/usr/bin/env bash
# Orbit Linux installer
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

echo "==> Installing Orbit..."

# 1. Create directories
mkdir -p "$INSTALL_DIR" "$DESKTOP_DIR" "$ICON_DIR"

# 2. Resolve latest AppImage download URL
echo "==> Fetching latest release from GitHub..."
DOWNLOAD_URL=$(
  curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep -o '"browser_download_url": "[^"]*\.AppImage"' \
    | grep -v '\.sig"' \
    | grep -o 'https://[^"]*' \
    | head -1
)

if [ -z "$DOWNLOAD_URL" ]; then
  echo "ERROR: Could not find AppImage in the latest release."
  echo "Check https://github.com/$REPO/releases for available assets."
  exit 1
fi

# 3. Download AppImage
echo "==> Downloading $(basename "$DOWNLOAD_URL")..."
curl -fsSL --progress-bar "$DOWNLOAD_URL" -o "$APP_PATH"
chmod +x "$APP_PATH"

# 4. Extract icon from the AppImage
echo "==> Extracting icon..."
EXTRACT_DIR=$(mktemp -d)
(
  cd "$EXTRACT_DIR"
  "$APP_PATH" --appimage-extract usr/share/icons >/dev/null 2>&1 || true
)

# Try 256x256, fall back to 128x128
for SIZE in 256x256 128x128 32x32; do
  CANDIDATE="$EXTRACT_DIR/squashfs-root/usr/share/icons/hicolor/$SIZE/apps/orbit.png"
  if [ -f "$CANDIDATE" ]; then
    cp "$CANDIDATE" "$ICON_PATH"
    break
  fi
done

rm -rf "$EXTRACT_DIR"

# 5. Create .desktop entry (makes Orbit appear in KDE Kickoff, GNOME Activities, etc.)
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

# 6. Refresh desktop and icon caches
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo ""
echo "Orbit installed successfully."
echo ""
echo "  App:     $APP_PATH"
echo "  Desktop: $DESKTOP_FILE"
echo ""
echo "Open from your application menu or run:"
echo "  $APP_PATH"
echo ""
echo "Orbit will update itself automatically when a new version is available."
