# Quartermaster

A dead-simple, minimal [Cargo Alternate Registry](https://doc.rust-lang.org/cargo/reference/registries.html) suitable for private registries.

# Why?

If you are tired of using git dependencies for your private crates and just want your own cargo registry now, Quartermaster is for you.

# Features

- Local or S3-based backing storage
- Extremely simple token-based auth

# Non-features

If you need any of these features, you're probably better off looking at alternatives.

- A Web UI
- Support for Rust versions before 1.74
- Git index protocol (sparse index is supported)

# Limitations

These are features which I haven't gotten around to implementing yet. Contributions are welcome!

- **HTTPS/SSL**: at the moment, Quartermaster is HTTP only. **Do not** expose Quartermaster to the open Internet. Do put it behind reverse proxy which handles SSL termination like [NGINX](http://nginx.org/), or a VPN like [https://www.wireguard.com/](Wireguard) or [https://openvpn.net/](OpenVPN), or do both!
- More granular auth. Currently, any valid token has full read/write access to the repository.
- More auth methods, e.g. OpenID. I have no need for them yet.
- Better cross-platform support. While in theory nothing stops Quartermaster from running on other platforms like Windows, MacOS or BSDs, I have only tested it on Linux. Feedback is welcome!

# License

This project and all contributions to it are dual-licensed under the Apache-2.0 and MIT licenses.
