use axum::Router;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

use crate::api::console::console_router;
use crate::api::router::s3_router;
use crate::auth::middleware::auth_middleware;
use crate::config::Config;
use crate::storage::filesystem::FilesystemStorage;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<FilesystemStorage>,
    pub config: Arc<Config>,
}

pub fn build_router(state: AppState) -> Router {
    let s3_routes = s3_router().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        auth_middleware,
    ));

    // Web console SPA at /ui/ — no auth middleware (separate auth later)
    // fallback_service returns index.html for client-side routing
    let ui_service = ServeDir::new("ui/dist")
        .fallback(ServeFile::new("ui/dist/index.html"));

    Router::new()
        .nest("/api", console_router(state.clone()))
        .nest_service("/ui", ui_service)
        .merge(s3_routes)
        .with_state(state)
}
