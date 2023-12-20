use std::{fmt::Display, path::PathBuf, sync::Arc};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    routing::{RouterExt, TypedPath},
    TypedHeader,
};
use error::ErrorResponse;
use index::IndexConfig;
use serde::Deserialize;
use simple_eyre::eyre;
use tracing::{debug, error};

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

    let auth_config = config.auth.clone();
    let auth = tokio::task::spawn_blocking(move || auth::Auth::new(&auth_config)).await?;
    let storage = storage::Storage::new(&config.storage).await?;

    let state = Arc::new(AppState {
        config: config.clone(),
        auth,
        storage,
    });

    let router = Router::new()
        .route("/index/config.json", get(get_index_config))
        .typed_get(get_index)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.server.bind.as_slice()).await?;
    axum::serve(listener, router).await?;

    println!("Hello, world!");

    Ok(())
}

async fn get_index_config(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(IndexConfig {
        dl: state
            .config
            .server
            .root_url
            .join("crates")
            .map_err(internal_server_error)?,

        api: state
            .config
            .server
            .root_url
            .join("api")
            .map_err(internal_server_error)?,

        auth_required: state.auth.auth_required(),
    }))
}

#[derive(Debug, Deserialize, TypedPath)]
#[typed_path("/index/*path")]
pub struct GetIndex {
    path: String,
}

#[tracing::instrument(skip(state))]
async fn get_index(
    GetIndex { path }: GetIndex,
    State(state): State<Arc<AppState>>,
    authorization: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<impl IntoResponse, ErrorResponse> {
    state
        .auth
        .authorize(authorization.as_ref().map(|a| a.token()))?;

    let path = PathBuf::from(path);
    let crate_name = CrateName::from_index_path(&path).map_err(ErrorResponse::not_found)?;

    debug!("Fetching index file for crate {crate_name}");

    Ok(Json(state.storage.get_index(&crate_name).await?))
}

struct AppState {
    config: Config,
    auth: auth::Auth,
    storage: storage::Storage,
}

fn internal_server_error<E: Display>(error: E) -> StatusCode {
    error!("Responding with 500 Internal Server Error: {error}");
    StatusCode::INTERNAL_SERVER_ERROR
}
