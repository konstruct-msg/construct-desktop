# AGENTS.md — construct-desktop

Context for AI agents working in this repository.

---

## What is construct-desktop?

Desktop client for Construct Messenger built as a Tauri 2 application with a Rust backend.
The checked-in frontend is a minimal static HTML/JavaScript UI in `dist/` — no React/Vue/Svelte
source tree is present in this repository right now.

This app currently uses `construct-core` directly as a Rust crate (`features = ["desktop"]`)
for crypto/session orchestration and `tonic` gRPC clients for server communication.

---

## Architecture

```
dist/index.html            — current frontend (plain HTML/JS, no framework source checked in)
src/main.rs                — Tauri bootstrap, command registration, shared engine state
src/ui.rs                  — Tauri commands exposed to the frontend via `invoke(...)`
src/engine.rs              — auth flow, construct-core integration, chat state, receive loop
src/grpc.rs                — tonic clients for AuthService / KeyService / MessagingService
src/storage.rs             — secure local persistence for tokens and state
src/wire.rs                — encrypted wire payload encode/decode helpers
src-tauri/tauri.conf.json  — Tauri app/window/build configuration
```

### Tauri boundary

Frontend code should stay thin. The browser side calls Tauri commands in `ui.rs`, while
networking, crypto, storage, and durable state stay in Rust.

### Frontend status

The current frontend is plain static HTML/JS served from `dist/`. If a framework is added later,
document it here and keep command/API ownership on the Rust side.

---

## Build & Run

```bash
cargo tauri dev                  # run desktop app in dev mode
cargo tauri build                # build distributable app
cargo build                      # build Rust backend only
cargo test                       # run tests
```

---

## Key conventions

- Tauri commands in `ui.rs` are the frontend/backend API surface — changing them is a breaking change for the UI
- Keep `construct-core` usage inside `engine.rs` and supporting modules, not in UI-facing command handlers
- Rust backend is the source of truth for auth, session, and message state
- Keep frontend assets in `dist/` lightweight; do not duplicate protocol logic in JavaScript

---
---

## Shared Construct Docs Workflow

These instructions apply to GitHub Copilot, Codex, OpenCode, and similar coding agents.

### Division of labour — read this first

| Role | Tool | Responsibility |
|------|------|----------------|
| **Coding agent** (you) | Copilot / Codex | Write code + drop raw session notes into `wiki/sessions/` and `wiki/decisions/`. That is all. |
| **Wiki pipeline** | `obsidian-llm-wiki-local` (olw) | Reads `raw/`, synthesizes concepts, creates/updates wiki articles, generates cross-links. |
| **Developer** | Human + Obsidian | Reviews wiki draft articles, approves/rejects. Curates `raw/`. |

**Your job is code.** olw handles article synthesis. Write plain-markdown session notes; let the pipeline do the rest.

### Shared knowledge base

- Vault: `/Users/maximeliseyev/Code/construct-docs`
- `raw/` — source corpus. Do **not** rewrite or reorganize.
- `wiki/` — canonical curated knowledge base. **Read** from here before architectural work.
- `wiki/.drafts/` — **reserved for olw**. Never write here manually.
- `wiki/sessions/` — where coding agents write session notes.
- `wiki/decisions/` — where coding agents write long-lived decision records.

### Where to save durable reasoning

After any session involving architectural changes, design decisions, API changes, or non-obvious implementation choices:

1. **Always** create or update `wiki/sessions/YYYY-MM-DD-<topic>.md`.
2. **Always** fill in `# Why` — reasoning, alternatives considered, why rejected. Most important section.
3. If the decision constrains future work, also create `wiki/decisions/<topic>.md`.
4. Session notes: plain markdown, **no YAML frontmatter, no `[[wikilinks]]`** — olw adds those.

Required note sections: `# Context`, `# What Changed`, `# Why`, `# Intended Outcome`, `# Decisions`, `# Open Questions`

### Operational logging

Append a one-line entry to `wiki/log.md` after writing a note.
Format: `[YYYY-MM-DD HH:MM] note | <topic>`

