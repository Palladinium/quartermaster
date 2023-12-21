use std::{io, path::PathBuf};

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

    pub async fn get_index(&self, crate_name: &CrateName) -> Result<IndexFile, Error> {
        let path = crate_name.index_path();

        // We generate the index path, but just in case...
        if !path.is_relative() {
            return Err(Error::NotFound);
        }

        let file_path = self.path.join(path);
        let contents = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied => Error::NotFound,
                _ => Error::Io(e),
            })?;

        serde_json::from_str(&contents).map_err(Error::Json)
    }
}
