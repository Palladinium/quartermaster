use std::fmt::{self, Debug, Formatter};

use sha2::{Digest, Sha512};
use subtle::ConstantTimeEq;

use crate::auth::Error;

pub struct Token {
    token_hash: [u8; 64],
}

impl Token {
    pub fn new(config: &crate::config::TokenAuth) -> Self {
        Self {
            token_hash: config.token_hash,
        }
    }

    pub fn authorize(&self, token: Option<&str>) -> Result<(), Error> {
        let token = token.ok_or(Error::Unauthorized)?;
        let token_hash = Sha512::digest(token);

        let token_hash_eq = bool::from(self.token_hash.ct_eq(token_hash.as_slice()));

        if token_hash_eq {
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
