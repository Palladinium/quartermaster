use std::fmt::{self, Debug, Formatter};

use sha2::{Digest, Sha512};

use crate::auth::Error;

pub struct Token {
    token_hash: [u8; 64],
}

impl Token {
    pub fn new(config: &crate::config::TokenAuth) -> Self {
        Self {
            token_hash: config.token_hash.clone(),
        }
    }

    pub fn authorize(&self, token: Option<&str>) -> Result<(), Error> {
        let token = token.ok_or(Error::Unauthorized)?;
        let token_hash = Sha512::digest(token);

        if self.token_hash == token_hash.as_slice() {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Token")
            .field("token_hash", &"<REDACTED>")
            .finish()
    }
}
