use std::io;

use axum::{body::Body, http::StatusCode};

use crate::{
    crate_name::CrateName,
    error::{ErrorResponse, ResponseError},
    index::IndexFile,
};

#[cfg(feature = "s3")]
pub mod s3;

pub mod local;

pub enum Storage {
    Local(local::LocalStorage),
    #[cfg(feature = "s3")]
    S3(s3::S3Storage),
}

impl Storage {
    pub async fn new(config: &crate::config::Storage) -> Result<Self, Error> {
        match config {
            crate::config::Storage::Local(local) => {
                Ok(Self::Local(local::LocalStorage::new(local).await?))
            }
            #[cfg(feature = "s3")]
            crate::config::Storage::S3(s3) => Ok(Self::S3(s3::S3Storage::new(s3)?)),
        }
    }

    pub async fn get_index(&self, crate_name: &CrateName) -> Result<IndexFile, Error> {
        match self {
            Storage::Local(local) => local.get_index(crate_name).await,
            #[cfg(feature = "s3")]
            Storage::S3(s3) => s3.get_index(crate_name).await,
        }
    }

    pub async fn get_crate(
        &self,
        crate_name: &CrateName,
        version: semver::Version,
    ) -> Result<Body, Error> {
        match self {
            Storage::Local(local) => local.get_crate(crate_name, version).await,
            #[cfg(feature = "s3")]
            Storage::S3(s3) => s3.get_crate(crate_name, version).await,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Crate not found")]
    NotFound,
    #[error("IO error")]
    Io(#[source] io::Error),
    #[error("JSON error")]
    Json(#[source] serde_json::Error),

    #[cfg(feature = "s3")]
    #[error("S3 error")]
    S3(#[source] ::s3::error::S3Error),
    #[cfg(feature = "s3")]
    #[error("S3 configuration error")]
    S3Configuration(#[from] s3::ConfigurationError),
}

impl From<Error> for ErrorResponse {
    fn from(e: Error) -> Self {
        match e {
            Error::NotFound => ErrorResponse {
                status: StatusCode::NOT_FOUND,
                errors: vec![ResponseError {
                    detail: String::from("Crate not found"),
                }],
            },
            Error::Io(_) | Error::Json(_) => ErrorResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                errors: vec![ResponseError {
                    detail: String::from("Error fetching index file"),
                }],
            },

            #[cfg(feature = "s3")]
            Error::S3(_) | Error::S3Configuration(_) => ErrorResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                errors: vec![ResponseError {
                    detail: String::from("Error fetching index file"),
                }],
            },
        }
    }
}
