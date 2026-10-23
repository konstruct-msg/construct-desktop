// The interface. It draws the view events Rust sends and sends back what the person did; it
// holds no key, token or protocol state and decides nothing the client decides
// (decisions/desktop-web-interface.md).
//
// Everything that came from the network — names, message text — goes into the page with
// textContent, never as markup. The page builds markup only from its own literals.

"use strict";

const { invoke, Channel } = window.__TAURI__.core;

// ── Words ───────────────────────────────────────────────────────────────────

const WORDS = {
  en: {
    appTitle: "KONSTRUCT", starting: "Opening…",
    tagline: "The internet is a space of free thought",
    aliasPlaceholder: "Choose a public alias (optional)", createIdentity: "Create identity",
    linkThisDevice: "Link this device to my phone",
    registeringHint: "The first launch solves a proof of work; it takes a minute or two.",
    linkTitle: "Link this device",
    linkHint: "On your phone: Konstruct → Settings → Devices → scan this code.",
    making: "Making a code…", newCode: "New code", back: "Back",
    unlockHint: "Enter the passphrase that protects this device's data.", unlock: "Unlock",
    setPassphraseHint: "This system has no key store. Choose a passphrase to protect this device's data.",
    setPassphrase: "Protect", chats: "Chats", search: "Search", addContact: "Add contact…",
    settings: "Settings", offline: "Offline", online: "Connected",
    selectChat: "Select a chat", selectChatHint: "Choose a conversation from the sidebar to start messaging.",
    profile: "Profile", messagePlaceholder: "Message…", send: "Send",
    pasteInvite: "Paste invite link", myQr: "My QR", findPlaceholder: "Invite link or username",
    find: "Find", add: "Add", noResults: "Nobody found",
    myQrHint: "Show this to someone so they can scan it.", copyLink: "Copy link", copied: "Copied",
    close: "Close", safetyNumbers: "Safety numbers",
    safetyHint: "Compare these numbers with {name} in person. If they match, nobody is in between.",
    deleteChat: "Delete chat", confirmDelete: "Delete? Click again",
    account: "Account", accountId: "Account", deviceId: "Device", server: "Server",
    appearance: "Appearance", themeSystem: "System", themeDark: "Dark", themeLight: "Light",
    signOut: "Sign out", confirmSignOut: "Sign out and erase this device? Click again",
    today: "Today", yesterday: "Yesterday",
  },
  ru: {
    appTitle: "КОНСТРУКТ", starting: "Открываю…",
    tagline: "Интернет — это пространство свободы выражения мысли",
    aliasPlaceholder: "Публичный псевдоним (необязательно)", createIdentity: "Создать идентичность",
    linkThisDevice: "Привязать к телефону",
    registeringHint: "При первом запуске решается задача доказательства работы, это минута-две.",
    linkTitle: "Привязать это устройство",
    linkHint: "На телефоне: Konstruct → Настройки → Устройства → отсканируйте этот код.",
    making: "Делаю код…", newCode: "Новый код", back: "Назад",
    unlockHint: "Введите пароль, которым защищены данные этого устройства.", unlock: "Открыть",
    setPassphraseHint: "В системе нет хранилища ключей. Задайте пароль для данных этого устройства.",
    setPassphrase: "Защитить", chats: "Чаты", search: "Поиск", addContact: "Добавить контакт…",
    settings: "Настройки", offline: "Нет связи", online: "На связи",
    selectChat: "Выберите чат", selectChatHint: "Выберите разговор на боковой панели, чтобы начать общение.",
    profile: "Профиль", messagePlaceholder: "Сообщение…", send: "Отправить",
    pasteInvite: "Вставить ссылку-приглашение", myQr: "Мой QR",
    findPlaceholder: "Ссылка-приглашение или имя", find: "Найти", add: "Добавить",
    noResults: "Никого не нашлось",
    myQrHint: "Покажите это человеку, чтобы он мог отсканировать.", copyLink: "Скопировать ссылку",
    copied: "Скопировано", close: "Закрыть", safetyNumbers: "Защитные числа",
    safetyHint: "Сравните эти числа с {name} при встрече. Если совпадают, посредника между вами нет.",
    deleteChat: "Удалить чат", confirmDelete: "Удалить? Нажмите ещё раз",
    account: "Аккаунт", accountId: "Аккаунт", deviceId: "Устройство", server: "Сервер",
    appearance: "Внешний вид", themeSystem: "Как в системе", themeDark: "Тёмная", themeLight: "Светлая",
    signOut: "Выйти", confirmSignOut: "Выйти и стереть данные устройства? Нажмите ещё раз",
    today: "Сегодня", yesterday: "Вчера",
  },
};
const LANG = (navigator.language || "en").toLowerCase().startsWith("ru") ? "ru" : "en";
const t = (key, vars = {}) =>
  (WORDS[LANG][key] ?? WORDS.en[key] ?? key).replace(/\{(\w+)\}/g, (_, k) => vars[k] ?? "");

function translatePage() {
  document.documentElement.lang = LANG;
  for (const el of document.querySelectorAll("[data-t]")) el.textContent = t(el.dataset.t);
  for (const el of document.querySelectorAll("[data-tp]")) el.placeholder = t(el.dataset.tp);
  for (const el of document.querySelectorAll("[data-tl]")) {
    el.setAttribute("aria-label", t(el.dataset.tl));
    el.title = t(el.dataset.tl);
  }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

const $ = (id) => document.getElementById(id);
const el = (tag, cls, text) => {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
};
const store = {
  get: (k) => { try { return localStorage.getItem(k); } catch { return null; } },
  set: (k, v) => { try { localStorage.setItem(k, v); } catch { /* per-viewer convenience */ } },
};

function hash(text) {
  let h = 2166136261;
  for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 16777619);
  // FNV's low bits barely move between similar ids: mix them (MurmurHash3's finaliser).
  h ^= h >>> 16;
  h = Math.imul(h, 0x85ebca6b);
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35);
  h ^= h >>> 16;
  return h >>> 0;
}

/** A dot-matrix identicon from the account id, like the macOS avatar: drawn, not fetched. */
function avatar(id, small = false) {
  const svgNs = "http://www.w3.org/2000/svg";
  const h = hash(id);
  // Twelve hues 30° apart: two ids rarely land on colours that look the same.
  const color = `hsl(${(h % 12) * 30} 70% 58%)`;
  const svg = document.createElementNS(svgNs, "svg");
  svg.setAttribute("viewBox", "0 0 40 40");
  svg.setAttribute("class", small ? "avatar small" : "avatar");
  svg.setAttribute("aria-hidden", "true");
  const ring = document.createElementNS(svgNs, "circle");
  ring.setAttribute("cx", "20"); ring.setAttribute("cy", "20"); ring.setAttribute("r", "19.5");
  ring.setAttribute("style", `fill:${color};fill-opacity:.18;stroke:${color};stroke-width:1`);
  svg.append(ring);
  let bits = hash(id + "dots");
  for (let y = 0; y < 5; y++) {
    for (let x = 0; x < 3; x++) {
      const on = bits & 1;
      bits >>>= 1;
      if (!on) continue;
      for (const cx of x === 2 ? [2] : [x, 4 - x]) {
        const dot = document.createElementNS(svgNs, "circle");
        dot.setAttribute("cx", String(10 + cx * 5));
        dot.setAttribute("cy", String(10 + y * 5));
        dot.setAttribute("r", "1.7");
        dot.setAttribute("style", `fill:${color};stroke:none`);
        svg.append(dot);
      }
    }
  }
  return svg;
}

function clock(ms) {
  return new Date(ms).toLocaleTimeString(LANG, { hour: "2-digit", minute: "2-digit" });
}
function dayLabel(ms) {
  const d = new Date(ms);
  const today = new Date();
  const yesterday = new Date(Date.now() - 86400000);
  if (d.toDateString() === today.toDateString()) return t("today");
  if (d.toDateString() === yesterday.toDateString()) return t("yesterday");
  return d.toLocaleDateString(LANG, { day: "numeric", month: "long", year: d.getFullYear() === today.getFullYear() ? undefined : "numeric" });
}
function rowTime(ms) {
  if (!ms) return "";
  const d = new Date(ms);
  return d.toDateString() === new Date().toDateString()
    ? clock(ms)
    : d.toLocaleDateString(LANG, { day: "numeric", month: "short" });
}

let toastTimer;
function toast(text, error = false) {
  const node = $("toast");
  node.textContent = text;
  node.classList.toggle("error", error);
  node.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (node.hidden = true), error ? 6000 : 3000);
}

// ── State the page draws from ───────────────────────────────────────────────

const state = {
  screen: "starting",
  chats: [],
  open: null,
  messages: new Map(), // peerId → [{id, text, sent, time, delivered}]
  account: null,
  filter: "",
  invite: null,
  link: null,
};

// ── Screens ─────────────────────────────────────────────────────────────────

function showScreen(screen) {
  state.screen = screen;
  const id = { unlock: "passphrase", setPassphrase: "passphrase" }[screen] ?? screen;
  for (const s of document.querySelectorAll(".screen")) s.hidden = s.id !== `screen-${id}`;
  if (screen === "unlock" || screen === "setPassphrase") {
    $("passphrase-hint").textContent = t(screen === "unlock" ? "unlockHint" : "setPassphraseHint");
    $("passphrase-submit").textContent = t(screen === "unlock" ? "unlock" : "setPassphrase");
    $("passphrase").autocomplete = screen === "unlock" ? "current-password" : "new-password";
    $("passphrase").value = "";
    $("passphrase").focus();
  }
  if (screen === "onboarding") $("alias").focus();
  if (screen === "link") {
    $("link-qr").hidden = true;
    $("link-wait").hidden = false;
    $("link-error").hidden = true;
    $("link-retry").hidden = true;
  }
  if (screen === "registering") setRegistration(0, 7, "");
  if (screen === "main") renderChats();
}

function setRegistration(done, total, label) {
  $("reg-bar").style.width = `${Math.round((done / total) * 100)}%`;
  $("reg-label").textContent = label;
}

// ── Chat list ───────────────────────────────────────────────────────────────

function renderChats() {
  const list = $("chat-list");
  const query = state.filter.toLowerCase();
  const rows = state.chats.filter((c) => !query || c.name.toLowerCase().includes(query));
  list.replaceChildren(
    ...rows.map((chat) => {
      const li = el("li", "chat-row");
      li.setAttribute("role", "option");
      li.setAttribute("aria-selected", String(chat.peerId === state.open));
      li.dataset.peer = chat.peerId;
      const lines = el("div", "lines");
      const top = el("div", "line");
      top.append(el("span", "name", chat.name), el("span", "time", rowTime(chat.time)));
      const bottom = el("div", "line");
      bottom.append(el("span", "preview", chat.preview));
      if (chat.unread > 0) bottom.append(el("span", "badge", chat.unread > 9999 ? "9999+" : String(chat.unread)));
      lines.append(top, bottom);
      li.append(avatar(chat.peerId), lines);
      li.addEventListener("click", () => openChat(chat.peerId));
      return li;
    }),
  );
}

function chatName(peerId) {
  return state.chats.find((c) => c.peerId === peerId)?.name ?? `${peerId.slice(0, 8)}…`;
}

// ── Conversation ────────────────────────────────────────────────────────────

function openChat(peerId) {
  state.open = peerId;
  $("empty").hidden = true;
  $("chat").hidden = false;
  $("chat-avatar").replaceChildren(avatar(peerId, true));
  $("chat-name").textContent = chatName(peerId);
  renderTranscript();
  invoke("open_chat", { peerId });
  renderChats();
  $("draft").focus();
}

function closeChat() {
  state.open = null;
  $("chat").hidden = true;
  $("empty").hidden = false;
  renderChats();
}

const DELIVERED = '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="m8 12.5 3 3 5-6"/></svg>';
const SENT = '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" style="fill:currentColor"/></svg>';

/** Redraw the open conversation; `stick` keeps it at the newest message, else where it was. */
function renderTranscript(stick = true) {
  const box = $("transcript");
  const kept = box.scrollTop;
  const messages = state.messages.get(state.open) ?? [];
  const nodes = [];
  let lastDay = "";
  let last = null;
  for (const m of messages) {
    const day = new Date(m.time).toDateString();
    if (day !== lastDay) {
      nodes.push(el("div", "day", dayLabel(m.time)));
      lastDay = day;
      last = null;
    }
    nodes.push(messageNode(m, last));
    last = m;
  }
  box.replaceChildren(...nodes);
  box.scrollTop = stick ? box.scrollHeight : kept;
}

function messageNode(m, previous) {
  const wrap = el("div", `msg ${m.sent ? "out" : "in"}`);
  if (previous && (previous.sent !== m.sent || m.time - previous.time > 5 * 60000)) wrap.classList.add("gap");
  wrap.dataset.id = m.id;
  wrap.append(el("div", "bubble", m.text));
  const meta = el("div", "meta");
  if (m.sent) {
    const mark = el("span", m.delivered ? "ok" : "");
    mark.innerHTML = m.delivered ? DELIVERED : SENT; // our own literals, never message data
    meta.append(mark);
  }
  meta.append(el("span", "", clock(m.time)));
  wrap.append(meta);
  return wrap;
}

function appendMessage(peerId, message) {
  const list = state.messages.get(peerId) ?? [];
  if (list.some((m) => m.id === message.id)) return;
  list.push(message);
  state.messages.set(peerId, list);
  if (peerId !== state.open) return;
  const box = $("transcript");
  const atBottom = box.scrollHeight - box.scrollTop - box.clientHeight < 80;
  // Our own message always shows; someone else's does not pull the reader down from history.
  renderTranscript(atBottom || message.sent);
}

function markDelivered(messageId) {
  for (const list of state.messages.values()) {
    const m = list.find((x) => x.id === messageId);
    if (m) {
      m.delivered = true;
      if (state.open && state.messages.get(state.open) === list) renderTranscript(false);
      return;
    }
  }
}

// ── Composer ────────────────────────────────────────────────────────────────

const draft = $("draft");
function fitDraft() {
  // scrollHeight leaves out the border; without it one line overflowed and showed a scrollbar.
  const border = draft.offsetHeight - draft.clientHeight;
  draft.style.height = "auto";
  const wanted = draft.scrollHeight + border;
  draft.style.height = `${Math.min(wanted, 180)}px`;
  draft.style.overflowY = wanted > 180 ? "auto" : "hidden";
  $("send").disabled = draft.value.trim() === "";
}
function sendDraft() {
  const text = draft.value.trim();
  if (!text || !state.open) return;
  invoke("send_message", { peerId: state.open, text });
  draft.value = "";
  fitDraft();
}
draft.addEventListener("input", fitDraft);
draft.addEventListener("keydown", (ev) => {
  if (ev.key === "Enter" && !ev.shiftKey && !ev.isComposing && ev.keyCode !== 229) {
    ev.preventDefault();
    sendDraft();
  }
});
$("composer").addEventListener("submit", (ev) => {
  ev.preventDefault();
  sendDraft();
});

// ── Dialogs ─────────────────────────────────────────────────────────────────

for (const dialog of document.querySelectorAll("dialog")) {
  dialog.addEventListener("click", (ev) => {
    if (ev.target.closest("[data-close]") || ev.target === dialog) dialog.close();
  });
}

function selectTab(dialog, name) {
  for (const tab of dialog.querySelectorAll(".tab[data-tab]")) tab.setAttribute("aria-selected", String(tab.dataset.tab === name));
  for (const pane of dialog.querySelectorAll(".tab-pane")) pane.hidden = pane.dataset.pane !== name;
}

function openAddContact() {
  const dialog = $("dlg-add");
  selectTab(dialog, "paste");
  $("find-results").replaceChildren();
  $("find-query").value = "";
  dialog.showModal();
  $("find-query").focus();
}
$("dlg-add").addEventListener("click", (ev) => {
  const tab = ev.target.closest(".tab[data-tab]");
  if (!tab) return;
  selectTab($("dlg-add"), tab.dataset.tab);
  if (tab.dataset.tab === "myqr" && !state.invite) mintInvite();
});
function mintInvite() {
  state.invite = null;
  $("invite-qr").hidden = true;
  $("invite-wait").hidden = false;
  invoke("mint_invite");
}
$("invite-new").addEventListener("click", mintInvite);
$("invite-copy").addEventListener("click", async () => {
  if (!state.invite) return;
  await navigator.clipboard.writeText(state.invite);
  toast(t("copied"));
});
$("find-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const query = $("find-query").value.trim();
  if (query) invoke("find_contact", { query });
});
function showResults(results) {
  const list = $("find-results");
  if (results.length === 0) {
    list.replaceChildren(el("li", "dim", t("noResults")));
    return;
  }
  list.replaceChildren(
    ...results.map((r) => {
      const li = el("li");
      const add = el("button", "primary", t("add"));
      add.type = "button";
      add.addEventListener("click", () => {
        invoke("add_contact", { userId: r.peerId, name: r.name });
        $("dlg-add").close();
      });
      li.append(avatar(r.peerId, true), el("span", "name", r.name), add);
      return li;
    }),
  );
}

let pendingConfirm = null;
function confirmTwice(button, key, action) {
  if (pendingConfirm === button) {
    pendingConfirm = null;
    action();
    return;
  }
  pendingConfirm = button;
  const original = button.textContent;
  button.textContent = t(key);
  setTimeout(() => {
    if (pendingConfirm === button) pendingConfirm = null;
    button.textContent = original;
  }, 4000);
}

function openInfo() {
  if (!state.open) return;
  const peer = state.open;
  $("info-avatar").replaceChildren(avatar(peer));
  $("info-name").textContent = chatName(peer);
  $("info-id").textContent = peer;
  $("info-hint").textContent = t("safetyHint", { name: chatName(peer) });
  $("safety").replaceChildren();
  invoke("safety_number", { peerId: peer });
  $("dlg-info").showModal();
}
$("info-delete").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmDelete", () => {
    invoke("delete_contact", { peerId: state.open });
    $("dlg-info").close();
    closeChat();
  }),
);

function openSettings() {
  $("set-user").textContent = state.account?.userId ?? "";
  $("set-device").textContent = state.account?.deviceId ?? "";
  $("set-server").textContent = state.account?.server ?? "";
  markTheme();
  $("dlg-settings").showModal();
}
$("sign-out").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmSignOut", () => {
    $("dlg-settings").close();
    invoke("logout");
  }),
);

// ── Theme ───────────────────────────────────────────────────────────────────

function applyTheme(choice) {
  if (choice === "dark" || choice === "light") document.documentElement.dataset.theme = choice;
  else delete document.documentElement.dataset.theme;
}
function markTheme() {
  const choice = store.get("theme") ?? "dark";
  for (const b of document.querySelectorAll("[data-theme-choice]")) b.setAttribute("aria-selected", String(b.dataset.themeChoice === choice));
}
$("theme-choice").addEventListener("click", (ev) => {
  const b = ev.target.closest("[data-theme-choice]");
  if (!b) return;
  store.set("theme", b.dataset.themeChoice);
  applyTheme(b.dataset.themeChoice);
  markTheme();
});

// ── Wiring ──────────────────────────────────────────────────────────────────

$("onboarding-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  invoke("register", { username: $("alias").value.trim() });
});
$("go-link").addEventListener("click", () => invoke("start_link"));
$("link-back").addEventListener("click", () => invoke("cancel_link"));
$("link-retry").addEventListener("click", () => invoke("start_link"));
$("passphrase-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const passphrase = $("passphrase").value;
  if (!passphrase) return;
  invoke(state.screen === "unlock" ? "unlock" : "set_passphrase", { passphrase });
  $("passphrase").value = "";
});
$("chat-search").addEventListener("input", (ev) => {
  state.filter = ev.target.value;
  renderChats();
});
$("add-contact").addEventListener("click", openAddContact);
$("empty-add").addEventListener("click", openAddContact);
$("open-settings").addEventListener("click", openSettings);
$("chat-info").addEventListener("click", openInfo);

// Shortcuts by physical key, so they work on a Russian layout too (Ctrl+K is not Ctrl+Л).
document.addEventListener("keydown", (ev) => {
  if (state.screen !== "main" || document.querySelector("dialog[open]")) return;
  const mod = ev.ctrlKey || ev.metaKey;
  if (mod && ev.code === "KeyK") { ev.preventDefault(); $("chat-search").focus(); }
  else if (mod && ev.code === "KeyN") { ev.preventDefault(); openAddContact(); }
  else if (mod && ev.code === "Comma") { ev.preventDefault(); openSettings(); }
  else if (mod && ev.code === "KeyI") { ev.preventDefault(); openInfo(); }
  else if (ev.altKey && (ev.code === "ArrowDown" || ev.code === "ArrowUp")) {
    ev.preventDefault();
    const ids = state.chats.map((c) => c.peerId);
    if (ids.length === 0) return;
    const i = ids.indexOf(state.open);
    const next = ev.code === "ArrowDown"
      ? (i < 0 ? 0 : Math.min(i + 1, ids.length - 1))
      : (i < 0 ? ids.length - 1 : Math.max(i - 1, 0));
    openChat(ids[next]);
  } else if (mod && /^Digit[1-9]$/.test(ev.code)) {
    const chat = state.chats[Number(ev.code.slice(5)) - 1];
    if (chat) { ev.preventDefault(); openChat(chat.peerId); }
  } else if (ev.key === "Escape" && state.open) closeChat();
});

// ── Events from Rust ────────────────────────────────────────────────────────

function onEvent(e) {
  switch (e.type) {
    case "screen": showScreen(e.screen); break;
    case "account": state.account = e; break;
    case "chats":
      state.chats = e.chats;
      renderChats();
      if (state.open) $("chat-name").textContent = chatName(state.open);
      break;
    case "history":
      state.messages.set(e.peerId, e.messages);
      if (e.peerId === state.open) renderTranscript();
      break;
    case "message": appendMessage(e.peerId, e.message); break;
    case "delivered": markDelivered(e.messageId); break;
    case "connection": {
      const dot = $("connection");
      dot.classList.toggle("online", e.connected);
      dot.title = t(e.connected ? "online" : "offline");
      break;
    }
    case "registrationStep": setRegistration(e.done, e.total, e.label); break;
    case "linkCode":
      $("link-qr").src = e.image;
      $("link-qr").hidden = false;
      $("link-wait").hidden = true;
      $("link-error").hidden = true;
      $("link-retry").hidden = true;
      break;
    case "linkFailed":
      $("link-qr").hidden = true;
      $("link-wait").hidden = true;
      $("link-error").textContent = e.reason;
      $("link-error").hidden = false;
      $("link-retry").hidden = false;
      break;
    case "searchResults": showResults(e.results); break;
    case "safetyNumber":
      if (e.peerId !== state.open) break;
      $("safety").replaceChildren(...e.number.split(/\s+/).filter(Boolean).map((chunk) => el("span", "", chunk)));
      break;
    case "invite":
      if (e.link) {
        state.invite = e.link;
        $("invite-qr").src = e.image;
        $("invite-qr").hidden = false;
        $("invite-wait").hidden = true;
      } else {
        toast(e.error ?? "", true);
      }
      break;
    case "notice": toast(e.text, e.error); break;
  }
}

async function start() {
  applyTheme(store.get("theme") ?? "dark");
  translatePage();
  const events = new Channel();
  events.onmessage = onEvent;
  await invoke("attach", { onEvent: events });
}

start();
