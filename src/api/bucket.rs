use std::collections::HashMap;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
};

use crate::error::S3Error;
use crate::server::AppState;
use crate::storage::{BucketMeta, StorageError};
use crate::xml::{response::to_xml, types::*};

pub async fn list_buckets(State(state): State<AppState>) -> Result<Response<Body>, S3Error> {
    let buckets = state
        .storage
        .list_buckets()
        .await
        .map_err(|e| S3Error::internal(e))?;

    let result = ListAllMyBucketsResult {
        owner: Owner {
            id: "maxio".to_string(),
            display_name: "maxio".to_string(),
        },
        buckets: Buckets {
            bucket: buckets
                .into_iter()
                .map(|b| BucketEntry {
                    name: b.name,
                    creation_date: b.created_at,
                })
                .collect(),
        },
    };

    let xml = to_xml(&result).map_err(|e| S3Error::internal(e))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/xml")
        .body(Body::from(xml))
        .unwrap())
}

pub async fn create_bucket(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
) -> Result<Response<Body>, S3Error> {
    validate_bucket_name(&bucket)?;

    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    let meta = BucketMeta {
        name: bucket.clone(),
        created_at: now,
        region: state.config.region.clone(),
        versioning: false,
    };

    let created = state
        .storage
        .create_bucket(&meta)
        .await
        .map_err(|e| S3Error::internal(e))?;

    if !created {
        return Err(S3Error::bucket_already_owned(&bucket));
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Location", format!("/{}", bucket))
        .body(Body::empty())
        .unwrap())
}

pub async fn head_bucket(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
) -> Result<Response<Body>, S3Error> {
    match state.storage.head_bucket(&bucket).await {
        Ok(true) => {}
        Ok(false) => return Err(S3Error::no_such_bucket(&bucket)),
        Err(e) => return Err(S3Error::internal(e)),
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("x-amz-bucket-region", &*state.config.region)
        .body(Body::empty())
        .unwrap())
}

pub async fn delete_bucket(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
) -> Result<Response<Body>, S3Error> {
    match state.storage.delete_bucket(&bucket).await {
        Ok(true) => Ok(Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .unwrap()),
        Ok(false) => Err(S3Error::no_such_bucket(&bucket)),
        Err(StorageError::BucketNotEmpty) => Err(S3Error::bucket_not_empty(&bucket)),
        Err(e) => Err(S3Error::internal(e)),
    }
}

pub async fn handle_bucket_put(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    body: Body,
) -> Result<Response<Body>, S3Error> {
    if params.contains_key("versioning") {
        return put_bucket_versioning(State(state), Path(bucket), body).await;
    }
    create_bucket(State(state), Path(bucket)).await
}

async fn put_bucket_versioning(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
    body: Body,
) -> Result<Response<Body>, S3Error> {
    match state.storage.head_bucket(&bucket).await {
        Ok(true) => {}
        Ok(false) => return Err(S3Error::no_such_bucket(&bucket)),
        Err(e) => return Err(S3Error::internal(e)),
    }

    let body_bytes = axum::body::to_bytes(body, 1024 * 64)
        .await
        .map_err(|e| S3Error::internal(e))?;
    let body_str = String::from_utf8_lossy(&body_bytes);

    // Parse <VersioningConfiguration><Status>Enabled|Suspended</Status></VersioningConfiguration>
    let enabled = if body_str.contains("<Status>Enabled</Status>") {
        true
    } else if body_str.contains("<Status>Suspended</Status>") {
        false
    } else {
        false
    };

    state
        .storage
        .set_versioning(&bucket, enabled)
        .await
        .map_err(|e| S3Error::internal(e))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap())
}

pub async fn get_bucket_versioning(
    state: AppState,
    bucket: String,
) -> Result<Response<Body>, S3Error> {
    let versioned = state
        .storage
        .is_versioned(&bucket)
        .await
        .map_err(|e| S3Error::internal(e))?;

    let result = VersioningConfiguration {
        status: if versioned {
            Some("Enabled".to_string())
        } else {
            None
        },
    };

    let xml = to_xml(&result).map_err(|e| S3Error::internal(e))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/xml")
        .body(Body::from(xml))
        .unwrap())
}

fn validate_bucket_name(name: &str) -> Result<(), S3Error> {
    if name.len() < 3 || name.len() > 63 {
        return Err(S3Error::invalid_bucket_name(name));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
    {
        return Err(S3Error::invalid_bucket_name(name));
    }
    if !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name.as_bytes()[name.len() - 1].is_ascii_alphanumeric()
    {
        return Err(S3Error::invalid_bucket_name(name));
    }
    Ok(())
}
