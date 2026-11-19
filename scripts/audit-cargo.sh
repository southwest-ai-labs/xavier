#!/usr/bin/env bash
# =============================================================================
# scripts/audit-cargo.sh — Run cargo audit with verified advisory configurations
#
# Audits Cargo.lock dependencies against the RustSec advisory database.
# Uses .cargo/audit_ignore.txt and .cargo/audit-warnings.toml for known
# unpatched transitive advisories.
#
# Part of SEC-02 repository security audit sanitization.
#
# Usage:
#   scripts/audit-cargo.sh [--no-fetch]
# =============================================================================
set -uo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_DIR"

echo "==> Running Cargo security audit on Cargo.lock..."

if ! command -v cargo-audit >/dev/null 2>&1 && ! cargo audit --version >/dev/null 2>&1; then
    echo "❌ cargo-audit is not installed. Install via: cargo install cargo-audit" >&2
    exit 2
fi

# Ensure .cargo/audit.toml points to .cargo/audit-warnings.toml for cargo-audit native loading
if [ ! -e .cargo/audit.toml ] && [ -f .cargo/audit-warnings.toml ]; then
    ln -sf audit-warnings.toml .cargo/audit.toml
fi

# Collect active advisories from .cargo/audit_ignore.txt
IGNORES=()
if [ -f .cargo/audit_ignore.txt ]; then
    while IFS= read -r line || [ -n "$line" ]; do
        clean_id="$(echo "$line" | sed 's/#.*//' | tr -d '[:space:]')"
        if [ -n "$clean_id" ]; then
            IGNORES+=(--ignore "$clean_id")
        fi
    done < .cargo/audit_ignore.txt
fi

exec cargo audit "${IGNORES[@]}" "$@"
