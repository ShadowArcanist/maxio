use axum::Router;
use std::sync::Arc;

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

    // Phase 3: web console routes at /ui/
    Router::new().merge(s3_routes).with_state(state)
}
