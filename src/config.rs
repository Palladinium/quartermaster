use std::{
    env,
    fmt::{self, Debug, Formatter},
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::PathBuf,
};

use bytesize::ByteSize;
use config::FileFormat;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub server: Server,
    #[serde(default)]
    pub crates: Crates,
    pub auth: Auth,
    pub storage: Storage,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    /// NOTE: This is a `String` rather than a `Url` because `Url` always appends a trailing slash
    /// if no path is provided, which breaks in the simplest configuration because Cargo introduces an extra slash,
    /// resulting in invalid URLs like `http://foo.bar//api/v1/crates/new`
    pub root_url: String,
    #[serde(default = "default_bind")]
    pub bind: Vec<SocketAddr>,
}

fn default_bind() -> Vec<SocketAddr> {
    vec![
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8000)),
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 8000, 0, 0)),
    ]
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crates {
    #[serde(default = "default_max_publish_size")]
    pub max_publish_size: ByteSize,
}

impl Default for Crates {
    fn default() -> Self {
        Self {
            max_publish_size: default_max_publish_size(),
        }
    }
}

fn default_max_publish_size() -> ByteSize {
    ByteSize::mib(100)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Auth {
    None,
    Token(TokenAuth),
}

#[derive(Clone, Deserialize)]
pub struct TokenAuth {
    #[serde(deserialize_with = "hex::serde::deserialize")]
    pub token_hash: [u8; 64],
}

impl Debug for TokenAuth {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("TokenAuth")
            .field("token_hash", &"<REDACTED>")
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Storage {
    Local(LocalStorage),
    #[cfg(feature = "s3")]
    S3(Box<S3Storage>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalStorage {
    pub path: PathBuf,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "s3")]
pub struct S3Storage {
    pub bucket: String,
    pub region: String,

    #[serde(default)]
    pub auto_credentials: bool,

    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_security_token: Option<String>,
    pub aws_session_token: Option<String>,

    pub sts_session_name: Option<String>,
    pub sts_role_arn: Option<String>,
    pub sts_web_identity_token_file: Option<String>,

    #[serde(default)]
    pub use_profile_credentials: bool,
    pub profile_section: Option<String>,

    #[serde(default)]
    pub use_instance_credentials: bool,
}

impl Debug for S3Storage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("S3Storage")
            .field("bucket", &self.bucket)
            .field("region", &self.region)
            .finish_non_exhaustive()
    }
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path = env::var("QUARTERMASTER_CONFIG_FILE")
            .unwrap_or_else(|_| String::from("/etc/quartermaster/config.toml"));

        config::Config::builder()
            .add_source(
                config::File::new(&config_path, FileFormat::Toml)
                    .format(config::FileFormat::Toml)
                    .required(false),
            )
            .add_source(
                config::Environment::with_prefix("QUARTERMASTER")
                    .prefix_separator("__")
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("server.bind")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_file() {
        env::set_var(
            "QUARTERMASTER_CONFIG_FILE",
            concat!(env!("CARGO_MANIFEST_DIR"), "/examples/config.toml"),
        );
        env::set_var("QUARTERMASTER__SERVER__ROOT_URL", "http://some.other.url");

        let config = Config::load().unwrap();

        assert_eq!(config.server.root_url, "http://some.other.url");
    }
}
