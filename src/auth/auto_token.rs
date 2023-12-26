use std::{
    fmt::{self, Debug, Formatter},
    io,
};

use base64::Engine;
use tracing::info;

use crate::auth::Error;

pub struct AutoToken {
    token: String,
}

impl AutoToken {
    pub async fn new(config: &crate::config::AutoTokenAuth) -> Result<Self, Error> {
        let token = match tokio::fs::read_to_string(&config.token_file).await {
            Ok(token) => token,
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    info!("Token file not found, generating new token");

                    let new_token = generate_token()?;

                    if let Some(parent) = config.token_file.parent() {
                        tokio::fs::create_dir_all(parent).await.map_err(Error::Io)?;
                    }

                    tokio::fs::write(&config.token_file, &new_token)
                        .await
                        .map_err(Error::Io)?;

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

impl Debug for AutoToken {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("AutoToken")
            .field("token", &"<REDACTED>")
            .finish()
    }
}

fn generate_token() -> Result<String, Error> {
    let mut bytes = [0; 64];
    getrandom::getrandom(&mut bytes).map_err(Error::Random)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}
