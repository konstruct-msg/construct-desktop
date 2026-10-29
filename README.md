# construct-desktop

Konstruct for Linux and Windows: a Tauri window with an interface modelled on Konstruct for macOS.
The client layer (account, keys, sessions, messages) is
[`construct-client`](https://github.com/konstruct-msg/construct-client), running in Rust; the
interface is plain HTML, CSS and JavaScript in `ui/`.

Status: early.

## Install (Linux)

Download a package from [Releases](https://github.com/konstruct-msg/construct-desktop/releases):

- Ubuntu, Linux Mint, Debian: `sudo apt install ./Konstruct_<version>_amd64.deb`
- Fedora, openSUSE: `sudo dnf install ./Konstruct-<version>-1.x86_64.rpm`
- any distribution: the `.AppImage`, after `chmod +x`. It needs FUSE 2 (`libfuse2`, on Ubuntu
  24.04 and Mint 22 `libfuse2t64`).

The key to the local database is kept in the system key store (GNOME Keyring or KWallet via
Secret Service). Without one, the app asks for a passphrase instead.

## Build

```bash
cargo run -p construct-desktop
npx @tauri-apps/cli@2 build
```

Needs `construct-client`, `construct-core` and `construct-protos` checked out beside this repository.

## Releasing

1. A PR sets the new version in `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
2. After it is merged, tag main: `git tag v<version> origin/main && git push origin v<version>`.

The `release` workflow builds the Linux packages and the Windows installers (the same builds and
smoke tests as CI), checks that the tag matches the version, and publishes them with
`SHA256SUMS` as a GitHub release. The Windows installers are unsigned until there is a
code-signing certificate. The notes
list the commits of construct-client, construct-core and construct-protos the build used.

License: MPL-2.0. JetBrains Mono is OFL-1.1
(`ui/fonts/OFL.txt`).
