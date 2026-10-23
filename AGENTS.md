# AGENTS.md — construct-desktop

Konstruct for **Linux and Windows**. macOS is covered by the `Construct Desktop` target in
construct-messenger; this repository is not for it.

Decisions: `~/Code/construct-docs/decisions/desktop-web-interface.md` (the interface, since
2026-10-23) and `desktop-is-the-tui-client-with-a-second-shell.md` (the client layer shared with
the TUI). Read them before working here.

## Where to start

The work plan, its stages and their gates, and the progress log are in the vault task
`~/Code/construct-docs/todo/218-desktop-linux-windows-plan.md`. Read it first. Then take the
first stage that is not done, and add a line to its «Ход» when you finish.

State on 2026-10-23 (evening):

- The window draws the real screens: `construct-screens` `App`, the same one the terminal shell
  draws (stage 3 of the plan, first step).
- `construct-client` builds against core 0.57 and talks sealed through the server (TUI↔TUI checked
  both ways after restarts). The run with iOS is not done yet.

## Checkouts and who owns them

This crate takes its dependencies by path, so the siblings must sit next to it:
`~/Code/construct-client` (the client layer and the screens), `~/Code/construct-core` and
`~/Code/construct-protos`. All use one toolchain, set in `rust-toolchain.toml`.

- Other agent sessions work in the `construct-core` and `construct-protos` working copies. Never
  switch their branches. Make changes there in a worktree, on a branch, through a PR. The owner
  merges PRs in `construct-core`.
- `construct-client` (repository) is where the client layer and the screens change, on their own
  branch and PR, never as a copy in this repository. `construct-tui` is only the terminal shell.
- To build this repository from a worktree, place symlinks to the siblings next to it, so that
  `../../construct-client` resolves.
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

A Tauri 2 window with an HTML/CSS interface modelled on the macOS `Construct Desktop` target:
sidebar of chats, conversation, composer, the `Color.CT` palette. The program itself — account,
keys, sessions, stream, storage — is `construct-client` (construct-client repository), running in
Rust. The webview draws typed view events and sends back what the person did.

```
src-tauri/src/main.rs    Tauri commands: attach, and one per intent (send_message, open_chat, …)
src-tauri/src/bridge.rs  construct-client ⇄ webview: ClientEvent → ViewEvent, intent → ClientCommand;
                         the model a reloaded page is redrawn from; QR codes rendered as SVG
ui/                      index.html, main.js, style.css (no framework, no bundler); JetBrains Mono
```

Until 2026-10-02 this repository held a different program: a Tauri app with an HTML interface
and its own `engine.rs`, which re-decided what the core decides and stopped building when the
core dropped its `desktop` feature. It was deleted, not migrated. From 2026-10-02 to 2026-10-23
the window drew the TUI's screens as a character grid; the owner replaced that with this interface.

## Invariants

- **The webview sees what is on screen, never the client's secrets.** `ViewEvent` carries names,
  message text, ids, QR images. No key, token, certificate, session or protocol state crosses the
  IPC boundary, and every intent from the webview is something the client checks.
- **Nothing from the network becomes markup.** Names and message text go into the page with
  `textContent`. `innerHTML` is used only with the page's own literals (two SVG icons).
- **No protocol in JavaScript.** `ui/main.js` draws and forwards intents; anything the core or
  `construct-client` decides is asked of them. The previous `engine.rs` is what this rule is about.
- **No framework, no bundler, no third-party JS.** The interface stays small enough to read whole,
  and the CSP stays simple.
- **Shortcuts go by the physical key** (`KeyboardEvent.code`). On a Cyrillic layout the browser
  reports Ctrl+K as Ctrl+л. Russian is the main audience's layout.
- **The CSP stays strict** (`tauri.conf.json`): only local scripts, no remote origins, images only
  local or `data:`, and no plugin permissions beyond `core:default`.

## Build & run

```bash
cargo run -p construct-desktop          # the window, without bundling
cargo test --workspace
npx @tauri-apps/cli@2 build             # deb/rpm/AppImage on Linux, nsis/msi on Windows
```

`construct-client`, `construct-core` and `construct-protos` must be checked out next to this
repository; the client and screens crates are path dependencies. The pre-push hook runs fmt and clippy (`-D warnings`).

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
