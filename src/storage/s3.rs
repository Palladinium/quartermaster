use std::{borrow::Cow, env, path::PathBuf, str::FromStr};

use axum::body::Body;

use crate::{crate_name::CrateName, index::IndexFile, storage::Error};

pub struct S3Storage {
    bucket: s3::Bucket,
}

impl S3Storage {
    pub fn new(config: &crate::config::S3Storage) -> Result<Self, Error> {
        let credentials = load_credentials(config).map_err(ConfigurationError::Credentials)?;

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

        serde_json::from_slice(contents.as_slice()).map_err(Error::Json)
    }

    pub async fn get_crate(
        &self,
        crate_name: &CrateName,
        version: semver::Version,
    ) -> Result<Body, Error> {
        let file_path = PathBuf::from("crates").join(crate_name.crate_path(version));

        // TODO: rust-s3 has a get_object_stream method, but its return type is !Send, so we can't convert it to a Body.
        // So we just have to download the whole file and then serve it all at once rather than streaming it.
        let data = self
            .bucket
            .get_object(file_path.to_str().unwrap())
            .await
            .map_err(Error::S3)?;

        Ok(Body::from(data.to_vec()))
    }
}

#[tracing::instrument]
pub fn load_credentials(
    config: &crate::config::S3Storage,
) -> Result<s3::creds::Credentials, s3::creds::error::CredentialsError> {
    if let Some(session_name) = &config.sts_session_name {
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
        return s3::creds::Credentials::from_profile(config.profile_section.as_deref());
    }

    if config.use_instance_credentials {
        return s3::creds::Credentials::from_instance_metadata();
    }

    s3::creds::Credentials::new(
        config.aws_access_key_id.as_deref(),
        config.aws_secret_access_key.as_deref(),
        config.aws_security_token.as_deref(),
        config.aws_session_token.as_deref(),
        None,
    )
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error("S3 credentials error")]
    Credentials(#[source] ::s3::creds::error::CredentialsError),
    #[error("Invalid S3 region")]
    Region(<::s3::region::Region as FromStr>::Err),
}
