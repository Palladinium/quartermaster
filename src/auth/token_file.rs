use std::{
    fmt::{self, Debug, Formatter},
    io,
    path::Path,
};

use base64::Engine;
use tracing::info;

use crate::auth::Error;

pub struct TokenFile {
    token: String,
}

impl TokenFile {
    pub async fn new(config: &crate::config::TokenFileAuth) -> Result<Self, Error> {
        let token = match tokio::fs::read_to_string(&config.token_file).await {
            Ok(token) => token,
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    info!("Token file not found, generating new token");

                    let new_token = generate_token()?;

                    if let Some(parent) = config.token_file.parent() {
                        tokio::fs::create_dir_all(&parent)
                            .await
                            .map_err(Error::Io)?;
                    }

                    save_token(&config.token_file, &new_token).await?;

                    new_token
                } else {
                    return Err(Error::Io(e));
                }
            }
        };

        Ok(Self { token })
    }

    pub fn authorize(&self, token: Option<&str>) -> Result<(), Error> {
        let token = token.ok_or(Error::Unauthorized)?;

        if self.token == token {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
}

impl Debug for TokenFile {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("TokenFile")
            .field("token", &"<REDACTED>")
            .finish()
    }
}

fn generate_token() -> Result<String, Error> {
    let mut bytes = [0; 64];
    getrandom::getrandom(&mut bytes).map_err(Error::Random)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[cfg(unix)]
async fn save_token(path: &Path, token: &str) -> Result<(), Error> {
    use tokio::{fs::OpenOptions, io::AsyncWriteExt};

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .mode(0o600)
        .open(path)
        .await
        .map_err(Error::Io)?;
    file.write_all(token.as_bytes()).await.map_err(Error::Io)?;

    Ok(())
}

// TODO: Figure out how to make this more secure on non-unix platforms
#[cfg(not(unix))]
async fn save_token(path: &Path, token: &str) -> Result<(), Error> {
    tokio::fs::write(path, token).await.map_err(Error::Io)
}
