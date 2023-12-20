# Quartermaster

A dead-simple, minimal [Cargo Alternate Registry](https://doc.rust-lang.org/cargo/reference/registries.html) suitable for private registries.

# Why?

If you are tired of using git dependencies for your private crates and just want your own cargo registry now, Quartermaster is for you.

## Features

- Local filesystem or S3-based backing storage
- Extremely simple token-based auth

## Non-features

If you need any of these features, you're probably better off looking at alternatives.

- A Web UI
- Support for Rust versions before 1.74
- Git index protocol (only sparse index supported)

## Limitations

These are features which I haven't gotten around to implementing yet. Contributions are welcome and appreciated!

- **HTTPS/SSL**: at the moment, Quartermaster is HTTP only. **Do not** expose Quartermaster to the open Internet. Do put it behind a reverse proxy which handles SSL termination like [NGINX](http://nginx.org/), or a VPN like [https://www.wireguard.com/](Wireguard) or [https://openvpn.net/](OpenVPN), or do both!
- More granular auth. Currently, any valid token has full read/write access to the repository.
- More auth methods, e.g. OpenID. I have no need for them yet.
- Better cross-platform support. While in theory nothing stops Quartermaster from running on other platforms like Windows, MacOS or BSDs, I have only tested it on Linux. Feedback is welcome!

# Installation

## Docker

## Arch Linux

Quartermaster has a [package](http://todo) on the Arch User Repository. Install it with your preferred AUR wrapper or [build it yourself](https://wiki.archlinux.org/title/Arch_User_Repository#Installing_and_upgrading_packages). For example, using yay:

```shell
yay quartermaster
```

Configure Quartermaster by editing `/etc/quartermaster/config.toml` and `/etc/quartermaster/secrets.toml`, and then start/enable the systemd service:

```shell
systemctl enable quartermaster.service
```

## Cargo

You can compile Quartermaster yourself with cargo.

```shell
cargo install --frozen quartermaster
```

## Docker

If you prefer running Quartermaster in a container, an [image](http://todo) is available on DockerHub. The preferred method of configuration when using Docker is through environment variables, but config files are still supported.

```shell
docker pull palladinium/quartermaster
```

# Configuration

Quartermaster uses the excellent [config](https://github.com/mehcode/config-rs) crate to support configuration through either a `toml` config file or environment variables, or a combination of both. The matching environment variable name is constructed by using double underscores as a separator, for example a configuration option `foo.bar_baz.boz` can be equivalently set through the environment variable `QUARTERMASTER__FOO__BAR_BAZ__BOZ`. Arrays of values can be defined by comma-separating individual values. Environment variables will override values set in the config file.

By default, Quartermaster expects to be run as a system service and will read `/etc/quartermaster/config.toml`, but this can be overridden by setting the environment variable `QUARTERMASTER_CONFIG_FILE`.

Take care when using config files to set secret values such as auth tokens and S3 credentials. The config file should have restrictive permissions to avoid exposing the secrets to other users on the system.

See the [example configuration](examples/config.toml) for more documentation on the individual options.

# License

This project and all contributions to it are dual-licensed under the Apache-2.0 and MIT licenses.