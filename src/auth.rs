use axum::http::StatusCode;
use tracing::{info, warn};

use crate::error::ErrorResponse;

pub enum Auth {
    None,
    EnvTokens,
}

impl Auth {
    pub fn new(config: &crate::config::Auth) -> Self {
        match config {
            crate::config::Auth::None => {
                warn!("Disabling authentication!");
                warn!("This is generally a BAD IDEA, as any request can freely read and modify the registry.");

                Self::None
            }

            crate::config::Auth::EnvTokens => {
                info!("Loading allowed auth tokens from environment variable");

                Self::EnvTokens
            }
        }
    }

    pub fn auth_required(&self) -> bool {
        match self {
            Self::None => false,
            Self::EnvTokens => true,
        }
    }

    pub fn authorize(&self, token: Option<&str>) -> Result<(), Error> {
        match self {
            Self::None => Ok(()),
            Self::EnvTokens => todo!(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The provided token is invalid")]
    Forbidden,
    #[error("No authorization token was provided")]
    Unauthorized,
}

impl From<Error> for ErrorResponse {
    fn from(e: Error) -> Self {
        match e {
            Error::Forbidden => ErrorResponse {
                status: StatusCode::FORBIDDEN,
                errors: Vec::new(),
            },
            Error::Unauthorized => ErrorResponse {
                status: StatusCode::UNAUTHORIZED,
                errors: Vec::new(),
            },
        }
    }
}
