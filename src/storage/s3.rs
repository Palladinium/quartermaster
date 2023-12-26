use std::{borrow::Cow, env};

use axum::body::Body;
use relative_path::RelativePathBuf;
use tracing::info;

use crate::{crate_name::CrateName, index::IndexFile, storage::Error};

pub struct S3Storage {
    bucket: s3::Bucket,
}

impl S3Storage {
    pub fn new(config: &crate::config::S3Storage) -> Result<Self, Error> {
        let credentials = load_credentials(config).map_err(Error::S3Credentials)?;

        let bucket = s3::Bucket::new(
            &config.bucket,
            config.region.parse().map_err(Error::S3Region)?,
            credentials,
        )
        .map_err(Error::S3)?;

        Ok(Self { bucket })
    }

    pub async fn read_index_file(&self, name: &CrateName) -> Result<IndexFile, Error> {
        let contents = self
            .bucket
            .get_object(name.index_path().as_str())
            .await
            .map_err(|e| {
                if matches!(e, s3::error::S3Error::Http(404, _)) {
                    Error::NotFound
                } else {
                    Error::S3(e)
                }
            })?;

        IndexFile::from_bytes(contents.as_slice()).map_err(Error::IndexFile)
    }

    pub async fn read_crate_file(
        &self,
        name: &CrateName,
        version: &semver::Version,
    ) -> Result<Body, Error> {
        let file_path = RelativePathBuf::from("crates").join(name.crate_path(version));

        // TODO: rust-s3 has a get_object_stream method, but its return type is !Send, so we can't convert it to a Body.
        // So we just have to download the whole file and then serve it all at once rather than streaming it.
        // We could probably use a redirect to a presigned URL instead to avoid this.
        let data = self
            .bucket
            .get_object(file_path.as_str())
            .await
            .map_err(Error::S3)?;

        Ok(Body::from(data.to_vec()))
    }

    pub async fn write_index_file(
        &self,
        name: &CrateName,
        index_file: &IndexFile,
    ) -> Result<(), Error> {
        let contents = index_file.to_bytes().map_err(Error::IndexFile)?;

        self.bucket
            .put_object(name.index_path().as_str(), &contents)
            .await
            .map_err(Error::S3)?;

        Ok(())
    }

    pub async fn write_crate_file(
        &self,
        name: &CrateName,
        version: &semver::Version,
        contents: &[u8],
    ) -> Result<(), Error> {
        self.bucket
            .put_object(name.crate_path(version).as_str(), contents)
            .await
            .map_err(Error::S3)?;

        Ok(())
    }
}

#[tracing::instrument]
pub fn load_credentials(
    config: &crate::config::S3Storage,
) -> Result<s3::creds::Credentials, s3::creds::error::CredentialsError> {
    if let Some(session_name) = &config.sts_session_name {
        info!("Fetching STS credentials");

        let role_arn = if let Some(role_arn) = &config.sts_role_arn {
            Cow::Borrowed(role_arn)
        } else {
            Cow::Owned(env::var("AWS_ROLE_ARN")?)
        };

        let web_identity_token_file =
            if let Some(web_identity_token_file) = &config.sts_web_identity_token_file {
                Cow::Borrowed(web_identity_token_file)
            } else {
                Cow::Owned(env::var("AWS_WEB_IDENTITY_TOKEN_FILE")?)
            };

        let web_identity_token = std::fs::read_to_string(web_identity_token_file.as_str())?;

        return s3::creds::Credentials::from_sts(&role_arn, session_name, &web_identity_token);
    }

    if config.use_profile_credentials {
        info!("Using profile credentials");
        return s3::creds::Credentials::from_profile(config.profile_section.as_deref());
    }

    if config.use_instance_credentials {
        info!("Using instance metadata credentials");
        return s3::creds::Credentials::from_instance_metadata();
    }

    // Credentials::new will try automatically detecting credentials if the access_key is None
    match (&config.aws_access_key_id, config.auto_credentials) {
        (Some(_), _) => info!("Using explicit credentials"),
        (None, true) => info!("Automatically detecting credentials"),
        (None, false) => return Err(s3::creds::error::CredentialsError::ConfigNotFound),
    }

    s3::creds::Credentials::new(
        config.aws_access_key_id.as_deref(),
        config.aws_secret_access_key.as_deref(),
        config.aws_security_token.as_deref(),
        config.aws_session_token.as_deref(),
        None,
    )
}
