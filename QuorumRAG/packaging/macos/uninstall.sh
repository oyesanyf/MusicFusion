#!/usr/bin/env bash
# QuorumRAG macOS LaunchDaemon Uninstallation Script
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
    echo "[-] Elevating privileges via sudo..." >&2
    exec sudo "$0" "$@"
fi

PLIST="/Library/LaunchDaemons/dev.quorumrag.daemon.plist"
if [ -f "${PLIST}" ]; then
    launchctl unload -w "${PLIST}" 2>/dev/null || true
    rm -f "${PLIST}"
    echo "[+] Removed ${PLIST}"
fi

pkill -TERM -f "/usr/local/bin/quorumrag" 2>/dev/null || true

if [ -f /usr/local/bin/quorumrag ]; then
    rm -f /usr/local/bin/quorumrag
    echo "[+] Removed /usr/local/bin/quorumrag"
fi

if [ -d "/Applications/QuorumRAG.app" ]; then
    rm -rf "/Applications/QuorumRAG.app"
    echo "[+] Removed /Applications/QuorumRAG.app"
fi

echo "[+] QuorumRAG uninstalled cleanly."
