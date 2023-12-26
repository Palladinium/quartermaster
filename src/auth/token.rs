use std::{
    fmt::{self, Debug, Formatter},
};

use crate::auth::Error;

pub struct Token {
    token: String,
}

impl Token {
    pub fn new(config: &crate::config::TokenAuth) -> Self {
        Self {
            token: config.token.clone(),
        }
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

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Token")
            .field("token", &"<REDACTED>")
            .finish()
    }
}
