# AGENTS.md — construct-desktop

Konstruct for **Linux and Windows**: a Tauri 2 window with an HTML/CSS interface modelled on the macOS `Construct Desktop` target (macOS itself is not this repo). The program — account, keys, sessions, stream, storage — is `construct-client`; the webview draws typed view events and sends back what the person did.

If a rule here and your plan disagree, the rule wins until a `decisions/` note says otherwise.

## Read first

| Area | Read |
|---|---|
| Why these rules exist; checklist before any protocol feature | `~/Code/construct-docs/decisions/rules-from-the-first-year.md` |
| Work plan, stages, gates, progress («Ход») — take the first stage not done | `~/Code/construct-docs/todo/218-desktop-linux-windows-plan.md` |
| The interface | `~/Code/construct-docs/decisions/desktop-web-interface.md` |
| Client layer shared with the TUI | `~/Code/construct-docs/decisions/desktop-is-the-tui-client-with-a-second-shell.md` |
| UI rules | `docs/UI_RULES.md` |
| Three questions, session lifecycle | `construct-messenger/AGENTS.md` |
| Everything else | `~/Code/construct-docs` (vault; its own `AGENTS.md` is authoritative) |

## The Zen of Konstruct

```
Someone has solved it before. Read them first.
The core decides; the app executes.
A message names the state it means. Time is not an address.
Reproduce, then fix.
One meaning, one carrier.
Bytes stay bytes.
Measure, then build.
The effect is the proof, not the exit code.
A test that cannot fail is worse than no test.
Boring before new.
Temporary is permanent, unless something checks it.
What nothing reads is not sent.
Silence is the worst failure: a message that never appears raises no error.
```

Canonical source: `~/Code/construct-docs/decisions/rules-from-the-first-year.md` — change it there first.

## Rules

1. **Prior art first.** Before any protocol mechanism, write down how Signal / MLS / Matrix solve it and why we deviate. The reference client is iOS.
2. **The core decides, this app executes.** Nothing two clients must compute identically, no session or ratchet decision, no lifecycle phase lives here or in JavaScript. Before writing one in Rust here or in `construct-client`, look in `construct-core/src/construct_core.udl`. The old `engine.rs` re-decided what the core decides; it was deleted.
3. **A control message names the state it refers to.** A timer, cooldown, window or retry budget in a fix means the message has no address.
4. **Reproduce before fixing**, on a stand that fails deterministically; a fix counts when it turns green.
5. **One meaning, one carrier.** Read `decisions/one-meaning-two-carriers.md` before adding a field, counter or timeout; the ids that cross IPC are the client's, not re-derived in JS.
6. **Binary stays binary**; text encodings only where a text boundary demands them (`data:` URIs for the page, QR).
7. **Measure, then build.**
8. **Verify the effect, not the exit code.**
9. **A test must be able to fail**; mutation-check new tests.
10. **Depth before breadth.** Linux first (stage by stage in the plan); Windows is a later stage.
11. **A temporary exception has an end date and a check.**
12. **Branch and PR, always** (see Git).

A change on the delivery or crypto path answers three questions in its commit message: what does the server learn that it did not before; what can a party withhold or substitute that it could not before; which trust boundary moves, and which way.

## Invariants of this repo

- **The webview sees what is on screen, never the client's secrets.** `ViewEvent` carries names, text, ids, QR images; no key, token, certificate, session or protocol state crosses IPC, and every intent is something the client checks. One exception: the recovery phrase, sent only when asked, dropped when its dialog closes; the copy is checked in `construct-client`.
- **Nothing from the network becomes markup.** `textContent` for names and text; `innerHTML` only for the page's own literals.
- **No protocol in JavaScript.** `ui/main.js` draws and forwards intents.
- **No framework, no bundler, no third-party JS** — small enough to read whole, simple CSP.
- **The CSP stays strict** (`tauri.conf.json`): local scripts only, no remote origins, images and sounds local or `data:`, no plugin permissions beyond `core:default`.
- **An attachment is shown as what its bytes are**, not what the sender declared: Rust makes a `data:` URI only when the first bytes are a picture or a sound (`bridge.rs` `sniff`). Anything else is saved to downloads, never over an existing file, and nothing from the network is opened by the system.
- **Shortcuts go by the physical key** (`KeyboardEvent.code`): Ctrl+K arrives as Ctrl+л on a Cyrillic layout.
- **This repository is public.** No server host names, IPs or other people's personal data in files, commits or PRs.
- **Testing against the server:** before release test and production are one machine; recognisable test accounts and test messages for interop are allowed (owner, 2026-10-23).

Layout: `src-tauri/src/main.rs` (Tauri commands, one per intent), `src-tauri/src/bridge.rs` (`ClientEvent` → `ViewEvent`, intent → `ClientCommand`, the model a reloaded page is redrawn from, QR as SVG), `ui/` (index.html, main.js, style.css; JetBrains Mono).

### Checkouts

Dependencies are by path; these must sit next to this repo: `~/Code/construct-client` (client layer and screens — they change there, on their own branch and PR, never as a copy here), `~/Code/construct-core`, `~/Code/construct-protos`. One toolchain, `rust-toolchain.toml`. Other sessions work in the core and protos working copies: never switch their branches; change them in a worktree, on a branch, through a PR (the owner merges core PRs). To build from a worktree, symlink the siblings so `../../construct-client` resolves. Once per clone: `git config core.hooksPath .githooks` (the commit-msg hook strips `Co-authored-by` lines; intended; pre-push runs fmt and clippy `-D warnings`).

### Build

```bash
cargo run -p construct-desktop   # the window, without bundling (works on macOS too)
cargo test --workspace
npx @tauri-apps/cli@2 build      # deb/rpm/AppImage on Linux, nsis/msi on Windows
```

Installers come only from the target system. The `linux` workflow builds the Linux ones on every change; a tag `vX.Y.Z` on main publishes a GitHub release via `release.yml` (README, "Releasing").

## Git and process

- **Never commit on `main`.** Topic branch (`feat|fix|docs|chore|test/<topic>`) from an up-to-date `main`, PR into `main`. Conventional Commits. Agents push and open the PR only when asked.
- Check branch and `git status` before every commit; never `checkout -- .`, `reset --hard` or `stash` in a shared tree; never rewrite pushed history. A commit made on `main` by mistake and not pushed: `git branch <topic> && git reset --keep origin/main && git switch <topic>`.
- Docs live in `~/Code/construct-docs` (its `AGENTS.md` is authoritative). After work: a line in the plan's «Ход»; a root cause or rejected alternatives to `sessions/` (with `## Why`); a constraint to `decisions/`; one line to `log.md`.
