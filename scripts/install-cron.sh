#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
WRAPPER="$SCRIPT_DIR/run-daily.sh"
LOG_FILE="$PROJECT_DIR/connections.log"

OS="$(uname -s)"

install_launchd() {
  PLIST_NAME="com.jhash.connections"
  PLIST_PATH="$HOME/Library/LaunchAgents/$PLIST_NAME.plist"

  cat > "$PLIST_PATH" << XML
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$PLIST_NAME</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/bash</string>
    <string>$WRAPPER</string>
  </array>
  <key>StartInterval</key>
  <integer>7200</integer>
  <key>StandardOutPath</key>
  <string>$LOG_FILE</string>
  <key>StandardErrorPath</key>
  <string>$LOG_FILE</string>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
XML

  launchctl unload "$PLIST_PATH" 2>/dev/null || true
  launchctl load -w "$PLIST_PATH"
  echo "Installed launchd agent: $PLIST_NAME (fires every 2h, logs → $LOG_FILE)"
}

install_systemd() {
  SERVICE_DIR="$HOME/.config/systemd/user"
  mkdir -p "$SERVICE_DIR"

  cat > "$SERVICE_DIR/connections-archive.service" << UNIT
[Unit]
Description=NYT Connections daily archive update

[Service]
Type=oneshot
ExecStart=/bin/bash $WRAPPER
StandardOutput=append:$LOG_FILE
StandardError=append:$LOG_FILE
UNIT

  cat > "$SERVICE_DIR/connections-archive.timer" << UNIT
[Unit]
Description=Run connections archive every 2 hours

[Timer]
OnBootSec=5min
OnUnitActiveSec=2h
Persistent=true

[Install]
WantedBy=timers.target
UNIT

  systemctl --user daemon-reload
  systemctl --user enable --now connections-archive.timer
  echo "Installed systemd user timer (fires every 2h, logs → $LOG_FILE)"
  echo "Status: systemctl --user status connections-archive.timer"
}

case "$OS" in
  Darwin) install_launchd ;;
  Linux)  install_systemd ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac
