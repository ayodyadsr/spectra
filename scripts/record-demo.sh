#!/usr/bin/env bash
# Record an asciinema cast of the Spectra synthetic-regression demo.
# Output: demo.cast (uploadable via `asciinema upload demo.cast`).
set -euo pipefail

if ! command -v asciinema >/dev/null 2>&1; then
  echo "asciinema not found. Install with:"
  echo "  Debian/Ubuntu: sudo apt install asciinema"
  echo "  pipx:          pipx install asciinema"
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust first: https://rustup.rs"
  exit 1
fi

cargo build --release

cat <<'EOF'
About to record. The demo runs:
  spectra check --old examples/lending_v1.json --new examples/lending_v2.json --format markdown

When recording starts, just wait for the command to finish, then press Ctrl+D.
EOF

asciinema rec demo.cast \
  --title "Spectra M0 PoC — Solana program upgrade diff" \
  --command "./target/release/spectra check --old examples/lending_v1.json --new examples/lending_v2.json --format markdown"

echo
echo "Recorded to demo.cast"
echo "Upload to asciinema.org: asciinema upload demo.cast"
echo "Or play locally:         asciinema play demo.cast"
