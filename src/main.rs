use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use axum::{
    body::{Body, HttpBody},
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    routing::{RouterExt, TypedPath},
    TypedHeader,
};
use error::{ErrorResponse, ResponseError};
use feature_name::FeatureName;
use http_body_util::{BodyExt, LengthLimitError, Limited};
use index::{DependencyKind, IndexConfig, IndexDependency, IndexEntry, IndexFile, MinRustVersion};
use relative_path::RelativePathBuf;
use semver::BuildMetadata;
use serde::{Deserialize, Serialize};
use simple_eyre::eyre;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use url::Url;

mod auth;
mod config;
mod crate_name;
mod error;
mod feature_name;
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
    let lock = RwLock::new(());

    let bind = config.server.bind.clone();

    let state = Arc::new(AppState {
        config,
        auth,
        storage,
        lock,
    });

    let router = Router::new()
        .route("/index/config.json", get(get_index_config))
        .typed_get(get_index_file)
        .typed_get(get_download_crate)
        .route("/api/v1/crates/new", put(put_publish_crate))
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
    lock: RwLock<()>,
}

#[tracing::instrument(skip_all)]
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

#[tracing::instrument(skip(state, authorization))]
async fn get_index_file(
    GetIndexFile { path }: GetIndexFile,
    State(state): State<Arc<AppState>>,
    authorization: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<Vec<u8>, ErrorResponse> {
    state
        .auth
        .authorize(authorization.as_ref().map(|a| a.token()))?;

    let path = RelativePathBuf::from(path);
    let crate_name = CrateName::from_index_path(&path).map_err(ErrorResponse::not_found)?;

    debug!("Fetching index file for crate {crate_name}");

    let index_file = {
        let _guard = state.lock.read().await;
        state.storage.read_index_file(&crate_name).await?
    };

    index_file
        .to_bytes()
        .map_err(ErrorResponse::internal_server_error)
}

#[derive(Debug, Deserialize, TypedPath)]
#[typed_path("/crates/:crate_name/:version/download")]
struct GetDownloadCrate {
    crate_name: String,
    version: String,
}

#[tracing::instrument(skip(state, authorization))]
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

    debug!("Fetching crate file for {crate_name}");

    let body = {
        let _guard = state.lock.read().await;
        state.storage.read_crate_file(&crate_name, &version).await?
    };

    // TODO: Configurable cache control headers?

    Ok(body)
}

// TODO: Make configurable?
const MAX_BODY_SIZE: usize = 100 * 1024 * 1024;

#[derive(Deserialize)]
#[allow(dead_code)]
struct PublishRequest {
    /// The name of the package.
    name: CrateName,
    /// The version of the package being published.
    vers: semver::Version,
    /// Array of direct dependencies of the package.
    deps: Vec<PublishDependency>,
    /// Set of features defined for the package.
    /// Each feature maps to an array of features or dependencies it enables.
    /// Cargo does not impose limitations on feature names, but crates.io
    /// requires alphanumeric ASCII, `_` or `-` characters.
    features: BTreeMap<FeatureName, Vec<String>>,
    /// List of strings of the authors.
    /// May be empty.
    authors: Vec<String>,
    /// Description field from the manifest.
    /// May be null. crates.io requires at least some content.
    description: Option<String>,
    /// String of the URL to the website for this package's documentation.
    /// May be null.
    documentation: Option<Url>,
    /// String of the URL to the website for this package's home page.
    /// May be null.
    homepage: Option<Url>,
    /// String of the content of the README file.
    /// May be null.
    readme: Option<String>,
    /// String of a relative path to a README file in the crate.
    /// May be null.
    readme_file: Option<PathBuf>,
    /// Array of strings of keywords for the package.
    keywords: Vec<String>,
    /// Array of strings of categories for the package.
    categories: Vec<String>,
    /// String of the license for the package.
    /// May be null. crates.io requires either `license` or `license_file` to be set.
    license: Option<String>,
    /// String of a relative path to a license file in the crate.
    /// May be null.
    license_file: Option<PathBuf>,
    /// String of the URL to the website for the source repository of this package.
    /// May be null.
    repository: Option<Url>,
    /// Optional object of "status" badges. Each value is an object of
    /// arbitrary string to string mappings.
    /// crates.io has special interpretation of the format of the badges.
    badges: Option<PublishBadges>,
    /// The `links` string value from the package's manifest, or null if not
    /// specified. This field is optional and defaults to null.
    links: Option<String>,
    /// The minimal supported Rust version (optional)
    /// This must be a valid version requirement without an operator (e.g. no `=`)
    rust_version: MinRustVersion,
}

#[derive(Deserialize)]
struct PublishDependency {
    /// Name of the dependency.
    /// If the dependency is renamed from the original package name,
    /// this is the original name. The new package name is stored in
    /// the `explicit_name_in_toml` field.
    name: String,
    /// The semver requirement for this dependency.
    version_req: semver::VersionReq,
    /// Array of features (as strings) enabled for this dependency.
    features: Vec<String>,
    /// Boolean of whether or not this is an optional dependency.
    optional: bool,
    /// Boolean of whether or not default features are enabled.
    default_features: bool,
    /// The target platform for the dependency.
    /// null if not a target dependency.
    /// Otherwise, a string such as "cfg(windows)".
    target: Option<String>,
    /// The dependency kind.
    /// "dev", "build", or "normal".
    kind: DependencyKind,
    /// The URL of the index of the registry where this dependency is
    /// from as a string. If not specified or null, it is assumed the
    /// dependency is in the current registry.
    registry: Option<Url>,
    /// If the dependency is renamed, this is a string of the new
    /// package name. If not specified or null, this dependency is not
    /// renamed.
    explicit_name_in_toml: Option<String>,
}

type PublishBadges = BTreeMap<String, PublishBadge>;
type PublishBadge = BTreeMap<String, String>;

#[derive(Serialize)]
struct PublishResponse {
    warnings: PublishWarnings,
}

#[derive(Serialize)]
struct PublishWarnings {
    invalid_categories: Vec<String>,
    invalid_badges: Vec<String>,
    other: Vec<String>,
}

#[tracing::instrument(skip_all)]
async fn put_publish_crate(
    State(state): State<Arc<AppState>>,
    authorization: Option<TypedHeader<Authorization<Bearer>>>,
    body: Body,
) -> Result<Json<PublishResponse>, ErrorResponse> {
    state
        .auth
        .authorize(authorization.as_ref().map(|a| a.token()))?;

    let mut warnings = Vec::new();

    let Some(body_size) = body.size_hint().exact() else {
        return Err(ErrorResponse::from_status(StatusCode::LENGTH_REQUIRED));
    };

    if body_size > u64::try_from(MAX_BODY_SIZE).unwrap() {
        return Err(ErrorResponse::from_status(StatusCode::PAYLOAD_TOO_LARGE));
    }

    let body_data = Limited::new(body, MAX_BODY_SIZE)
        .collect()
        .await
        .map_err(|e| {
            if e.is::<LengthLimitError>() {
                ErrorResponse::from_status(StatusCode::PAYLOAD_TOO_LARGE)
            } else {
                ErrorResponse::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            }
        })?
        .to_bytes();

    let json_length_bytes = body_data
        .get(0..4)
        .ok_or_else(|| ErrorResponse::from_status(StatusCode::BAD_REQUEST))?;
    let json_length =
        usize::try_from(u32::from_le_bytes(json_length_bytes.try_into().unwrap())).unwrap();

    let json_bytes = body_data
        .get(4..(json_length + 4))
        .ok_or_else(|| ErrorResponse::from_status(StatusCode::BAD_REQUEST))?;
    let publish_request: PublishRequest =
        serde_json::from_slice(json_bytes).map_err(|e| ErrorResponse {
            status: StatusCode::BAD_REQUEST,
            errors: vec![ResponseError {
                detail: e.to_string(),
            }],
        })?;

    let crate_length_bytes = body_data
        .get((4 + json_length)..(4 + json_length + 4))
        .ok_or_else(|| ErrorResponse::from_status(StatusCode::BAD_REQUEST))?;
    let crate_length =
        usize::try_from(u32::from_le_bytes(crate_length_bytes.try_into().unwrap())).unwrap();

    let crate_data = body_data
        .get((4 + json_length + 4)..(4 + json_length + 4 + crate_length))
        .ok_or_else(|| ErrorResponse::from_status(StatusCode::BAD_REQUEST))?;

    let cksum = sha256::digest(crate_data);

    let crate_name = publish_request.name;
    let crate_version = semver::Version {
        major: publish_request.vers.major,
        minor: publish_request.vers.minor,
        patch: publish_request.vers.patch,
        pre: publish_request.vers.pre,
        // We ignore build metadata
        build: BuildMetadata::EMPTY,
    };

    if !publish_request.vers.build.is_empty() {
        warn!("Ignoring build metadata");
        warnings.push(format!(
            "Build metadata in crate version was ignored: {}",
            &publish_request.vers.build
        ));
    }

    {
        let _guard = state.lock.write().await;

        // Load the index (if it exists) and check that this crate version doesn't already exist

        let mut index_file = match state.storage.read_index_file(&crate_name).await {
            Ok(index) => index,
            Err(storage::Error::NotFound) => IndexFile::default(),
            Err(e) => return Err(e.into()),
        };

        if index_file
            .entries
            .iter()
            .any(|entry| entry.vers == crate_version)
        {
            return Err(ErrorResponse {
                status: StatusCode::BAD_REQUEST,
                errors: vec![ResponseError {
                    detail: format!("Crate {crate_name} already has version {crate_version}"),
                }],
            });
        }

        // Construct the new index entry and append it to the index
        let index_entry = IndexEntry {
            name: crate_name.clone(),
            vers: crate_version.clone(),
            deps: publish_request
                .deps
                .into_iter()
                .map(|dep| {
                    let (name, package) =
                        if let Some(explicit_name_in_toml) = dep.explicit_name_in_toml {
                            // The dependency has been renamed
                            (explicit_name_in_toml, Some(dep.name))
                        } else {
                            (dep.name, None)
                        };

                    IndexDependency {
                        name,
                        req: dep.version_req,
                        features: dep.features,
                        optional: dep.optional,
                        default_features: dep.default_features,
                        target: dep.target,
                        kind: dep.kind,
                        registry: dep.registry,
                        package,
                    }
                })
                .collect(),
            cksum,
            features: publish_request.features,
            yanked: false,
            links: publish_request.links,
            // NOTE: crates.io ignores this field and instead reads it from the Cargo.toml in the .crate file
            rust_version: publish_request.rust_version,
        };

        index_file.entries.push(index_entry);

        // Write the crate to storage, and then the index
        state
            .storage
            .write_crate_file(&crate_name, &crate_version, crate_data)
            .await?;

        state
            .storage
            .write_index_file(&crate_name, &index_file)
            .await?;
    }

    Ok(Json(PublishResponse {
        warnings: PublishWarnings {
            invalid_categories: Vec::new(),
            invalid_badges: Vec::new(),
            other: warnings,
        },
    }))
}
