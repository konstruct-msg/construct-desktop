# UI rules — construct-desktop

The interface is modelled on the macOS `Construct Desktop` target: sidebar of chats, conversation,
composer, the `Color.CT` palette, JetBrains Mono (`ui/fonts`). Files: `ui/index.html`, `ui/main.js`,
`ui/style.css`. The security-relevant UI invariants (textContent only, strict CSP, no protocol in
JS, attachment sniffing) are in `AGENTS.md`.

- No framework, no bundler, no third-party JS.
- Shortcuts use `KeyboardEvent.code`, not `key`: on a Cyrillic layout Ctrl+K reports as Ctrl+л, and
  Russian is the main audience's layout.
- Interface decision and its history: `~/Code/construct-docs/decisions/desktop-web-interface.md`.
  Until 2026-10-23 the window drew the TUI's screens as a character grid; the owner replaced that
  with this interface.
