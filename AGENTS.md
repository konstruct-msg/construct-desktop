# AGENTS.md — construct-desktop

Konstruct for **Linux and Windows**. macOS is covered by the `Construct Desktop` target in
construct-messenger; this repository is not for it.

Decision: `~/Code/construct-docs/decisions/desktop-is-the-tui-client-with-a-second-shell.md`. Read
it before working here.

## Where to start

The work plan, its stages and their gates, and the progress log are in the vault task
`~/Code/construct-docs/todo/218-desktop-linux-windows-plan.md`. Read it first. Then take the
first stage that is not done, and add a line to its «Ход» when you finish.

State on 2026-10-23:

- The window, input and resizing work, but the screen inside is still a placeholder.
- **The tree does not build** against the current `construct-core` (0.57). `construct-client` was
  last built against core 0.30 (2026-10-02). The first error is the `PlatformBridge` import in
  `construct-client/src/bridge.rs`, removed by core 0.54 (`31b3de5`, "remove the dead code").
  More errors will follow it.
- The client has never exchanged a message with iOS in both directions. What separates them is
  listed in the task.

## Checkouts and who owns them

This crate takes its dependencies by path, so the siblings must sit next to it:
`~/Code/construct-tui` (it contains `construct-client`), `~/Code/construct-core`, and
`~/Code/construct-veil`. All three use one toolchain, set in `rust-toolchain.toml`.

- Other agent sessions work in the `construct-core` and `construct-protos` working copies. Never
  switch their branches. Make changes there in a worktree, on a branch, through a PR. The owner
  merges PRs in `construct-core`.
- `construct-tui` is where `construct-client` changes. Make those changes in `construct-tui` as
  well, on their own branch and PR, and not as a copy in this repository.
- To build this repository from a worktree, place symlinks to the siblings next to it, so that
  `../../construct-tui` resolves.
- Turn the hooks on once per clone with `git config core.hooksPath .githooks`. The commit-msg hook
  strips `Co-authored-by` lines; that is intended.

## Working rules beyond this file

- **Testing against the server.** Before release, the test and production server are the same
  machine (owner, 2026-10-23). Registering test accounts and sending messages there for interop
  checks is allowed. Keep them recognisably test accounts.
- **This repository is public.** Never write server host names, IP addresses or other people's
  personal data here, in commit messages, or in PR descriptions. Keep the wording neutral.
- On the delivery or crypto path, every commit answers the three questions in
  `construct-messenger/AGENTS.md`:
  1. What does the server learn?
  2. What can a party withhold or substitute?
  3. Which trust boundary moves?
- Protocol decisions are the core's (see "No protocol here" below). Before writing one in Rust
  here or in `construct-client`, look in `construct-core/src/construct_core.udl` and the crate API
  for it.

## What this is

A Tauri 2 window whose whole content is a character grid. Ratatui draws the grid in Rust; xterm.js
in the webview only paints the frames and sends input back. The program itself, meaning the
account, keys, sessions, stream and storage, is `construct-client`, the crate in
`construct-tui/crates/construct-client`. This repository adds only the window.

```
src-tauri/src/main.rs    Tauri commands: attach / resize / key / text
src-tauri/src/grid.rs    ratatui → ANSI frames → Channel; the size comes from the webview, not a tty
src-tauri/src/keys.rs    DOM KeyboardEvent → crossterm KeyEvent (shortcuts by physical key)
src-tauri/src/screen.rs  placeholder screen; replaced by the konstruct screens
ui/                      index.html, main.js, style.css; vendored xterm.js, JetBrains Mono
```

Until 2026-10-02 this repository held a different program: a Tauri app with an HTML interface
and its own `engine.rs`, which re-decided what the core decides and stopped building when the
core dropped its `desktop` feature. It was deleted, not migrated. Only the name remains.

## Invariants

- **The webview sees frames, never the client.** No `ClientEvent`, contact list, key or token
  crosses the IPC boundary, only rendered text and input. A feature that seems to need the
  webview to know something is built as a screen in Rust.
- **No interface in JavaScript.** `ui/main.js` paints and forwards input. A panel, a dialog or a
  list written in HTML would be a third interface, which the decision exists to prevent.
- **No protocol here.** Anything the core or `construct-client` decides is asked of them. The
  previous `engine.rs` is what this rule is about.
- **Shortcuts go by the physical key.** On a Cyrillic layout the browser reports Ctrl+K as
  Ctrl+л. `keys.rs` takes the letter from `KeyboardEvent.code` when a modifier is held, and the
  copy/paste checks in `main.js` do the same. Russian is the main audience's layout.
- **The CSP stays strict** (`tauri.conf.json`): only local scripts, no remote origins, and no
  plugin permissions beyond `core:default`.
- **Vendored xterm.js is unmodified.** The versions are listed in `ui/vendor/xterm/VERSIONS`;
  update it by the route described there.

## Build & run

```bash
cargo run -p construct-desktop          # the window, without bundling
cargo test --workspace
npx @tauri-apps/cli@2 build             # deb/rpm/AppImage on Linux, nsis/msi on Windows
```

`construct-tui` and `construct-core` must be checked out next to this repository; the client
crate is a path dependency. The pre-push hook runs fmt and clippy (`-D warnings`).

`cargo run` works on macOS too, which is enough for development. The installers come only from
the target system: deb, rpm and AppImage from Linux, nsis and msi from Windows. Until CI builds
them (stage 5 of the plan), no Linux or Windows build has been run.

## Documentation & session notes

Docs live in `~/Code/construct-docs`, an Obsidian vault with flat domain folders. **The vault's
`AGENTS.md` is authoritative** for structure and writing rules.

After any session with architectural changes, design decisions, root-cause analysis or
non-obvious choices:

1. Write `sessions/YYYY-MM-DD-<topic>.md` (Context / What Changed / **Why** / Decisions / Open
   Questions). `## Why` with rejected alternatives is mandatory.
2. If it constrains future work, add or update `decisions/<slug>.md`.
3. Patch the affected spec in the **same** session.
4. Append one line to `~/Code/construct-docs/log.md`: `[YYYY-MM-DD HH:MM] note | <topic>`.

## Git workflow (branch + PR only)

**Never commit on `main`.** Every change goes on a topic branch cut from an up-to-date `main`
(`feat|fix|docs|chore|test/<topic>`) and lands through a GitHub pull request. Agents push and
open the PR only when asked.

`main` is what a release is built from, so it moves only by a reviewed merge. From 2026-09-11 to
2026-10-01 changes went straight to `main` across the construct-* repos — two people on the
project made a branch per change look like ceremony. That was reversed on purpose: the habit has
to be in place before there is a release for it to break.

A commit that landed on `main` by mistake and is not pushed moves off it with
`git branch <topic> && git reset --keep origin/main && git switch <topic>`. Pushed history is
never rewritten.
