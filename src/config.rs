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
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path =
            env::var("QUARTERMASTER_CONFIG_FILE").unwrap_or_else(|_| String::from("config.toml"));

        Ok(config::Config::builder()
            .add_source(config::Environment::with_prefix("QUARTERMASTER"))
            .add_source(
                config::File::new(&config_path, FileFormat::Toml).format(config::FileFormat::Toml),
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
            concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml"),
        );

        Config::load().unwrap();
    }
}
