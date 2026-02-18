#!/usr/bin/env bash
set -euo pipefail

# Seeds a running maxio server with sample buckets and objects for development.
# Usage: ./scripts/seed.sh [port]

PORT="${1:-9000}"
BASE="http://localhost:${PORT}"
ALIAS="maxio-seed-$$"
TMPDIR=$(mktemp -d)

cleanup() {
    mc alias rm "$ALIAS" 2>/dev/null || true
    rm -rf "$TMPDIR"
}
trap cleanup EXIT

echo "Seeding maxio at $BASE ..."

mc alias set "$ALIAS" "$BASE" minioadmin minioadmin --api s3v4 >/dev/null

# Create buckets
for bucket in uploads backups logs media archives; do
    if mc ls "$ALIAS/$bucket" >/dev/null 2>&1; then
        echo "  bucket '$bucket' already exists, skipping"
    else
        mc mb "$ALIAS/$bucket" >/dev/null
        echo "  created bucket '$bucket'"
    fi
done

# Seed sample objects
echo "hello world" > "$TMPDIR/readme.txt"
echo '{"version": 1, "status": "ok"}' > "$TMPDIR/config.json"
dd if=/dev/urandom bs=1024 count=64 of="$TMPDIR/data.bin" 2>/dev/null

mc cp "$TMPDIR/readme.txt"  "$ALIAS/uploads/readme.txt" >/dev/null
mc cp "$TMPDIR/config.json" "$ALIAS/uploads/config.json" >/dev/null
mc cp "$TMPDIR/readme.txt"  "$ALIAS/uploads/docs/getting-started.txt" >/dev/null
mc cp "$TMPDIR/data.bin"    "$ALIAS/backups/2024/jan/backup.bin" >/dev/null
mc cp "$TMPDIR/config.json" "$ALIAS/logs/app.json" >/dev/null

echo ""
echo "Done! Seeded 5 buckets with sample objects."
echo ""
mc ls "$ALIAS"
