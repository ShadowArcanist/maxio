# Multipart Upload Implementation Plan

## Context
MaxIO currently supports single-request PutObject only. S3 clients (AWS CLI, mc) automatically use multipart upload for files >8MB, so this is required for real-world usage. This adds the 6 S3 multipart operations.

## On-Disk Layout
```
{data_dir}/buckets/{bucket}/.uploads/{uploadId}/
    .meta.json       # MultipartUploadMeta (key, content_type, initiated)
    1                # part 1 bytes
    1.meta.json      # PartMeta (part_number, etag, size)
    ...
```

## Step 0 — Error + Storage Types (foundation)

**`src/error.rs`** — Add 3 variants + constructors:
- `NoSuchUpload` (404), `InvalidPart` (400), `EntityTooSmall` (400)

**`src/storage/mod.rs`** — Add types:
- `MultipartUploadMeta { upload_id, bucket, key, content_type, initiated }`
- `PartMeta { part_number, etag, size, last_modified }`
- `StorageError::UploadNotFound(String)`

## Step 1 — Storage Methods (parallel with Step 2)

**`src/storage/filesystem.rs`** — 6 new methods:
- `create_multipart_upload(bucket, key, content_type) -> MultipartUploadMeta`
- `upload_part(bucket, upload_id, part_number, body) -> PartMeta`
- `complete_multipart_upload(bucket, upload_id, parts) -> PutResult`
  - Validates part ETags match, enforces 5MB min (except last part)
  - Concatenates parts to final object path, writes `.meta.json`
  - Composite ETag: `MD5(concat(raw_md5_bytes))-{N}`
  - Cleans up `.uploads/{uploadId}/`
- `abort_multipart_upload(bucket, upload_id)` — removes upload dir
- `list_parts(bucket, upload_id) -> (MultipartUploadMeta, Vec<PartMeta>)`
- `list_multipart_uploads(bucket) -> Vec<MultipartUploadMeta>`

**Also modify:**
- `has_objects()` — skip `.uploads` directory
- `walk_dir()` — skip `.uploads` directory

## Step 2 — XML Types (parallel with Step 1)

**`src/xml/types.rs`** — Add:
- `InitiateMultipartUploadResult` (Bucket, Key, UploadId)
- `CompleteMultipartUploadResult` (Location, Bucket, Key, ETag)
- `ListPartsResult` (Bucket, Key, UploadId, IsTruncated, Vec<PartEntry>)
- `ListMultipartUploadsResult` (Bucket, IsTruncated, Vec<MultipartUploadEntry>)

## Step 3 — API Handlers

**New file: `src/api/multipart.rs`** — 6 handlers:

| Handler | Trigger | Returns |
|---|---|---|
| `create_multipart_upload` | POST `?uploads` | 200 + InitiateMultipartUploadResult XML |
| `upload_part` | PUT `?partNumber&uploadId` | 200 + ETag header |
| `complete_multipart_upload` | POST `?uploadId` | 200 + CompleteMultipartUploadResult XML |
| `abort_multipart_upload` | DELETE `?uploadId` | 204 |
| `list_parts` | GET `?uploadId` | 200 + ListPartsResult XML |
| `list_multipart_uploads` | GET `/{bucket}?uploads` | 200 + ListMultipartUploadsResult XML |

**Parse `CompleteMultipartUpload` XML body** with `quick_xml::Reader` (same pattern as `delete_objects` in `object.rs`).

**Extract chunked-decode logic** from `put_object` into shared helper for reuse by `upload_part`.

## Step 4 — Router + Query-Param Dispatch

**`src/api/mod.rs`** — Add `pub mod multipart;`

**`src/api/router.rs`** — Add:
```rust
.route("/{bucket}/{*key}", post(object::post_object))
```

**`src/api/object.rs`** — Add query-param guards at top of existing handlers:
- `put_object`: if `?uploadId` present → delegate to `multipart::upload_part`
- `delete_object`: if `?uploadId` present → delegate to `multipart::abort_multipart_upload`
- `get_object`: if `?uploadId` present → delegate to `multipart::list_parts`
- New `post_object`: `?uploads` → create, `?uploadId` → complete

**`src/api/list.rs`** — In `handle_bucket_get`, add check before `?location`:
- if `?uploads` → delegate to `multipart::list_multipart_uploads`

## Step 5 — Tests

**`tests/integration.rs`** — Rust integration tests:
- `test_multipart_create_upload` — initiate returns UploadId
- `test_multipart_upload_part` — part returns ETag
- `test_multipart_complete` — full happy path, GET returns concatenated data
- `test_multipart_complete_part_too_small` — 400 EntityTooSmall
- `test_multipart_abort` — cleanup verified
- `test_multipart_list_parts` — returns correct parts
- `test_multipart_list_uploads` — returns in-progress uploads
- `test_multipart_no_such_upload` — 404
- `test_multipart_excluded_from_list_objects` — in-progress uploads invisible
- `test_multipart_etag_format` — ETag matches `"<hex>-<N>"` pattern

**`tests/mc_test.sh`** — Large file upload (15MB, auto-multipart by mc)

**`tests/aws_cli_test.sh`** — Large file upload + verify multipart ETag suffix

## Execution Order
```
Step 0 (errors + types)
  ├→ Step 1 (storage)  ─┐
  └→ Step 2 (XML types) ─┤
                          ├→ Step 3 (handlers)
                          │    └→ Step 4 (router wiring)
                          │         └→ Step 5 (tests)
```
Steps 1 and 2 can run in parallel. Auth layer needs zero changes.

## Verification
1. `cargo test` — all existing + new integration tests pass
2. `cargo build && ./target/debug/maxio --data-dir /tmp/maxio-test --port 9876 &`
3. `./tests/mc_test.sh 9876 /tmp/maxio-test` — includes large file multipart test
4. `./tests/aws_cli_test.sh 9876 /tmp/maxio-test` — includes multipart ETag verification
5. Manual: `dd if=/dev/urandom of=/tmp/big.bin bs=1M count=20 && aws --endpoint-url http://localhost:9876 s3 cp /tmp/big.bin s3://test-bucket/big.bin` — confirm upload succeeds and download matches
