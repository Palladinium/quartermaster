use axum::http::StatusCode;
use tracing::{info, warn};

use crate::error::ErrorResponse;

pub mod token;

pub enum Auth {
    None,
    Token(token::Token),
}

impl Auth {
    pub async fn new(config: &crate::config::Auth) -> Result<Self, Error> {
        match config {
            crate::config::Auth::None => {
                warn!("Disabling authentication!");
                warn!("This is generally a BAD IDEA, as any request can freely read and modify the registry.");

                Ok(Self::None)
            }

            crate::config::Auth::Token(token) => {
                info!("Using token authentication");

                Ok(Self::Token(token::Token::new(token)))
            }
        }
    }

    pub fn auth_required(&self) -> bool {
        match self {
            Self::None => false,
            Self::Token(_) => true,
        }
    }

    // TODO: Implement more granular authorization
    pub fn authorize(&self, token: Option<&str>) -> Result<(), Error> {
        match self {
            Self::None => Ok(()),
            Self::Token(token_auth) => token_auth.authorize(token),
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
