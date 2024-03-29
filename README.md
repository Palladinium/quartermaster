# Quartermaster
t
[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/quartermaster.svg
[crates-url]: https://crates.io/crates/quartermaster

A dead-simple [Cargo Alternate Registry](https://doc.rust-lang.org/cargo/reference/registries.html) suitable for private registries.

## Why?

If you are tired of using git dependencies for your private crates and just want to host your own cargo registry now, Quartermaster is for you.

### Features

- Local filesystem or S3-based backing storage - No DB required
- Extremely simple token-based auth

### Non-features

If you need any of these features, you're probably better off looking at alternatives.

- A Web UI
- Support for Rust versions before 1.74
- Git index protocol (only sparse index supported)

### Limitations

Quartermaster is still very early in development, and these are features which are planned but I haven't gotten around to implementing yet. Contributions are welcome and appreciated!

- **No HTTPS/SSL**: at the moment, Quartermaster is HTTP only. **Do not** expose Quartermaster to the open Internet. **Do** put it behind a correctly configured reverse proxy which handles SSL termination like [NGINX](http://nginx.org/), or a VPN like [https://www.wireguard.com/](Wireguard) or [https://openvpn.net/](OpenVPN), or do both!
- Granular auth: Currently, a valid token has full read/write access to the repository.
- User/owner endpoints: Currently, tokens are global, and all crates are owned by nobody. The various `owner` endpoints are not implemented.
- More varied and robust auth methods (e.g. OpenID). I have no need for them yet.
- Cross-platform support: While in theory nothing stops Quartermaster from running on other platforms like Windows, MacOS or BSDs, I have only tested it on x86_64 Linux and the default values for the configuration reflect this.
- CLI management utility: Instead of a Web UI, I'm planning to add a CLI utility to perform registry maintenance tasks which cannot be performed through the Cargo API (e.g. fully removing crates, managing auth)

## Installation

### Docker

If you prefer running Quartermaster in a container, an [image](https://hub.docker.com/r/palladinium/quartermaster) is available on DockerHub. The preferred method of configuration when using Docker is through environment variables, but config files are still supported.

```shell
docker pull palladinium/quartermaster
```

### Cargo

You can compile Quartermaster yourself with cargo.

```shell
cargo install --frozen quartermaster
```

## Configuration

Quartermaster uses the excellent [config](https://github.com/mehcode/config-rs) crate to support configuration through either a `toml` config file or environment variables, or a combination of both. The matching environment variable name is constructed by using double underscores as a separator, for example a configuration option `foo.bar_baz.boz` can be equivalently set through the environment variable `QUARTERMASTER__FOO__BAR_BAZ__BOZ`. Arrays of values can be defined by comma-separating individual values. Environment variables will override values set in the config file.

By default, Quartermaster expects to be run as a system service and will read `/etc/quartermaster/config.toml`, but this can be overridden by setting the environment variable `QUARTERMASTER_CONFIG_FILE`.

Take care when using config files to set secret values such as auth tokens and S3 credentials. The config file should have restrictive permissions to avoid exposing the secrets to other users on the system.

See the [example configuration](examples/config.toml) for more documentation on
the individual options.

## License

This project and all contributions to it are licensed under the GPL General Public License v3.
