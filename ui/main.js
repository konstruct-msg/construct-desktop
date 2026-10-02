// Paints the grid Rust draws and sends input back. Nothing else lives here: no state, no
// protocol, no client — see AGENTS.md.

const { invoke, Channel } = window.__TAURI__.core;

const FONT = '"JetBrains Mono", monospace';

async function start() {
  // xterm measures the cell once, at open; with the font still loading it measures a fallback.
  await document.fonts.load(`15px ${FONT}`);

  const term = new Terminal({
    fontFamily: FONT,
    fontSize: 15,
    // Box drawing and block elements drawn to the cell, not taken from the font: no gaps.
    customGlyphs: true,
    cursorBlink: false,
    // Colours come from the frames; these only cover the instant before the first one.
    theme: { background: "#062038", foreground: "#eaf2f4" },
    screenReaderMode: false,
    allowProposedApi: false,
  });

  const fit = new FitAddon.FitAddon();
  term.loadAddon(fit);
  term.open(document.getElementById("grid"));

  try {
    const webgl = new WebglAddon.WebglAddon();
    // A lost context (driver reset, WebKitGTK without GPU) falls back to the DOM renderer.
    webgl.onContextLoss(() => webgl.dispose());
    term.loadAddon(webgl);
  } catch (e) {
    console.warn("WebGL unavailable, DOM renderer", e);
  }

  fit.fit();

  term.attachCustomKeyEventHandler((ev) => {
    if (ev.type !== "keydown") return true;
    // Not handled and not prevented: the browser pastes into xterm's textarea, xterm emits the
    // text. Letting xterm handle the key instead would send ^V and swallow the paste.
    if (isPaste(ev)) return false;
    if (isCopy(ev)) {
      if (term.hasSelection()) navigator.clipboard.writeText(term.getSelection());
      ev.preventDefault();
      return false;
    }
    if (!isForRust(ev)) return true; // printable: xterm turns it into text (and IME works)
    ev.preventDefault();
    invoke("key", {
      key: { key: ev.key, code: ev.code, ctrl: ev.ctrlKey, alt: ev.altKey, shift: ev.shiftKey, meta: ev.metaKey },
    });
    return false;
  });

  term.onData((text) => invoke("text", { text }));
  term.onResize(({ cols, rows }) => invoke("resize", { cols, rows }));
  window.addEventListener("resize", () => fit.fit());

  const frames = new Channel();
  frames.onmessage = (frame) => term.write(frame);
  await invoke("attach", { cols: term.cols, rows: term.rows, onFrame: frames });
  term.focus();
}

// Named keys, and anything held with Ctrl, Alt or Super, go to Rust as keys. AltGr reports
// Ctrl+Alt on Windows while typing ordinary characters (@, € on many layouts), so it is text.
function isForRust(ev) {
  if (ev.isComposing || ev.keyCode === 229) return false;
  if (ev.getModifierState("AltGraph")) return false;
  if (ev.key.length > 1) return true;
  return ev.ctrlKey || ev.altKey || ev.metaKey;
}

// Copy and paste stay with the window, as in a terminal: Ctrl+Shift+C/V, and Ctrl+V. By the
// physical key, so they work on any layout.
function isCopy(ev) {
  return ev.ctrlKey && ev.shiftKey && ev.code === "KeyC";
}

function isPaste(ev) {
  return ev.ctrlKey && ev.code === "KeyV";
}

start();
