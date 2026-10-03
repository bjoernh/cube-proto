# cube-proto

Shared wire crates of the **LED Cube 2.0** system: the JSON control protocol spoken by the
`cubed` compositor daemon, the TOML config/manifest/schema loaders, preset storage, and an
async admin client. They are used by the daemon itself, its companion tools, and by the
app SDKs ([`cubekit`](https://github.com/bjoernh/cubekit) for Rust).

| Crate | Purpose |
|---|---|
| [`cube-proto`](crates/cube-proto) | Request / response / event types, error codes, frame header, param values |
| [`cube-config`](crates/cube-config) | Loaders for `system.toml`, app `manifest.toml` and `schema.toml` |
| [`cube-presets`](crates/cube-presets) | Preset TOML I/O with flock-protected atomic writes |
| [`cube-admin-client`](crates/cube-admin-client) | Async JSON-Lines client for the daemon's admin socket |

All four are published on crates.io. App authors normally don't depend on them directly —
use `cubekit` — but they are the reference for the wire format if you build your own client.

## Developing

```sh
cargo test --workspace
```

To use a local checkout from another workspace (for example while changing the protocol and
`cubekit` together), add to that workspace's `Cargo.toml`:

```toml
[patch.crates-io]
cube-proto = { path = "../cube-proto/crates/cube-proto" }
cube-config = { path = "../cube-proto/crates/cube-config" }
cube-presets = { path = "../cube-proto/crates/cube-presets" }
cube-admin-client = { path = "../cube-proto/crates/cube-admin-client" }
```

## Repository

The canonical repository is hosted on Forgejo (`git.64b.de`); this GitHub repository is a
mirror of its `main` branch and tags. License: MIT.
