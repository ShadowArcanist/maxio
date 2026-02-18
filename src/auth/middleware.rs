use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::error::S3Error;
use crate::server::AppState;

use super::signature_v4;

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, S3Error> {
    let method = request.method().as_str().to_string();
    let uri = request.uri().to_string();

    tracing::debug!("{} {}", method, uri);

    let auth_header = match request.headers().get("authorization") {
        Some(h) => h
            .to_str()
            .map_err(|_| S3Error::access_denied("Invalid Authorization header"))?,
        None => {
            tracing::debug!("No Authorization header present");
            return Err(S3Error::access_denied("Missing Authorization header"));
        }
    };

    tracing::debug!("Authorization: {}", auth_header);

    let parsed = signature_v4::parse_authorization_header(auth_header)
        .map_err(|e| S3Error::access_denied(e))?;

    tracing::debug!(
        "Parsed: access_key={}, date={}, region={}, signed_headers={:?}",
        parsed.access_key,
        parsed.date,
        parsed.region,
        parsed.signed_headers
    );

    if parsed.access_key != state.config.access_key {
        tracing::debug!(
            "Access key mismatch: got '{}', expected '{}'",
            parsed.access_key,
            state.config.access_key
        );
        return Err(S3Error::invalid_access_key());
    }

    if parsed.region != state.config.region {
        tracing::debug!(
            "Region mismatch: got '{}', expected '{}'",
            parsed.region,
            state.config.region
        );
        return Err(S3Error::access_denied("Invalid region in credential scope"));
    }

    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or("").to_string();

    tracing::debug!("Verifying signature for {} {} ?{}", method, path, query);

    // Log all headers that are part of signing
    for h in &parsed.signed_headers {
        let val = request
            .headers()
            .get(h.as_str())
            .and_then(|v| v.to_str().ok())
            .unwrap_or("<missing>");
        tracing::debug!("  signed header '{}': '{}'", h, val);
    }

    let valid = signature_v4::verify_signature(
        &method,
        &path,
        &query,
        request.headers(),
        &parsed,
        &state.config.secret_key,
    );

    if !valid {
        tracing::debug!("Signature verification FAILED");
        return Err(S3Error::signature_mismatch());
    }

    tracing::debug!("Signature verification OK");
    let response = next.run(request).await;
    tracing::debug!("{} {} -> {}", method, uri, response.status());
    Ok(response)
}
