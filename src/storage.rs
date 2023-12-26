use std::{io, str::FromStr};

use axum::{body::Body, http::StatusCode};

use crate::{
    crate_name::CrateName,
    error::{ErrorResponse, ResponseError},
    index::{IndexFile, IndexFileError},
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

    // TODO: Add an option to just fetch the index-file as is or genrate a redirect, without always reserializing it
    pub async fn read_index_file(&self, name: &CrateName) -> Result<IndexFile, Error> {
        match self {
            Storage::Local(local) => local.read_index_file(name).await,
            #[cfg(feature = "s3")]
            Storage::S3(s3) => s3.read_index_file(name).await,
        }
    }

    pub async fn read_crate_file(
        &self,
        name: &CrateName,
        version: &semver::Version,
    ) -> Result<Body, Error> {
        match self {
            Storage::Local(local) => local.read_crate_file(name, version).await,
            #[cfg(feature = "s3")]
            Storage::S3(s3) => s3.read_crate_file(name, version).await,
        }
    }

    pub async fn write_index_file(
        &self,
        name: &CrateName,
        index_file: &IndexFile,
    ) -> Result<(), Error> {
        match self {
            Storage::Local(local) => local.write_index_file(name, index_file).await,
            #[cfg(feature = "s3")]
            Storage::S3(s3) => s3.write_index_file(name, index_file).await,
        }
    }

    pub async fn write_crate_file(
        &self,
        name: &CrateName,
        version: &semver::Version,
        contents: &[u8],
    ) -> Result<(), Error> {
        match self {
            Storage::Local(local) => local.write_crate_file(name, version, contents).await,
            #[cfg(feature = "s3")]
            Storage::S3(s3) => s3.write_crate_file(name, version, contents).await,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Crate not found")]
    /// NOTE: Storage implementations should only return this variant if they positively know
    /// the crate does not exist. In the case of an index file being inaccessible due to permissions
    /// on the storage backend, a different error should be returned so that Quartermaster
    /// doesn't attempt to overwrite a potentially present but not readable index file.
    NotFound,
    #[error("IO error")]
    Io(#[source] io::Error),
    #[error("Error parsing index file")]
    IndexFile(#[source] IndexFileError),

    #[cfg(feature = "s3")]
    #[error("S3 error")]
    S3(#[source] ::s3::error::S3Error),

    #[cfg(feature = "s3")]
    #[error("S3 credentials error")]
    S3Credentials(#[source] ::s3::creds::error::CredentialsError),

    #[cfg(feature = "s3")]
    #[error("Invalid S3 region")]
    S3Region(<::s3::region::Region as FromStr>::Err),
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
            Error::Io(_) | Error::IndexFile(_) => ErrorResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                errors: vec![ResponseError {
                    detail: String::from("Error fetching file"),
                }],
            },

            #[cfg(feature = "s3")]
            Error::S3(_) | Error::S3Credentials(_) | Error::S3Region(_) => ErrorResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                errors: vec![ResponseError {
                    detail: String::from("Error fetching file"),
                }],
            },
        }
    }
}
