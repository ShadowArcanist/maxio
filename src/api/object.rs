use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use futures::TryStreamExt;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio_util::io::ReaderStream;

use crate::error::S3Error;
use crate::server::AppState;
use crate::storage::StorageError;

use super::multipart;

pub async fn put_object(
    State(state): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response<Body>, S3Error> {
    if params.contains_key("uploadId") {
        return multipart::upload_part(
            State(state),
            Path((bucket, key)),
            Query(params),
            headers,
            body,
        )
        .await;
    }

    match state.storage.head_bucket(&bucket).await {
        Ok(true) => {}
        Ok(false) => return Err(S3Error::no_such_bucket(&bucket)),
        Err(e) => return Err(S3Error::internal(e)),
    }

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    let reader = body_to_reader(&headers, body).await?;

    // Verify Content-MD5 if provided
    let content_md5 = headers
        .get("content-md5")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let result = state
        .storage
        .put_object(&bucket, &key, content_type, reader)
        .await
        .map_err(|e| match e {
            StorageError::InvalidKey(msg) => S3Error::invalid_argument(&msg),
            _ => S3Error::internal(e),
        })?;

    if let Some(expected_md5) = content_md5 {
        let hex_md5 = result.etag.trim_matches('"');
        if let Ok(md5_bytes) = hex::decode(hex_md5) {
            use base64::Engine;
            let computed_md5 = base64::engine::general_purpose::STANDARD.encode(&md5_bytes);
            if computed_md5 != expected_md5 {
                let _ = state.storage.delete_object(&bucket, &key).await;
                return Err(S3Error::bad_digest());
            }
        }
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("ETag", &result.etag)
        .header("Content-Length", result.size.to_string())
        .body(Body::empty())
        .unwrap())
}

/// Convert ISO 8601 timestamp to HTTP date (RFC 7231) for Last-Modified header.
fn to_http_date(iso: &str) -> String {
    chrono::DateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S%.3fZ")
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(iso))
        .map(|dt| dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
        .unwrap_or_else(|_| iso.to_string())
}

pub async fn get_object(
    State(state): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response<Body>, S3Error> {
    if params.contains_key("uploadId") {
        return multipart::list_parts(State(state), Path((bucket, key)), Query(params)).await;
    }

    let (reader, meta) = state
        .storage
        .get_object(&bucket, &key)
        .await
        .map_err(|e| match e {
            StorageError::NotFound(_) => S3Error::no_such_key(&key),
            StorageError::InvalidKey(msg) => S3Error::invalid_argument(&msg),
            _ => S3Error::internal(e),
        })?;

    let stream = ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", &meta.content_type)
        .header("Content-Length", meta.size.to_string())
        .header("ETag", &meta.etag)
        .header("Last-Modified", to_http_date(&meta.last_modified))
        .body(body)
        .unwrap())
}

pub async fn head_object(
    State(state): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response<Body>, S3Error> {
    let meta = state
        .storage
        .head_object(&bucket, &key)
        .await
        .map_err(|e| match e {
            StorageError::NotFound(_) => S3Error::no_such_key(&key),
            StorageError::InvalidKey(msg) => S3Error::invalid_argument(&msg),
            _ => S3Error::internal(e),
        })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", &meta.content_type)
        .header("Content-Length", meta.size.to_string())
        .header("ETag", &meta.etag)
        .header("Last-Modified", to_http_date(&meta.last_modified))
        .body(Body::empty())
        .unwrap())
}

pub async fn delete_object(
    State(state): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response<Body>, S3Error> {
    if params.contains_key("uploadId") {
        return multipart::abort_multipart_upload(State(state), Path((bucket, key)), Query(params))
            .await;
    }

    state.storage.delete_object(&bucket, &key).await
        .map_err(|e| S3Error::internal(e))?;

    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap())
}

pub async fn post_object(
    State(state): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response<Body>, S3Error> {
    if params.contains_key("uploads") {
        return multipart::create_multipart_upload(State(state), Path((bucket, key)), headers).await;
    }
    if params.contains_key("uploadId") {
        return multipart::complete_multipart_upload(
            State(state),
            Path((bucket, key)),
            Query(params),
            body,
        )
        .await;
    }
    Err(S3Error::not_implemented("Unsupported POST object operation"))
}

const DELETE_BODY_MAX: usize = 1024 * 1024;

/// Handle POST /{bucket}?delete — multi-object delete (DeleteObjects API).
pub async fn delete_objects(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
    body: Body,
) -> Result<Response<Body>, S3Error> {
    let bytes = axum::body::to_bytes(body, DELETE_BODY_MAX)
        .await
        .map_err(|e| S3Error::internal(e))?;
    let body_str = String::from_utf8_lossy(&bytes);

    let mut keys = Vec::new();
    let mut reader = quick_xml::Reader::from_str(&body_str);
    reader.config_mut().trim_text(true);
    let mut in_key = false;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(e)) if e.name().as_ref() == b"Key" => {
                in_key = true;
            }
            Ok(quick_xml::events::Event::Text(e)) if in_key => {
                keys.push(e.unescape().unwrap_or_default().into_owned());
                in_key = false;
            }
            Ok(quick_xml::events::Event::End(e)) if e.name().as_ref() == b"Key" => {
                in_key = false;
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => return Err(S3Error::malformed_xml()),
            _ => {}
        }
    }

    let mut set = tokio::task::JoinSet::new();
    for key in keys {
        let storage = state.storage.clone();
        let bucket = bucket.clone();
        set.spawn(async move {
            let result = storage.delete_object(&bucket, &key).await;
            (key, result)
        });
    }

    let mut deleted_xml = String::new();
    let mut error_xml = String::new();
    while let Some(result) = set.join_next().await {
        if let Ok((key, delete_result)) = result {
            match delete_result {
                Ok(()) => {
                    deleted_xml.push_str(&format!(
                        "<Deleted><Key>{}</Key></Deleted>",
                        quick_xml::escape::escape(&key)
                    ));
                }
                Err(e) => {
                    error_xml.push_str(&format!(
                        "<Error><Key>{}</Key><Code>InternalError</Code><Message>{}</Message></Error>",
                        quick_xml::escape::escape(&key),
                        quick_xml::escape::escape(&e.to_string())
                    ));
                }
            }
        }
    }

    let response_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <DeleteResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">{}{}</DeleteResult>",
        deleted_xml, error_xml
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/xml")
        .body(Body::from(response_xml))
        .unwrap())
}

pub(crate) async fn body_to_reader(
    headers: &HeaderMap,
    body: Body,
) -> Result<std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>, S3Error> {
    let is_aws_chunked = headers
        .get("x-amz-content-sha256")
        .and_then(|v| v.to_str().ok())
        == Some("STREAMING-AWS4-HMAC-SHA256-PAYLOAD");

    let stream = body.into_data_stream();
    let raw_reader = tokio_util::io::StreamReader::new(
        stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
    );

    if is_aws_chunked {
        let mut buf_reader = tokio::io::BufReader::new(raw_reader);
        let mut decoded = Vec::new();
        loop {
            let mut line = String::new();
            let n = buf_reader
                .read_line(&mut line)
                .await
                .map_err(S3Error::internal)?;
            if n == 0 {
                break;
            }
            let line = line.trim_end_matches(|c| c == '\r' || c == '\n');
            let size_str = line.split(';').next().unwrap_or("0");
            let chunk_size = usize::from_str_radix(size_str.trim(), 16)
                .map_err(|_| S3Error::internal("invalid chunk size"))?;
            if chunk_size == 0 {
                break;
            }
            let mut chunk = vec![0u8; chunk_size];
            buf_reader
                .read_exact(&mut chunk)
                .await
                .map_err(S3Error::internal)?;
            decoded.extend_from_slice(&chunk);
            let mut crlf = [0u8; 2];
            let _ = buf_reader.read_exact(&mut crlf).await;
        }
        Ok(Box::pin(std::io::Cursor::new(decoded)))
    } else {
        Ok(Box::pin(raw_reader))
    }
}
