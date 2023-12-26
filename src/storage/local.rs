use std::{io, path::PathBuf};

use axum::body::Body;
use futures::TryStreamExt;
use tokio_util::io::ReaderStream;
use tracing::{error, info};

use crate::{crate_name::CrateName, index::IndexFile};

use super::Error;

pub struct LocalStorage {
    path: PathBuf,
}

impl LocalStorage {
    pub async fn new(config: &crate::config::LocalStorage) -> Result<Self, Error> {
        info!("Using local storage at {}", config.path.display());

        if !tokio::fs::try_exists(&config.path)
            .await
            .map_err(Error::Io)?
        {
            error!("Local storage path does not exist");

            return Err(Error::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "path not found",
            )));
        }

        Ok(Self {
            path: config.path.clone(),
        })
    }

    pub async fn read_index_file(&self, crate_name: &CrateName) -> Result<IndexFile, Error> {
        let file_path = crate_name.index_path().to_path(&self.path);
        let contents = tokio::fs::read(file_path).await.map_err(map_io_error)?;

        IndexFile::from_bytes(&contents).map_err(Error::IndexFile)
    }

    pub async fn read_crate_file(
        &self,
        crate_name: &CrateName,
        version: &semver::Version,
    ) -> Result<Body, Error> {
        let file_path = crate_name
            .crate_path(version)
            .to_path(self.path.join("crates"));

        let file = tokio::fs::File::open(file_path)
            .await
            .map_err(map_io_error)?;

        Ok(Body::from_stream(
            ReaderStream::new(file).map_err(Error::Io),
        ))
    }

    pub async fn write_index_file(
        &self,
        crate_name: &CrateName,
        index_file: &IndexFile,
    ) -> Result<(), Error> {
        let file_path = crate_name.index_path().to_path(&self.path);
        let contents = index_file.to_bytes().map_err(Error::IndexFile)?;

        tokio::fs::write(file_path, contents)
            .await
            .map_err(Error::Io)?;

        Ok(())
    }

    pub async fn write_crate_file(
        &self,
        crate_name: &CrateName,
        version: &semver::Version,
        contents: &[u8],
    ) -> Result<(), Error> {
        let file_path = crate_name
            .crate_path(version)
            .to_path(self.path.join("crates"));

        tokio::fs::write(file_path, contents)
            .await
            .map_err(Error::Io)?;

        Ok(())
    }
}

fn map_io_error(e: io::Error) -> Error {
    if matches!(e.kind(), io::ErrorKind::NotFound) {
        Error::NotFound
    } else {
        Error::Io(e)
    }
}
