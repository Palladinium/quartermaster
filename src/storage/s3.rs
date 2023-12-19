use std::{env, str::FromStr};

use crate::{crate_name::CrateName, index::IndexFile, storage::Error};

pub struct S3Storage {
    bucket: s3::Bucket,
}

impl S3Storage {
    pub fn new(config: &crate::config::S3Storage) -> Result<Self, Error> {
        let credentials = load_credentials().map_err(ConfigurationError::Credentials)?;

        let bucket = s3::Bucket::new(
            &config.bucket,
            config.region.parse().map_err(ConfigurationError::Region)?,
            credentials,
        )
        .map_err(Error::S3)?;

        Ok(Self { bucket })
    }

    pub async fn get_index(&self, crate_name: &CrateName) -> Result<IndexFile, Error> {
        let path = crate_name.index_path();

        // TODO: rust-s3 has very undefined semantics for their error types, so we can't intercept "not found" errors
        let contents = self
            .bucket
            .get_object(path.to_str().unwrap())
            .await
            .map_err(Error::S3)?;

        Ok(serde_json::from_slice(contents.as_slice()).map_err(Error::Json)?)
    }
}

#[tracing::instrument]
pub fn load_credentials() -> Result<s3::creds::Credentials, s3::creds::error::CredentialsError> {
    // TODO: Maybe put in a PR with rust-s3 to have this work out of the box with Credentials::new
    if let Ok(sts_session_name) = env::var("STS_SESSION_NAME") {
        s3::creds::Credentials::from_sts_env(&sts_session_name)
    } else {
        s3::creds::Credentials::new(None, None, None, None, None)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error("S3 credentials error")]
    Credentials(#[source] ::s3::creds::error::CredentialsError),
    #[error("Invalid S3 region")]
    Region(<::s3::region::Region as FromStr>::Err),
}
