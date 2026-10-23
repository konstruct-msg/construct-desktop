# construct-desktop

Konstruct for Linux and Windows. It is a Tauri window whose content is the same character grid as
the terminal client, drawn by Ratatui and painted by xterm.js. The client layer (account, keys,
sessions, messages) and the screens are
[`construct-client`](https://github.com/konstruct-msg/construct-client); the terminal client
draws the same screens.

Status: early. The window draws the Konstruct screens; installers are not built yet.

```bash
cargo run -p construct-desktop
npx @tauri-apps/cli@2 build
```

Needs `construct-client`, `construct-core` and `construct-protos` checked out beside this repository.

License: MPL-2.0. Vendored xterm.js is MIT (`ui/vendor/xterm/LICENSE`); JetBrains Mono is OFL-1.1
(`ui/fonts/OFL.txt`).
