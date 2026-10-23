# construct-desktop

Konstruct for Linux and Windows: a Tauri window with an interface modelled on Konstruct for macOS.
The client layer (account, keys, sessions, messages) is
[`construct-client`](https://github.com/konstruct-msg/construct-client), running in Rust; the
interface is plain HTML, CSS and JavaScript in `ui/`.

Status: early. CI builds deb, rpm and AppImage on every change (Actions → `linux` → artifact).

```bash
cargo run -p construct-desktop
npx @tauri-apps/cli@2 build
```

Needs `construct-client`, `construct-core` and `construct-protos` checked out beside this repository.

License: MPL-2.0. JetBrains Mono is OFL-1.1
(`ui/fonts/OFL.txt`).
