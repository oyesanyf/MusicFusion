#!/usr/bin/env bash
# QuorumRAG macOS LaunchDaemon Installation Script (Zero-Touch PATH)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "$(id -u)" -ne 0 ]; then
    echo "[-] Elevating privileges via sudo..." >&2
    exec sudo "$0" "$@"
fi

mkdir -p /usr/local/bin \
         "/Library/Application Support/QuorumRAG/data" \
         "/Library/Application Support/QuorumRAG/docs" \
         "/Library/Logs/QuorumRAG" \
         /Library/LaunchDaemons

if [ -f "${SCRIPT_DIR}/quorumrag" ]; then
    install -m 755 "${SCRIPT_DIR}/quorumrag" /usr/local/bin/quorumrag
    echo "[+] Installed /usr/local/bin/quorumrag"
fi

if [ -f "${SCRIPT_DIR}/dev.quorumrag.daemon.plist" ]; then
    TARGET_PLIST="/Library/LaunchDaemons/dev.quorumrag.daemon.plist"
    launchctl unload -w "${TARGET_PLIST}" 2>/dev/null || true
    install -m 644 "${SCRIPT_DIR}/dev.quorumrag.daemon.plist" "${TARGET_PLIST}"
    echo "[+] Installed ${TARGET_PLIST} (on-demand; start with: sudo launchctl load -w ${TARGET_PLIST})"
fi

echo "[+] QuorumRAG installed on macOS (Zero-Touch PATH preserved)."
