#!/usr/bin/env bash
set -euo pipefail

# Integration tests using MinIO Client (mc) against a running maxio server.
# Usage: ./tests/mc_test.sh [port] [data_dir]
# Expects maxio to be running on localhost:${PORT:-9000}

PORT="${1:-9000}"
DATA_DIR="$(cd "${2:-./data}" && pwd)"
ALIAS="maxio-test-$$"
BUCKET="test-bucket-$$"
TMPDIR=$(mktemp -d)
PASS=0
FAIL=0

cleanup() {
    mc alias rm "$ALIAS" 2>/dev/null || true
    rm -rf "$TMPDIR"
}
trap cleanup EXIT

red()   { printf "\033[31m%s\033[0m\n" "$1"; }
green() { printf "\033[32m%s\033[0m\n" "$1"; }

assert() {
    local name="$1"
    shift
    if "$@" > /dev/null 2>&1; then
        green "PASS: $name"
        PASS=$((PASS + 1))
    else
        red "FAIL: $name"
        FAIL=$((FAIL + 1))
    fi
}

assert_fail() {
    local name="$1"
    shift
    if "$@" > /dev/null 2>&1; then
        red "FAIL: $name (expected failure but succeeded)"
        FAIL=$((FAIL + 1))
    else
        green "PASS: $name"
        PASS=$((PASS + 1))
    fi
}

assert_eq() {
    local name="$1" expected="$2" actual="$3"
    if [ "$expected" = "$actual" ]; then
        green "PASS: $name"
        PASS=$((PASS + 1))
    else
        red "FAIL: $name (expected '$expected', got '$actual')"
        FAIL=$((FAIL + 1))
    fi
}

assert_file_exists() {
    local name="$1" path="$2"
    if [ -e "$path" ]; then
        green "PASS: $name"
        PASS=$((PASS + 1))
    else
        red "FAIL: $name (file not found: $path)"
        FAIL=$((FAIL + 1))
    fi
}

assert_file_not_exists() {
    local name="$1" path="$2"
    if [ ! -e "$path" ]; then
        green "PASS: $name"
        PASS=$((PASS + 1))
    else
        red "FAIL: $name (file should not exist: $path)"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== Maxio mc integration tests ==="
echo "Server: localhost:$PORT"
echo "Data dir: $DATA_DIR"
echo ""

# --- Setup ---
if mc alias set "$ALIAS" "http://localhost:$PORT" minioadmin minioadmin 2>&1 | grep -qi "error"; then
    red "FAIL: alias set"
    FAIL=$((FAIL + 1))
else
    green "PASS: alias set"
    PASS=$((PASS + 1))
fi

# --- Bucket operations ---
assert "create bucket" mc mb "$ALIAS/$BUCKET"
assert_file_exists "bucket dir exists on disk" "$DATA_DIR/buckets/$BUCKET"
assert_file_exists "bucket meta exists on disk" "$DATA_DIR/buckets/$BUCKET/.bucket.json"

# List buckets and check ours is there
OUTPUT=$(mc ls "$ALIAS/" 2>&1)
assert_eq "list buckets contains our bucket" "true" "$(echo "$OUTPUT" | grep -q "$BUCKET" && echo true || echo false)"

# --- Object operations ---
echo "hello maxio" > "$TMPDIR/test.txt"

assert "upload object" mc cp "$TMPDIR/test.txt" "$ALIAS/$BUCKET/test.txt"
assert_file_exists "object file exists on disk" "$DATA_DIR/buckets/$BUCKET/test.txt"
assert_file_exists "object meta exists on disk" "$DATA_DIR/buckets/$BUCKET/test.txt.meta.json"

# Verify the on-disk content matches
assert_eq "on-disk content matches" "hello maxio" "$(cat "$DATA_DIR/buckets/$BUCKET/test.txt")"

# List objects
OUTPUT=$(mc ls "$ALIAS/$BUCKET/" 2>&1)
assert_eq "list objects contains test.txt" "true" "$(echo "$OUTPUT" | grep -q "test.txt" && echo true || echo false)"

# Download and verify content
assert "download object" mc cp "$ALIAS/$BUCKET/test.txt" "$TMPDIR/downloaded.txt"
assert_eq "content matches" "hello maxio" "$(cat "$TMPDIR/downloaded.txt")"

# Cat object
OUTPUT=$(mc cat "$ALIAS/$BUCKET/test.txt" 2>&1)
assert_eq "cat object" "hello maxio" "$OUTPUT"

# --- Nested keys ---
assert "upload nested object" mc cp "$TMPDIR/test.txt" "$ALIAS/$BUCKET/folder/nested/file.txt"
assert_file_exists "nested object exists on disk" "$DATA_DIR/buckets/$BUCKET/folder/nested/file.txt"
assert_file_exists "nested meta exists on disk" "$DATA_DIR/buckets/$BUCKET/folder/nested/file.txt.meta.json"

OUTPUT=$(mc ls "$ALIAS/$BUCKET/folder/" 2>&1)
assert_eq "list nested prefix" "true" "$(echo "$OUTPUT" | grep -q "nested" && echo true || echo false)"

assert "download nested object" mc cp "$ALIAS/$BUCKET/folder/nested/file.txt" "$TMPDIR/nested.txt"
assert_eq "nested content matches" "hello maxio" "$(cat "$TMPDIR/nested.txt")"

# --- Multipart upload (large file) ---
dd if=/dev/urandom of="$TMPDIR/big.bin" bs=1M count=15 status=none
assert "upload large object (multipart)" mc cp "$TMPDIR/big.bin" "$ALIAS/$BUCKET/big.bin"
assert "download large object" mc cp "$ALIAS/$BUCKET/big.bin" "$TMPDIR/big.download.bin"
assert_eq "large object size matches" "$(wc -c < "$TMPDIR/big.bin" | tr -d ' ')" "$(wc -c < "$TMPDIR/big.download.bin" | tr -d ' ')"
assert_eq "large object sha256 matches" "$(shasum -a 256 "$TMPDIR/big.bin" | awk '{print $1}')" "$(shasum -a 256 "$TMPDIR/big.download.bin" | awk '{print $1}')"
OUTPUT=$(mc stat "$ALIAS/$BUCKET/big.bin" 2>&1)
assert_eq "multipart etag suffix present" "true" "$(echo "$OUTPUT" | grep -Eq 'ETag.*-[0-9]+' && echo true || echo false)"

# --- Overwrite object ---
echo "updated content" > "$TMPDIR/updated.txt"
assert "overwrite object" mc cp "$TMPDIR/updated.txt" "$ALIAS/$BUCKET/test.txt"

OUTPUT=$(mc cat "$ALIAS/$BUCKET/test.txt" 2>&1)
assert_eq "overwritten content" "updated content" "$OUTPUT"
assert_eq "on-disk overwritten content" "updated content" "$(cat "$DATA_DIR/buckets/$BUCKET/test.txt")"

# --- Delete operations ---
assert "delete object" mc rm "$ALIAS/$BUCKET/test.txt"
assert_file_not_exists "deleted object gone from disk" "$DATA_DIR/buckets/$BUCKET/test.txt"
assert_file_not_exists "deleted meta gone from disk" "$DATA_DIR/buckets/$BUCKET/test.txt.meta.json"
assert_fail "get deleted object" mc cat "$ALIAS/$BUCKET/test.txt"

assert "delete nested object" mc rm "$ALIAS/$BUCKET/folder/nested/file.txt"
assert_file_not_exists "deleted nested object gone from disk" "$DATA_DIR/buckets/$BUCKET/folder/nested/file.txt"
assert "delete large object" mc rm "$ALIAS/$BUCKET/big.bin"

# Delete bucket (should work now that it's empty)
assert "delete empty bucket" mc rb "$ALIAS/$BUCKET"
assert_file_not_exists "bucket dir gone from disk" "$DATA_DIR/buckets/$BUCKET"
assert_fail "head deleted bucket" mc ls "$ALIAS/$BUCKET/"

# --- Summary ---
echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
