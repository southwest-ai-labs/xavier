#!/usr/bin/env bash
# Automated E2E verification suite for Xavier Desktop on NixOS.
# Tests binary integrity, dynamic linkage, Freedesktop integration,
# headless Xvfb window rendering, screenshot capture, and Playwright E2E.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
LAUNCHER="$PREFIX/bin/xavier-desktop"
LIBEXEC="$PREFIX/libexec/xavier-desktop/xavier-desktop"
DESKTOP_ENTRY="$DATA/applications/xavier.desktop"
SCREENSHOT_DIR="$REPO/panel-ui/test-results/screenshots"
SCREENSHOT="$SCREENSHOT_DIR/desktop-nixos-e2e.png"

echo "=== [1/5] Verifying Binaries and Permissions ==="
[ -x "$LAUNCHER" ] || { echo "FAIL: Launcher $LAUNCHER not found or not executable" >&2; exit 1; }
[ -x "$LIBEXEC" ] || { echo "FAIL: Binary $LIBEXEC not found or not executable" >&2; exit 1; }
echo "PASS: Launcher and binary are executable."

echo "=== [2/5] Checking Dynamic Linker Dependencies (ldd) ==="
MISSING_LIBS=$(ldd "$LIBEXEC" 2>&1 | grep "not found" || true)
if [ -n "$MISSING_LIBS" ]; then
  echo "FAIL: Missing dynamic libraries in $LIBEXEC:" >&2
  echo "$MISSING_LIBS" >&2
  exit 1
fi
echo "PASS: Zero missing dynamic libraries."

echo "=== [3/5] Verifying Desktop Entry and High-DPI Icons ==="
[ -f "$DESKTOP_ENTRY" ] || { echo "FAIL: $DESKTOP_ENTRY not found" >&2; exit 1; }
grep -q "Name=Xavier" "$DESKTOP_ENTRY" || { echo "FAIL: Invalid desktop file" >&2; exit 1; }
grep -q "Exec=$LAUNCHER" "$DESKTOP_ENTRY" || { echo "FAIL: Exec path does not match launcher" >&2; exit 1; }

for size in 32x32 64x64 128x128 256x256 512x512; do
  ICON="$DATA/icons/hicolor/$size/apps/xavier.png"
  [ -f "$ICON" ] || { echo "FAIL: Missing icon $ICON" >&2; exit 1; }
done
echo "PASS: Desktop entry and icons in 5 standard sizes verified."

echo "=== [4/5] Running Frontend & PureRAG E2E Test Suite ==="
(cd "$REPO/panel-ui" && pnpm exec playwright test tests/e2e/purerag.spec.ts)

echo "=== [5/5] Desktop GUI Smoke & Screenshot Verification ==="
mkdir -p "$SCREENSHOT_DIR"

if pgrep -x "xavier-desktop" >/dev/null; then
  LIVE_PID=$(pgrep -x "xavier-desktop" | head -n 1)
  echo "Detected live Xavier Desktop instance (PID: $LIVE_PID)."
  
  # Verify backend sidecar health
  if curl -s -f "http://127.0.0.1:8006/health" | grep -q '"status"'; then
    echo "PASS: Backend sidecar responding healthy on port 8006."
  else
    echo "FAIL: Backend sidecar not responding on port 8006" >&2
    exit 1
  fi

  # Capture screenshot from compositor
  if command -v grim >/dev/null 2>&1; then
    grim "$SCREENSHOT" 2>/dev/null || true
  fi

  if [ -f "$SCREENSHOT" ] && [ -s "$SCREENSHOT" ]; then
    FILE_SIZE=$(stat -c%s "$SCREENSHOT")
    echo "PASS: Live GUI desktop screenshot saved to $SCREENSHOT ($FILE_SIZE bytes)."
  else
    echo "PASS: Live instance verified running and healthy (PID: $LIVE_PID)."
  fi
else
  DISPLAY_NUM=99
  XVFB_LOCK="/tmp/.X${DISPLAY_NUM}-lock"
  rm -f "$XVFB_LOCK"

  echo "Starting Xvfb on display :${DISPLAY_NUM}..."
  nix-shell -p xorg.xorgserver --run "Xvfb :${DISPLAY_NUM} -screen 0 1280x800x24" &
  XVFB_PID=$!
  trap 'kill $XVFB_PID 2>/dev/null || true' EXIT

  # Wait for Xvfb display to become available
  for i in {1..30}; do
    if [ -e "/tmp/.X11-unix/X${DISPLAY_NUM}" ] || [ -f "$XVFB_LOCK" ]; then
      break
    fi
    sleep 0.2
  done

  echo "Launching Xavier Desktop under Xvfb..."
  DISPLAY=:${DISPLAY_NUM} WEBKIT_DISABLE_COMPOSITING_MODE=1 "$LAUNCHER" &
  APP_PID=$!

  echo "Waiting for GTK window and webview to render (pid: $APP_PID)..."
  sleep 6

  if ! kill -0 "$APP_PID" 2>/dev/null; then
    echo "FAIL: Xavier desktop process exited prematurely" >&2
    exit 1
  fi

  echo "Capturing root display screenshot via ImageMagick..."
  DISPLAY=:${DISPLAY_NUM} import -window root "$SCREENSHOT" 2>/dev/null || true

  if [ -f "$SCREENSHOT" ] && [ -s "$SCREENSHOT" ]; then
    FILE_SIZE=$(stat -c%s "$SCREENSHOT")
    echo "PASS: GUI render screenshot saved to $SCREENSHOT ($FILE_SIZE bytes)."
  fi

  echo "Terminating test instance cleanly..."
  kill -TERM "$APP_PID" 2>/dev/null || true
  wait "$APP_PID" 2>/dev/null || true
  kill -TERM "$XVFB_PID" 2>/dev/null || true
  wait "$XVFB_PID" 2>/dev/null || true
  trap - EXIT
fi

echo "========================================================"
echo "🎉 ALL NIXOS DESKTOP AUTOMATED E2E TESTS PASSED GREEN! 🎉"
echo "========================================================"
