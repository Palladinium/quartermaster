use std::{path::PathBuf, sync::Arc};

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    routing::{RouterExt, TypedPath},
    TypedHeader,
};
use error::ErrorResponse;
use index::{IndexConfig, IndexFile};
use serde::Deserialize;
use simple_eyre::eyre;
use tracing::debug;

mod auth;
mod config;
mod crate_name;
mod error;
mod index;
mod storage;

use crate::{config::Config, crate_name::CrateName};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    simple_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config = Config::load()?;

    let auth = auth::Auth::new(&config.auth);
    let storage = storage::Storage::new(&config.storage).await?;

    let bind = config.server.bind.clone();

    let state = Arc::new(AppState {
        config,
        auth,
        storage,
    });

    let router = Router::new()
        .route("/index/config.json", get(get_index_config))
        .typed_get(get_index_file)
        .typed_get(get_download_crate)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind.as_slice()).await?;
    axum::serve(listener, router).await?;

    println!("Hello, world!");

    Ok(())
}

struct AppState {
    config: Config,
    auth: auth::Auth,
    storage: storage::Storage,
}

async fn get_index_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<IndexConfig>, ErrorResponse> {
    Ok(Json(IndexConfig {
        dl: state
            .config
            .server
            .root_url
            .join("crates")
            .map_err(ErrorResponse::internal_server_error)?,

        api: state
            .config
            .server
            .root_url
            .join("api")
            .map_err(ErrorResponse::internal_server_error)?,

        auth_required: state.auth.auth_required(),
    }))
}

#[derive(Debug, Deserialize, TypedPath)]
#[typed_path("/index/*path")]
struct GetIndexFile {
    path: String,
}

#[tracing::instrument(skip(state))]
async fn get_index_file(
    GetIndexFile { path }: GetIndexFile,
    State(state): State<Arc<AppState>>,
    authorization: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<Json<IndexFile>, ErrorResponse> {
    state
        .auth
        .authorize(authorization.as_ref().map(|a| a.token()))?;

    let path = PathBuf::from(path);
    let crate_name = CrateName::from_index_path(&path).map_err(ErrorResponse::not_found)?;

    debug!("Fetching index file for crate {crate_name}");

    Ok(Json(state.storage.get_index(&crate_name).await?))
}

#[derive(Debug, Deserialize, TypedPath)]
#[typed_path("/crates/:crate_name/:version/download")]
struct GetDownloadCrate {
    crate_name: String,
    version: String,
}

async fn get_download_crate(
    GetDownloadCrate {
        crate_name,
        version,
    }: GetDownloadCrate,
    State(state): State<Arc<AppState>>,
    authorization: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<impl IntoResponse, ErrorResponse> {
    state
        .auth
        .authorize(authorization.as_ref().map(|a| a.token()))?;

    let crate_name = CrateName::new(&crate_name).map_err(ErrorResponse::not_found)?;
    let version = semver::Version::parse(&version).map_err(ErrorResponse::not_found)?;

    let body = state.storage.get_crate(&crate_name, version).await?;

    debug!("Fetching crate file for {crate_name}");

    // TODO: Configurable cache control headers?

    Ok(body)
}
