use std::io;

use axum::http::StatusCode;
use tracing::{info, warn};

use crate::error::ErrorResponse;

pub mod auto_token;

pub enum Auth {
    None,
    TokenList(auto_token::AutoToken),
}

impl Auth {
    pub async fn new(config: &crate::config::Auth) -> Result<Self, Error> {
        match config {
            crate::config::Auth::None => {
                warn!("Disabling authentication!");
                warn!("This is generally a BAD IDEA, as any request can freely read and modify the registry.");

                Ok(Self::None)
            }

            crate::config::Auth::AutoToken(token) => {
                info!("Using token list authentication");

                Ok(Self::TokenList(auto_token::AutoToken::new(token).await?))
            }
        }
    }

    pub fn auth_required(&self) -> bool {
        match self {
            Self::None => false,
            Self::TokenList(_) => true,
        }
    }

    // TODO: Implement more granular authorization
    pub fn authorize(&self, token: Option<&str>) -> Result<(), Error> {
        match self {
            Self::None => Ok(()),
            Self::TokenList(tokens) => tokens.authorize(token),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The provided token is invalid")]
    Forbidden,
    #[error("No authorization token was provided")]
    Unauthorized,

    #[error("IO error")]
    Io(io::Error),
    #[error("RNG error")]
    Random(getrandom::Error),
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
            Error::Io(_) | Error::Random(_) => ErrorResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                errors: Vec::new(),
            },
        }
    }
}
