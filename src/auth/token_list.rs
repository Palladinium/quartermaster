use std::{
    collections::HashSet,
    fmt::{self, Debug, Formatter},
};

use tracing::warn;

use crate::auth::Error;

pub struct TokenList {
    tokens: HashSet<String>,
}

impl TokenList {
    pub fn new(config: &crate::config::TokenListAuth) -> Self {
        if config.tokens.is_empty() {
            warn!("No tokens set. No requests will be possible.");
        }

        Self {
            tokens: config.tokens.iter().cloned().collect(),
        }
    }

    pub fn authorize(&self, token: Option<&str>) -> Result<(), Error> {
        let token = token.ok_or(Error::Unauthorized)?;

        if self.tokens.contains(token) {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
}

impl Debug for TokenList {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("TokenList")
            .field("tokens", &"<REDACTED>")
            .finish()
    }
}
