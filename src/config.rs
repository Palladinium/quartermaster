use std::{
    env,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::PathBuf,
};

use config::FileFormat;
use serde::Deserialize;
use url::Url;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub server: Server,
    pub auth: Auth,
    pub storage: Storage,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    pub root_url: Url,
    #[serde(default = "default_bind")]
    pub bind: Vec<SocketAddr>,
}

fn default_bind() -> Vec<SocketAddr> {
    vec![
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 80)),
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 80, 0, 0)),
    ]
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Auth {
    None,
    EnvTokens,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Storage {
    Local(LocalStorage),
    #[cfg(feature = "s3")]
    S3(S3Storage),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalStorage {
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "s3")]
pub struct S3Storage {
    pub bucket: String,
    pub region: String,

    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_security_token: Option<String>,
    pub aws_session_token: Option<String>,

    pub sts_session_name: Option<String>,
    pub sts_role_arn: Option<String>,
    pub sts_web_identity_token_file: Option<String>,

    pub use_profile_credentials: bool,
    pub profile_section: Option<String>,

    pub use_instance_credentials: bool,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path = env::var("QUARTERMASTER_CONFIG_FILE")
            .unwrap_or_else(|_| String::from("/etc/quartermaster/config.toml"));

        Ok(config::Config::builder()
            .add_source(
                config::File::new(&config_path, FileFormat::Toml)
                    .format(config::FileFormat::Toml)
                    .required(false),
            )
            .add_source(
                config::Environment::with_prefix("QUARTERMASTER")
                    .prefix_separator("__")
                    .separator("__"),
            )
            .build()?
            .try_deserialize()?)
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
        env::set_var("QUARTERMASTER__SERVER__ROOT_URL", "http://some.other.url/");

        let config = Config::load().unwrap();

        assert_eq!(
            config.server.root_url,
            Url::parse("http://some.other.url/").unwrap()
        );
    }
}
