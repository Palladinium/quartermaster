use axum::http::StatusCode;
use tracing::{info, warn};

use crate::error::ErrorResponse;

pub mod token_list;

pub enum Auth {
    None,
    TokenList(token_list::TokenList),
}

impl Auth {
    pub fn new(config: &crate::config::Auth) -> Self {
        match config {
            crate::config::Auth::None => {
                warn!("Disabling authentication!");
                warn!("This is generally a BAD IDEA, as any request can freely read and modify the registry.");

                Self::None
            }

            crate::config::Auth::TokenList(tokens) => {
                info!("Using token list authentication");

                Self::TokenList(token_list::TokenList::new(tokens))
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
