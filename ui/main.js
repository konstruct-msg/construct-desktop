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
    recoveryKey: "Recovery key", makeCopy: "Make a copy…", enterPhrase: "Enter the phrase…",
    recoveryCopyPending: "Your account's key was made on this device. Write down its twelve words: they are the only way back into the account if every device is lost, and nobody, us included, can restore them.",
    recoveryReady: "The copy is made. The phrase is no longer kept on this device.",
    recoveryNeedsPhrase: "This account's key was made on another device. Enter its twelve words here to show your QR code and invite link.",
    recoveryUnknown: "Not checked yet: the server could not be reached.",
    copyHint: "Write these words down, in order, and keep them offline.",
    writtenDown: "I've written them down", later: "Later",
    quizHint: "Type back the words at these positions.", word: "Word {n}", check: "Check",
    quizWrong: "Those words don't match. Look at your copy again.", copyConfirmed: "Copy confirmed",
    enterHint: "The twelve words, in order, separated by spaces.", save: "Save", phraseAccepted: "Phrase accepted",
    about: "About", version: "Version", logs: "Logs", openLogs: "Open logs folder",
    veil: "VEIL", veilOff: "Off", veilAuto: "Auto", veilOn: "On",
    veilHint: "VEIL reaches Konstruct through a front — an ordinary-looking HTTPS site — where Konstruct is blocked. Auto: directly, and through a front when the direct connection is blocked. On: only through a front; with none answering, nothing is sent.",
    veilStateOff: "Connected directly.", veilStateConnecting: "Connecting to a front…",
    veilStateUp: "Connected through a front ({front}).", veilStateNoFront: "No front answers: nothing is sent. Add a link from another device.",
    veilFronts: "Fronts on this device: {n}",
    veilAddLink: "Add a front", veilAddLinkHint: "A konstruct://veil-config link from someone whose Konstruct works, or from your other device.",
    veilLinkPlaceholder: "konstruct://veil-config?d=…", veilImport: "Add",
    veilVoucher: "A link for someone else", veilVoucherHint: "For a person or device that cannot reach Konstruct: a link to one front, valid for a short time. Up to three a day.",
    veilVoucherMake: "Make a link", veilVoucherExp: "Valid until {time}.",
    veilNoAccess: "Can't reach Konstruct?",
    veilStartHint: "Paste a VEIL link someone whose Konstruct works made for you (Settings → VEIL → A link for someone else). Creating an account or linking this device then goes through a front, and nothing goes anywhere else.",
    veilConnect: "Connect", veilDisconnect: "Disconnect",
    diagnostics: "Diagnostics", logsSize: "Size", saveLogs: "Save logs", clearLogs: "Clear logs",
    recentLogs: "Recent lines", logsEmpty: "Nothing written yet", logsCleared: "Logs cleared",
    update: "Updates", updateCheck: "Check for updates", updateInstall: "Install and restart", updatePage: "Release page",
    updateUnknown: "Checked at start and once a day.", updateChecking: "Checking…", updateCurrent: "This is the latest version.",
    updateAvailable: "Version {version} is available.", updateInstalling: "Downloading and installing {version}…",
    updateFailed: "Update failed: {error}",
    updateDeb: "Installed from a .deb: download Konstruct_{version}_amd64.deb from the release page and run sudo apt install ./Konstruct_{version}_amd64.deb",
    updateRpm: "Installed from an .rpm: download Konstruct-{version}-1.x86_64.rpm from the release page and run sudo dnf install ./Konstruct-{version}-1.x86_64.rpm",
    updateManual: "This copy was not installed from a package: build or download the new version.",
    logsSaved: "Logs saved to Downloads: {name}. Click to show the folder; attach the file to your report.",
    diagnosticsHint: "Logs show what the app did: connections, delivery, errors. They hold no message text and no keys, and stay on this computer until you send them yourself.",
    lockHint: "Enter the code to open Konstruct.", unlock: "Unlock",
    lockWrong: "Wrong code", lockWait: "Too many wrong codes: wait {n} s",
    lockForgot: "Forgot the code? Sign out and erase this device",
    confirmLockForgot: "Erase this device? Click again",
    appLock: "App lock", lockOn: "On: the code is asked for after the window has been away", lockOffStatus: "Off",
    lockNowDelay: "At once", lock1m: "1 min", lock5m: "5 min", lock15m: "15 min", lock1h: "1 hour",
    lockDelayHint: "How long the window may be away (another window in front, or minimized) before the code is asked for.",
    lockCurrent: "Current code", lockNew: "New code (at least 4 characters)", lockRepeat: "New code again",
    lockEnable: "Turn on", lockChange: "Change code", lockNow: "Lock now", lockOff: "Turn off",
    lockMismatch: "The two codes differ", lockShort: "A code has at least 4 characters", lockNeedCurrent: "Enter the current code",
    lockScreenHint: "It locks the window, not the data on disk. Forgetting the code means signing out.",
    minutesShort: "min", issuedInvites: "Issued invites", noInvites: "No invite can be redeemed now.",
    inviteKind_qr: "QR code and link", inviteKind_link: "Link", inviteLeft: "{time} left",
    revokeInvite: "Revoke", revokeAllInvites: "Revoke all", confirmRevokeInvite: "Revoke? Click again",
    issuedInvitesHint: "An invite works once and for a few minutes. Revoke one you showed by mistake: nobody can use it then.",
    historyTransfer: "History transfer",
    historyTransferHint: "Move your chats to another device of the account. Over Wi-Fi, with both devices on one network: \"Send over Wi-Fi\" next to the device; it takes the history by itself while Konstruct is open on it. History sent here from your phone arrives the same way. Or as a file: \"History file\" writes one into Downloads, and that device imports it. Either way, only that device can read it.",
    historyNearbySend: "Send over Wi-Fi", historyNearbyReceiving: "Receiving history from your other device…",
    historyNearbyWaiting_send: "Waiting for the other device. Keep Konstruct open on it, on the same network.",
    historyNearbyMoving: "Transferring history…",
    historyNearbySent: "History sent: {messages} messages, {media} files",
    historyNearbySkipped: "The other device chose not to send history",
    historyExport: "History file", historyImport: "Import a history file…",
    historyWriting: "Writing the history file…", historyReading: "Importing the history…",
    historyWritten: "History written: {messages} messages, {media} files → {file}. Click to show",
    historyImported: "History imported: {messages} new messages, {media} files",
    historyFailed: "History not moved: {error}",
    linkDevice: "Link a device", showLinkCode: "Show a code for a new device",
    showLinkCodeHint: "The new device scans it when it signs in to an existing account. This device then vouches for it to your contacts.",
    approvePlaceholder: "konstruct://link-to-me?…", approve: "Link",
    approveHint: "If the new device shows its own code instead (another computer: Copy code), paste it here.",
    offerHint: "On the new device, sign in to an existing account and scan this code. Whoever sees it can join your account: show it to no one.",
    offerValid: "Valid for {time}", offerWaiting: "Waiting for the device…", deviceLinked: "Device {id} linked",
    copyCode: "Copy code", linkCopyHint: "To link to another computer: paste this code in its Settings → Devices.",
    connectionLabel: "Connection", fingerprint: "Key fingerprint", clickToCopy: "Click to copy",
    fingerprintHint: "Your identity key, short. Someone who reads the same digits on their side has your key.",
    dangerZone: "Danger zone", signOutEverywhere: "Sign out everywhere",
    confirmSignOutEverywhere: "Sign out every device and erase this one? Click again",
    signOutEverywhereHint: "Every other device of the account is signed out, then this one is erased. The server keeps the account's first device signed in.",
    deleteAccount: "Delete account",
    deleteAccountWarning: "The account is deleted on the server: your username, your devices, the messages waiting for you. Then this device is erased. Your contacts keep the chats they have. This cannot be undone.",
    deletingIn: "Deleting in {n}… Cancel", deleting: "Deleting…", deleteFailed: "The account was not deleted: {reason}",
    deleteLocally: "Erase only this device", confirmDeleteLocally: "Erase this device, leave the account? Click again",
    dataAndStorage: "Data and storage", mediaCache: "Media cache",
    cacheUsage: "{size} in {n} files", cacheUsageOf: "{size} of {limit} in {n} files",
    clearCache: "Clear media cache", confirmClearCache: "Delete every file? Click again",
    clearCacheHint: "Files are fetched again when shown, while the server still holds them: a week from upload. Older ones are gone for good.",
    autoDownload: "Fetch on arrival", autoNever: "Never", autoSmall: "Up to 16 MB", autoAlways: "Always",
    autoDownloadHint: "The server keeps a file a week from upload. A file not fetched on arrival may be gone when you open its chat.",
    cacheLimit: "Cache limit", noLimit: "No limit",
    cacheLimitHint: "Over the limit, only files the server may still hold are deleted; files fetched over a week ago stay.",
    keepMedia: "Keep files", keepForever: "Always", keep7: "7 days", keep30: "30 days", keep90: "90 days",
    keepMediaHint: "Files fetched longer ago are deleted, the only copies too.",
    unvouchedToast: "{name} has a device none of their other devices confirmed. Open the chat to verify.",
    unvouchedBanner: "{name} has a device none of their other devices confirmed. It may be theirs, or someone else's.",
    compareSafety: "Compare safety numbers", unvouchedTitle: "Unconfirmed device",
    blockUser: "Block user", unblockUser: "Unblock user",
    confirmBlock: "Block? Click again", blockedBadge: "Blocked",
    blockHint: "What {name} sends is not shown and gets no receipt; they are not told. Messages you write them still go out.",
    blockedBanner: "You blocked {name}. What they send is not shown.",
    gapTitle: "Some messages may not have arrived",
    gapBody: "Messages sent to you between {from} and {until} were removed from the server before this device collected them. The server keeps undelivered messages for a limited time.",
    gapBodyOpen: "Messages sent to you since {from} were removed from the server before this device collected them. The server keeps undelivered messages for a limited time.",
    reply: "Reply", edit: "Edit", copy: "Copy", deleteForMe: "Delete for me",
    confirmDeleteMessage: "Delete here? Click again", edited: "edited", replyingTo: "Reply to {name}",
    editing: "Editing", cancel: "Cancel", you: "you", messageActions: "Message actions",
    deletedHint: "Deleted on this device. {name} still has it.",
    attachment_photo: "Photo", attachment_video: "Video", attachment_audio: "Audio",
    attachment_file: "File", attachment_voice: "Voice message", attachment_sticker: "Sticker",
    attachment_location: "Location", attachment_contact: "Contact", attachment_poll: "Poll",
    attachment_other: "Attachment", attachmentHint: "Not shown here yet — open it on your phone.",
    save: "Save", saved: "Saved to Downloads: {name}. Click to show the folder.",
    loading: "Loading…", mediaFailed: "Did not load: {reason}", retry: "Try again",
    recordVoice: "Record a voice message", pause: "Pause", voiceSpeed: "Playback speed",
    play: "Play", close: "Close", attach: "Attach", sending: "Sending {name}…",
    stickers: "Stickers", photoOrVideo: "Photo…", fileEllipsis: "File…",
    recentStickers: "Recent", morePacks: "More packs", addPack: "Add", removePack: "Remove", confirmRemovePack: "Remove?",
    packInfo: "{publisher} · {count} stickers · {size}", catalogFailed: "The catalog did not load",
    noStickers: "No stickers yet", sticker: "Sticker {emoji}",
    tooLarge: "{name} is larger than 95 MB and cannot be sent",
    profile: "Profile", devices: "Devices", chatsAndLook: "Chats and look",
    notificationsAndPrivacy: "Notifications and privacy", usernamePlaceholder: "username",
    saveUsername: "Save", noUsername: "No username",
    usernameHint: "3 to 20 letters, digits or _. People find you by it when you allow it below.",
    discoverable: "People can find me by username",
    discoverableNeedsName: "Choose a username first.",
    discoverableHint: "Off: only people you gave an invite link or QR code can write to you.",
    devicesHint: "Devices signed in to this account.",
    thisDevice: "This device", mainDevice: "Main device", deviceAdded: "added {date}",
    deviceName: "Device {id}", revoke: "Sign out", confirmRevoke: "Sign it out? Click again",
    revokeOthers: "Sign out all other devices", confirmRevokeOthers: "Sign out every other device? Click again",
    loadingDevices: "Loading…", textSize: "Text size", textCompact: "Compact", textStandard: "Standard",
    textLarge: "Large", language: "Language", langAuto: "As in the system", chatsSection: "Chats",
    sendOnEnter: "Send with Enter",
    sendOnEnterHint: "Off: Enter starts a new line, Ctrl+Enter sends.",
    showTimes: "Show the time of each message", notifications: "Notifications",
    notifyOn: "Notify me of new messages", newMessage: "New message",
    previewNameAndMessage: "Name and message", previewNameAndMessageHint: "Show the sender and the text",
    previewNameOnly: "Name only", previewNameOnlyHint: "Show only who wrote",
    previewNone: "No preview", previewNoneHint: "Show only “New message”",
    privacy: "Privacy", deliveryReceipts: "Send delivery receipts",
    deliveryReceiptsHint: "Off: people do not see that your devices received their messages. You still see theirs.",
    choosePhoto: "Choose a photo…", removePhoto: "Remove photo", displayName: "Name",
    displayNamePlaceholder: "Your name, as contacts see it", usernameSection: "Username",
    profileHint: "Only people you share your profile with see your name and photo. The switch is in each contact's card.",
    shareProfile: "Share my name and photo",
    shareProfileHint: "They get your profile now, and again whenever you change it.",
    photoUnreadable: "This picture could not be read", photoSending: "Photo saved; contacts get it shortly",
    thisDeviceName: "This device's name", allDevices: "All devices",
    deviceNameHint: "The account's other devices see it; it is sealed to them, the server cannot read it.",
    platform_ios: "iPhone", platform_android: "Android", platform_desktop: "Computer",
    platform_web: "Web", platform_bot: "Bot", unnamedDevice: "Not named yet",
    aliasPlaceholder: "Your name for them", aliasHint: "Only you see it; it stays on this device.",
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
    recoveryKey: "Ключ восстановления", makeCopy: "Сделать копию…", enterPhrase: "Ввести фразу…",
    recoveryCopyPending: "Ключ аккаунта создан на этом устройстве. Запишите его двенадцать слов: только по ним можно вернуть аккаунт, если пропадут все устройства, и никто, включая нас, их не восстановит.",
    recoveryReady: "Копия сделана. Фраза на этом устройстве больше не хранится.",
    recoveryNeedsPhrase: "Ключ этого аккаунта создан на другом устройстве. Введите здесь его двенадцать слов, чтобы показывать свой QR-код и ссылку-приглашение.",
    recoveryUnknown: "Ещё не проверено: сервер недоступен.",
    copyHint: "Запишите эти слова по порядку и храните не в сети.",
    writtenDown: "Я записал", later: "Позже",
    quizHint: "Введите слова, стоящие на этих местах.", word: "Слово {n}", check: "Проверить",
    quizWrong: "Слова не совпадают. Сверьтесь с записью.", copyConfirmed: "Копия подтверждена",
    enterHint: "Двенадцать слов по порядку через пробел.", save: "Сохранить", phraseAccepted: "Фраза принята",
    about: "О приложении", version: "Версия", logs: "Логи", openLogs: "Открыть папку с логами",
    veil: "VEIL", veilOff: "Выкл", veilAuto: "Авто", veilOn: "Вкл",
    veilHint: "VEIL подключает к Konstruct через фронт — обычный с виду HTTPS-сайт — там, где Konstruct блокируют. Авто: напрямую, а если прямое соединение блокируют — через фронт. Вкл: только через фронт; если ни один не отвечает, ничего не отправляется.",
    veilStateOff: "Соединение напрямую.", veilStateConnecting: "Подключение к фронту…",
    veilStateUp: "Подключено через фронт ({front}).", veilStateNoFront: "Ни один фронт не отвечает: ничего не отправляется. Добавьте ссылку с другого устройства.",
    veilFronts: "Фронтов на этом устройстве: {n}",
    veilAddLink: "Добавить фронт", veilAddLinkHint: "Ссылка konstruct://veil-config от того, у кого Konstruct работает, или с вашего другого устройства.",
    veilLinkPlaceholder: "konstruct://veil-config?d=…", veilImport: "Добавить",
    veilVoucher: "Ссылка для другого", veilVoucherHint: "Тому, кто не может подключиться к Konstruct: ссылка на один фронт, действует недолго. Не больше трёх в день.",
    veilVoucherMake: "Сделать ссылку", veilVoucherExp: "Действует до {time}.",
    veilNoAccess: "Нет доступа к Konstruct?",
    veilStartHint: "Вставьте ссылку VEIL, которую сделал для вас тот, у кого Konstruct работает (Настройки → VEIL → Ссылка для другого). Тогда создание аккаунта или привязка этого устройства пойдут через фронт, и больше никуда.",
    veilConnect: "Подключиться", veilDisconnect: "Отключить",
    diagnostics: "Диагностика", logsSize: "Размер", saveLogs: "Сохранить логи", clearLogs: "Очистить логи",
    recentLogs: "Последние строки", logsEmpty: "Пока ничего не записано", logsCleared: "Логи очищены",
    update: "Обновления", updateCheck: "Проверить обновления", updateInstall: "Установить и перезапустить", updatePage: "Страница релиза",
    updateUnknown: "Проверяется при запуске и раз в день.", updateChecking: "Проверка…", updateCurrent: "Установлена последняя версия.",
    updateAvailable: "Доступна версия {version}.", updateInstalling: "Загрузка и установка {version}…",
    updateFailed: "Обновление не удалось: {error}",
    updateDeb: "Установлено из .deb: скачайте Konstruct_{version}_amd64.deb на странице релиза и выполните sudo apt install ./Konstruct_{version}_amd64.deb",
    updateRpm: "Установлено из .rpm: скачайте Konstruct-{version}-1.x86_64.rpm на странице релиза и выполните sudo dnf install ./Konstruct-{version}-1.x86_64.rpm",
    updateManual: "Эта копия установлена не из пакета: соберите или скачайте новую версию.",
    logsSaved: "Логи сохранены в «Загрузки»: {name}. Нажмите, чтобы открыть папку, и приложите файл к сообщению.",
    diagnosticsHint: "В логах — что делало приложение: соединения, доставка, ошибки. Текста сообщений и ключей в них нет, и они остаются на этом компьютере, пока вы сами их не отправите.",
    lockHint: "Введите код, чтобы открыть Konstruct.", unlock: "Открыть",
    lockWrong: "Неверный код", lockWait: "Слишком много неверных кодов: подождите {n} с",
    lockForgot: "Забыли код? Выйти и стереть устройство",
    confirmLockForgot: "Стереть устройство? Нажмите ещё раз",
    appLock: "Блокировка", lockOn: "Включена: код спрашивается, когда окно было в стороне", lockOffStatus: "Выключена",
    lockNowDelay: "Сразу", lock1m: "1 мин", lock5m: "5 мин", lock15m: "15 мин", lock1h: "1 час",
    lockDelayHint: "Сколько окно может быть в стороне (другое окно поверх или свёрнуто), прежде чем спросить код.",
    lockCurrent: "Текущий код", lockNew: "Новый код (не меньше 4 символов)", lockRepeat: "Новый код ещё раз",
    lockEnable: "Включить", lockChange: "Сменить код", lockNow: "Заблокировать сейчас", lockOff: "Выключить",
    lockMismatch: "Коды не совпадают", lockShort: "Код — не меньше 4 символов", lockNeedCurrent: "Введите текущий код",
    lockScreenHint: "Закрывается окно, а не данные на диске. Забытый код — это выход из аккаунта.",
    minutesShort: "мин", issuedInvites: "Выданные приглашения", noInvites: "Сейчас ни одним приглашением нельзя воспользоваться.",
    inviteKind_qr: "QR-код и ссылка", inviteKind_link: "Ссылка", inviteLeft: "осталось {time}",
    revokeInvite: "Отозвать", revokeAllInvites: "Отозвать все", confirmRevokeInvite: "Отозвать? Нажмите ещё раз",
    issuedInvitesHint: "Приглашение срабатывает один раз и действует несколько минут. Отзовите показанное по ошибке, и им никто не воспользуется.",
    historyTransfer: "Перенос истории",
    historyTransferHint: "Перенесите переписку на другое устройство аккаунта. По Wi-Fi, когда оба устройства в одной сети: «Отправить по Wi-Fi» рядом с устройством, и оно примет историю само, пока на нём открыт Konstruct. История, отправленная сюда с телефона, приходит так же. Или файлом: «Файл истории» записывает его в Загрузки, а то устройство его импортирует. В обоих случаях прочитать историю может только это устройство.",
    historyNearbySend: "Отправить по Wi-Fi", historyNearbyReceiving: "Получаю историю с другого устройства…",
    historyNearbyWaiting_send: "Жду другое устройство. На нём должен быть открыт Konstruct, в той же сети.",
    historyNearbyMoving: "Переношу историю…",
    historyNearbySent: "История отправлена: сообщений {messages}, файлов {media}",
    historyNearbySkipped: "Другое устройство решило не отправлять историю",
    historyExport: "Файл истории", historyImport: "Импортировать файл истории…",
    historyWriting: "Записываю файл истории…", historyReading: "Импортирую историю…",
    historyWritten: "История записана: сообщений {messages}, файлов {media} → {file}. Нажмите, чтобы показать",
    historyImported: "История импортирована: новых сообщений {messages}, файлов {media}",
    historyFailed: "История не перенесена: {error}",
    linkDevice: "Привязать устройство", showLinkCode: "Показать код для нового устройства",
    showLinkCodeHint: "Новое устройство сканирует его при входе в существующий аккаунт. Затем это устройство подтверждает его вашим собеседникам.",
    approvePlaceholder: "konstruct://link-to-me?…", approve: "Привязать",
    approveHint: "Если новое устройство показывает свой код (другой компьютер: «Скопировать код»), вставьте его сюда.",
    offerHint: "На новом устройстве войдите в существующий аккаунт и отсканируйте этот код. Кто его увидит, сможет войти в ваш аккаунт: никому его не показывайте.",
    offerValid: "Действует ещё {time}", offerWaiting: "Жду устройство…", deviceLinked: "Устройство {id} привязано",
    copyCode: "Скопировать код", linkCopyHint: "Чтобы привязать к другому компьютеру, вставьте этот код в его Настройки → Устройства.",
    connectionLabel: "Соединение", fingerprint: "Отпечаток ключа", clickToCopy: "Нажмите, чтобы скопировать",
    fingerprintHint: "Ваш ключ личности, коротко. Если у собеседника те же цифры, у него ваш ключ.",
    dangerZone: "Опасная зона", signOutEverywhere: "Выйти на всех устройствах",
    confirmSignOutEverywhere: "Выйти везде и стереть это устройство? Нажмите ещё раз",
    signOutEverywhereHint: "Все остальные устройства аккаунта отключаются, затем это устройство стирается. Первое устройство аккаунта сервер оставляет подключённым.",
    deleteAccount: "Удалить аккаунт",
    deleteAccountWarning: "Аккаунт удаляется на сервере: имя пользователя, устройства, ждущие вас сообщения. Затем это устройство стирается. У собеседников переписка остаётся. Отменить нельзя.",
    deletingIn: "Удаление через {n}… Отменить", deleting: "Удаляю…", deleteFailed: "Аккаунт не удалён: {reason}",
    deleteLocally: "Стереть только это устройство", confirmDeleteLocally: "Стереть устройство, оставив аккаунт? Нажмите ещё раз",
    dataAndStorage: "Данные и память", mediaCache: "Кэш медиа",
    cacheUsage: "{size}, файлов: {n}", cacheUsageOf: "{size} из {limit}, файлов: {n}",
    clearCache: "Очистить кэш медиа", confirmClearCache: "Удалить все файлы? Нажмите ещё раз",
    clearCacheHint: "Файлы скачаются снова при показе, пока они есть на сервере: неделю с момента отправки. Более старые пропадут насовсем.",
    autoDownload: "Скачивать при получении", autoNever: "Никогда", autoSmall: "До 16 МБ", autoAlways: "Всегда",
    autoDownloadHint: "Сервер хранит файл неделю с момента отправки. Файл, не скачанный сразу, может пропасть к тому времени, как вы откроете чат.",
    cacheLimit: "Предел кэша", noLimit: "Без предела",
    cacheLimitHint: "Сверх предела удаляются только файлы, которые ещё могут быть на сервере; скачанные больше недели назад остаются.",
    keepMedia: "Хранить файлы", keepForever: "Всегда", keep7: "7 дней", keep30: "30 дней", keep90: "90 дней",
    keepMediaHint: "Файлы, скачанные раньше, удаляются, в том числе единственные копии.",
    unvouchedToast: "У {name} появилось устройство, которое не подтвердило ни одно другое устройство этого аккаунта. Откройте чат, чтобы проверить.",
    unvouchedBanner: "У {name} появилось устройство, которое не подтвердило ни одно другое устройство этого аккаунта. Возможно, оно не принадлежит собеседнику.",
    compareSafety: "Сравнить защитные числа", unvouchedTitle: "Неподтверждённое устройство",
    blockUser: "Заблокировать пользователя", unblockUser: "Разблокировать пользователя",
    confirmBlock: "Заблокировать? Нажмите ещё раз", blockedBadge: "Заблокирован",
    blockHint: "Что присылает {name}, не показывается и не подтверждается; их об этом не уведомляют. Ваши сообщения им по-прежнему уходят.",
    blockedBanner: "Вы заблокировали {name}. То, что они присылают, не показывается.",
    gapTitle: "Часть сообщений могла не дойти",
    gapBody: "Сообщения, отправленные вам с {from} по {until}, удалены с сервера раньше, чем это устройство их забрало. Сервер хранит недоставленные сообщения ограниченное время.",
    gapBodyOpen: "Сообщения, отправленные вам с {from}, удалены с сервера раньше, чем это устройство их забрало. Сервер хранит недоставленные сообщения ограниченное время.",
    reply: "Ответить", edit: "Изменить", copy: "Копировать", deleteForMe: "Удалить у себя",
    confirmDeleteMessage: "Удалить здесь? Нажмите ещё раз", edited: "изменено",
    replyingTo: "Ответ {name}", editing: "Редактирование", cancel: "Отмена", you: "вам",
    messageActions: "Действия с сообщением",
    deletedHint: "Удалено на этом устройстве. У {name} оно осталось.",
    attachment_photo: "Фото", attachment_video: "Видео", attachment_audio: "Аудио",
    attachment_file: "Файл", attachment_voice: "Голосовое сообщение", attachment_sticker: "Стикер",
    attachment_location: "Геопозиция", attachment_contact: "Контакт", attachment_poll: "Опрос",
    attachment_other: "Вложение", attachmentHint: "Здесь пока не показывается — откройте на телефоне.",
    save: "Сохранить", saved: "Сохранено в «Загрузки»: {name}. Нажмите, чтобы открыть папку.",
    loading: "Загрузка…", mediaFailed: "Не загрузилось: {reason}", retry: "Повторить",
    recordVoice: "Записать голосовое сообщение", pause: "Пауза", voiceSpeed: "Скорость воспроизведения",
    play: "Воспроизвести", close: "Закрыть", attach: "Прикрепить", sending: "Отправляется {name}…",
    stickers: "Стикеры", photoOrVideo: "Фото…", fileEllipsis: "Файл…",
    recentStickers: "Недавние", morePacks: "Ещё наборы", addPack: "Добавить", removePack: "Убрать", confirmRemovePack: "Убрать?",
    packInfo: "{publisher} · стикеров: {count} · {size}", catalogFailed: "Каталог не загрузился",
    noStickers: "Стикеров пока нет", sticker: "Стикер {emoji}",
    tooLarge: "{name} больше 95 МБ — такой файл не отправить",
    profile: "Профиль", devices: "Устройства", chatsAndLook: "Чаты и вид",
    notificationsAndPrivacy: "Уведомления и приватность", usernamePlaceholder: "имя пользователя",
    saveUsername: "Сохранить", noUsername: "Без имени пользователя",
    usernameHint: "От 3 до 20 латинских букв, цифр или _. По нему вас находят, если вы разрешите ниже.",
    discoverable: "Меня можно найти по имени пользователя",
    discoverableNeedsName: "Сначала выберите имя пользователя.",
    discoverableHint: "Выключено: написать вам смогут только те, кому вы дали ссылку-приглашение или QR-код.",
    devicesHint: "Устройства, в которых открыт этот аккаунт.",
    thisDevice: "Это устройство", mainDevice: "Основное устройство", deviceAdded: "добавлено {date}",
    deviceName: "Устройство {id}", revoke: "Отключить", confirmRevoke: "Отключить? Нажмите ещё раз",
    revokeOthers: "Отключить все остальные устройства", confirmRevokeOthers: "Отключить все остальные? Нажмите ещё раз",
    loadingDevices: "Загрузка…", textSize: "Размер текста", textCompact: "Мелкий", textStandard: "Обычный",
    textLarge: "Крупный", language: "Язык", langAuto: "Как в системе", chatsSection: "Чаты",
    sendOnEnter: "Отправлять по Enter",
    sendOnEnterHint: "Выключено: Enter переносит строку, Ctrl+Enter отправляет.",
    showTimes: "Показывать время сообщений", notifications: "Уведомления",
    notifyOn: "Уведомлять о новых сообщениях", newMessage: "Новое сообщение",
    previewNameAndMessage: "Имя и текст", previewNameAndMessageHint: "Показывать отправителя и текст",
    previewNameOnly: "Только имя", previewNameOnlyHint: "Показывать только, кто написал",
    previewNone: "Без превью", previewNoneHint: "Показывать только «Новое сообщение»",
    privacy: "Приватность", deliveryReceipts: "Отправлять отчёты о доставке",
    deliveryReceiptsHint: "Выключено: собеседники не видят, что сообщение дошло до ваших устройств. Их отчёты вы видите по-прежнему.",
    choosePhoto: "Выбрать фото…", removePhoto: "Убрать фото", displayName: "Имя",
    displayNamePlaceholder: "Ваше имя для контактов", usernameSection: "Имя пользователя",
    profileHint: "Имя и фото видят только те, с кем вы делитесь профилем. Переключатель — в карточке контакта.",
    shareProfile: "Делиться моим именем и фото",
    shareProfileHint: "Профиль придёт им сейчас и потом при каждом изменении.",
    photoUnreadable: "Эту картинку не удалось прочитать", photoSending: "Фото сохранено, контакты скоро его получат",
    thisDeviceName: "Имя этого устройства", allDevices: "Все устройства",
    deviceNameHint: "Его видят другие устройства аккаунта: оно зашифровано для них, сервер его не читает.",
    platform_ios: "iPhone", platform_android: "Android", platform_desktop: "Компьютер",
    platform_web: "Веб", platform_bot: "Бот", unnamedDevice: "Пока без имени",
    aliasPlaceholder: "Ваше имя для собеседника", aliasHint: "Его видите только вы, оно хранится на этом устройстве.",
  },
};
// The language chosen in settings, or the system's. A change reloads the page.
const LANG = (() => {
  let chosen = null;
  try { chosen = localStorage.getItem("lang"); } catch { /* per-viewer convenience */ }
  if (chosen === "ru" || chosen === "en") return chosen;
  return (navigator.language || "en").toLowerCase().startsWith("ru") ? "ru" : "en";
})();
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
// A person's picture when their profile carried one (a data URI Rust checked to be an image),
// else the pattern drawn from their id. `url` overrides, for our own.
function avatar(id, small = false, url = state.avatars.get(id)) {
  if (url) {
    const img = el("img", small ? "avatar small" : "avatar");
    img.src = url;
    img.alt = "";
    return img;
  }
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
// `action`: what a click on it does, if anything.
function toast(text, error = false, action = null) {
  const node = $("toast");
  node.textContent = text;
  node.onclick = action;
  node.classList.toggle("action", Boolean(action));
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
  messages: new Map(), // peerId → [MessageRow, as bridge.rs sends it]
  compose: null, // {mode: "reply" | "edit", message}
  account: null,
  recovery: "unknown",
  filter: "",
  invite: null,
  settings: null, // the account's settings as the client last sent them
  lock: { enabled: false, delaySecs: 0, locked: false },
  connected: false,
  avatars: new Map(), // peerId → data URI of the picture their profile carried
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
  if (screen === "onboarding") {
    // Signed out or deleted: what was open belonged to the account.
    resetDelete();
    onUnlocked();
    for (const d of document.querySelectorAll("dialog[open]")) d.close();
    state.settings = null;
    $("alias").focus();
  }
  if (screen === "link") {
    state.linkCode = null;
    $("link-copy").hidden = true;
    $("link-copy-hint").hidden = true;
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
      top.append(el("span", "name", chat.name));
      if (chat.unvouched) {
        const warn = el("span", "warn", "!");
        warn.title = t("unvouchedTitle");
        top.append(warn);
      }
      if (chat.blocked) {
        const mark = el("span", "blocked-mark", "⊘");
        mark.title = t("blockedBadge");
        top.append(mark);
      }
      top.append(el("span", "time", rowTime(chat.time)));
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

/** The banner over the open conversation while its account shows an unconfirmed device. */
function renderAlarm() {
  const row = state.chats.find((c) => c.peerId === state.open);
  // The banner takes height from the transcript: a reader at the newest message stays there.
  const box = $("transcript");
  const atBottom = box.scrollHeight - box.scrollTop - box.clientHeight < 8;
  $("alarm").hidden = !row?.unvouched;
  $("blocked-note").hidden = !row?.blocked;
  if (atBottom) box.scrollTop = box.scrollHeight;
  if (row?.unvouched) $("alarm-text").textContent = t("unvouchedBanner", { name: row.name });
  if (row?.blocked) $("blocked-text").textContent = t("blockedBanner", { name: row.name });
}
$("alarm-compare").addEventListener("click", () => openInfo());
$("blocked-unblock").addEventListener("click", () => invoke("set_blocked", { peerId: state.open, blocked: false }));

/** Messages the server removed before this device collected them: one notice above the chat
 *  list until closed (decisions/mailbox-gap-is-reported.md). Whose they were nobody knows. */
function onMailboxGap(gap) {
  $("gap").hidden = !gap;
  if (!gap) return;
  const when = (ms) => new Date(ms).toLocaleString(LANG, { dateStyle: "medium", timeStyle: "short" });
  $("gap-text").textContent = gap.until == null
    ? t("gapBodyOpen", { from: when(gap.from) })
    : t("gapBody", { from: when(gap.from), until: when(gap.until) });
}
$("gap-close").addEventListener("click", () => invoke("close_mailbox_gap"));

function chatName(peerId) {
  return state.chats.find((c) => c.peerId === peerId)?.name ?? `${peerId.slice(0, 8)}…`;
}

// ── Conversation ────────────────────────────────────────────────────────────

function openChat(peerId) {
  if (state.open !== peerId && state.compose) cancelCompose();
  if (state.open !== peerId && recording) stopRecording(false);
  closeMessageMenu();
  closePicker();
  state.open = peerId;
  $("empty").hidden = true;
  $("chat").hidden = false;
  $("chat-avatar").replaceChildren(avatar(peerId, true));
  $("chat-name").textContent = chatName(peerId);
  renderAlarm();
  renderTranscript();
  invoke("open_chat", { peerId });
  renderChats();
  $("draft").focus();
}

function closeChat() {
  if (recording) stopRecording(false);
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

const REACTIONS = ["👍", "❤️", "😂", "😮", "😢", "🙏"];

function messageNode(m, previous) {
  const wrap = el("div", `msg ${m.sent ? "out" : "in"}`);
  if (previous && (previous.sent !== m.sent || m.time - previous.time > 5 * 60000)) wrap.classList.add("gap");
  wrap.dataset.id = m.id;
  const bubble = el("div", m.sticker ? "bubble sticker" : "bubble");
  if (m.sticker) bubble.append(stickerArt(m.sticker));
  if (m.quote) {
    const quote = el("button", "quote");
    quote.type = "button";
    quote.textContent = m.quote.text || "…";
    quote.addEventListener("click", () => scrollToMessage(m.quote.id));
    bubble.append(quote);
  }
  m.media.forEach((item, index) => bubble.append(mediaNode(m, item, index)));
  if (m.attachment && m.media.length === 0 && !m.sticker) {
    const chip = el("div", "attachment");
    chip.append(el("span", "kind", `📎 ${t(`attachment_${m.attachment}`)}`), el("span", "hint", t("attachmentHint")));
    bubble.append(chip);
  }
  if (m.text && !m.sticker) bubble.append(el("span", "text", m.text));
  wrap.append(bubble);
  if (m.reactions.length > 0) {
    const row = el("div", "reactions");
    for (const r of m.reactions) {
      const chip = el("button", r.mine ? "reaction mine" : "reaction", r.count > 1 ? `${r.emoji} ${r.count}` : r.emoji);
      chip.type = "button";
      chip.addEventListener("click", () => invoke("react", { messageId: m.id, emoji: r.emoji }));
      row.append(chip);
    }
    wrap.append(row);
  }
  const meta = el("div", "meta");
  if (m.edited) meta.append(el("span", "", t("edited")));
  if (m.sent) {
    const mark = el("span", m.delivered ? "ok" : "");
    mark.innerHTML = m.delivered ? DELIVERED : SENT; // our own literals, never message data
    meta.append(mark);
  }
  meta.append(el("span", "time", clock(m.time)));
  const more = el("button", "more", "⋯");
  more.type = "button";
  more.setAttribute("aria-label", t("messageActions"));
  more.addEventListener("click", (ev) => {
    ev.stopPropagation();
    openMessageMenu(m, ev.currentTarget.getBoundingClientRect());
  });
  meta.append(more);
  wrap.append(meta);
  bubble.addEventListener("contextmenu", (ev) => {
    ev.preventDefault();
    openMessageMenu(m, { left: ev.clientX, right: ev.clientX, top: ev.clientY, bottom: ev.clientY });
  });
  bubble.addEventListener("dblclick", () => {
    window.getSelection()?.removeAllRanges();
    invoke("react", { messageId: m.id, emoji: "❤️" });
  });
  return wrap;
}

// ── Attachments ─────────────────────────────────────────────────────────────
// A file is fetched and opened by the client; the page gets a picture or a sound as a data URI,
// or nothing — anything else is saved to disk, never put in the page.

const media = new Map(); // "messageId:index" → {loading} | {url} | {url: null} | {error}
const mediaKey = (id, index) => `${id}:${index}`;

function loadMedia(m, index) {
  const key = mediaKey(m.id, index);
  if (media.has(key) && !media.get(key).error) return;
  media.set(key, { loading: true });
  invoke("load_media", { messageId: m.id, index });
}

// Asked again from the start: a failed fetch, tried once more.
function reloadMedia(m, index) {
  media.delete(mediaKey(m.id, index));
  loadMedia(m, index);
  redrawMedia(m.id, index);
}

function saveMedia(m, index) {
  invoke("save_media", { messageId: m.id, index });
}

function onMedia(e) {
  media.set(mediaKey(e.messageId, e.index), e.error ? { error: e.error } : { url: e.url ?? null });
  redrawMedia(e.messageId, e.index, true);
}

function redrawMedia(messageId, index, play = false) {
  const m = (state.messages.get(state.open) ?? []).find((x) => x.id === messageId);
  if (!m) return;
  for (const node of $("transcript").querySelectorAll(".media")) {
    if (node.dataset.key === mediaKey(messageId, index)) node.replaceWith(mediaNode(m, m.media[index], index, play));
  }
}

function fileSize(bytes) {
  if (!bytes) return "";
  const units = LANG === "ru" ? ["Б", "КБ", "МБ", "ГБ"] : ["B", "KB", "MB", "GB"];
  let n = bytes, u = 0;
  while (n >= 1024 && u < units.length - 1) { n /= 1024; u++; }
  return `${n < 10 && u > 0 ? n.toFixed(1) : Math.round(n)} ${units[u]}`;
}

function duration(ms) {
  const s = Math.round(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

function mediaNode(m, item, index, play = false) {
  const key = mediaKey(m.id, index);
  const entry = media.get(key);
  const node = el("div", `media ${item.kind}`);
  node.dataset.key = key;
  const picture = item.kind === "photo" || item.kind === "sticker";
  const sound = item.kind === "voice" || item.kind === "audio";

  if (picture && entry?.url) {
    const img = el("img");
    img.src = entry.url;
    img.alt = t(`attachment_${item.kind}`);
    img.addEventListener("click", () => openViewer(m, index, entry.url));
    node.append(img);
    return node;
  }
  if (item.kind === "voice" && entry?.url !== null && !entry?.error) {
    node.append(voiceNode(m, item, index, entry));
    return node;
  }
  if (sound && entry?.url) {
    const audio = el("audio");
    audio.controls = true;
    audio.src = entry.url;
    if (play) audio.play().catch(() => {});
    node.append(audio);
    return node;
  }
  if (picture && !entry) loadMedia(m, index);
  if (picture && (!entry || entry.loading)) {
    const box = el("div", "placeholder", t("loading"));
    if (item.width && item.height) {
      const w = Math.min(280, item.width);
      box.style.width = `${w}px`;
      box.style.aspectRatio = `${item.width} / ${item.height}`;
    }
    node.append(box);
    return node;
  }

  // A file, a sound not played yet, a picture the page cannot show, or one that failed.
  const icon = sound ? "▶" : item.kind === "video" ? "🎬" : picture ? "🖼" : "📄";
  const name = item.name || t(`attachment_${item.kind}`);
  const details = [sound && item.durationMs ? duration(item.durationMs) : "", fileSize(item.size)]
    .filter(Boolean).join(" · ");
  const row = el("div", "file");
  const lead = el("button", "file-icon", icon);
  lead.type = "button";
  lead.setAttribute("aria-label", sound ? t("play") : t("save"));
  // A sound plays here once fetched, unless the page could not play it (`url: null`): then saved.
  const playable = sound && entry?.url !== null;
  lead.addEventListener("click", () => {
    if (!playable) return saveMedia(m, index);
    if (entry?.error) return reloadMedia(m, index);
    loadMedia(m, index);
    redrawMedia(m.id, index);
  });
  const label = el("div", "file-label");
  label.append(el("span", "file-name", name));
  if (entry?.loading) label.append(el("span", "file-details", t("loading")));
  else if (entry?.error) label.append(el("span", "file-details error", t("mediaFailed", { reason: entry.error })));
  else if (details) label.append(el("span", "file-details", details));
  const save = el("button", "link", entry?.error ? t("retry") : t("save"));
  save.type = "button";
  save.addEventListener("click", () => (entry?.error ? reloadMedia(m, index) : saveMedia(m, index)));
  row.append(lead, label, save);
  node.append(row);
  return node;
}

function openViewer(m, index, url) {
  const overlay = el("div", "viewer");
  overlay.tabIndex = -1;
  const img = el("img");
  img.src = url;
  const bar = el("div", "viewer-bar");
  const save = el("button", "secondary", t("save"));
  save.type = "button";
  save.addEventListener("click", (ev) => { ev.stopPropagation(); saveMedia(m, index); });
  const close = el("button", "secondary", t("close"));
  close.type = "button";
  bar.append(save, close);
  overlay.append(img, bar);
  const done = () => overlay.remove();
  overlay.addEventListener("click", (ev) => { if (ev.target !== img) done(); });
  overlay.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape") { ev.preventDefault(); ev.stopPropagation(); done(); }
  });
  document.body.append(overlay);
  overlay.focus();
}

function onMediaSaved(e) {
  toast(t("saved", { name: e.name }), false, () => invoke("open_downloads").catch((err) => toast(String(err), true)));
}

function scrollToMessage(id) {
  const node = [...$("transcript").querySelectorAll(".msg")].find((n) => n.dataset.id === id);
  if (!node) return;
  node.scrollIntoView({ block: "center", behavior: "smooth" });
  node.classList.add("flash");
  setTimeout(() => node.classList.remove("flash"), 1200);
}

// ── Message menu ────────────────────────────────────────────────────────────

function openMessageMenu(m, at) {
  const menu = $("msg-menu");
  const quick = el("div", "quick");
  for (const emoji of REACTIONS) {
    const b = el("button", m.reactions.some((r) => r.mine && r.emoji === emoji) ? "mine" : "", emoji);
    b.type = "button";
    b.addEventListener("click", () => { closeMessageMenu(); invoke("react", { messageId: m.id, emoji }); });
    quick.append(b);
  }
  const item = (key, action, cls = "") => {
    const b = el("button", `item ${cls}`, t(key));
    b.type = "button";
    b.addEventListener("click", (ev) => action(ev.currentTarget));
    return b;
  };
  const items = [item("reply", () => { closeMessageMenu(); startCompose("reply", m); })];
  if (m.sent && !m.attachment) items.push(item("edit", () => { closeMessageMenu(); startCompose("edit", m); }));
  if (m.text && !m.sticker) {
    items.push(item("copy", async () => {
      closeMessageMenu();
      await navigator.clipboard.writeText(m.text);
      toast(t("copied"));
    }));
  }
  items.push(item("deleteForMe", (button) => confirmTwice(button, "confirmDeleteMessage", () => {
    closeMessageMenu();
    invoke("delete_message", { messageId: m.id });
  }), "danger"));
  menu.replaceChildren(quick, ...items);
  menu.hidden = false;
  // Beside the message, inside the window.
  const { innerWidth: w, innerHeight: h } = window;
  const box = menu.getBoundingClientRect();
  const left = Math.min(Math.max(8, at.left), w - box.width - 8);
  const top = at.bottom + box.height + 8 < h ? at.bottom + 4 : Math.max(8, at.top - box.height - 4);
  menu.style.left = `${left}px`;
  menu.style.top = `${top}px`;
  menu.querySelector(".item")?.focus();
}
function closeMessageMenu() {
  $("msg-menu").hidden = true;
}
document.addEventListener("click", (ev) => {
  if (!$("msg-menu").hidden && !ev.target.closest("#msg-menu")) closeMessageMenu();
});
$("transcript").addEventListener("scroll", closeMessageMenu);
window.addEventListener("blur", closeMessageMenu);

// ── Replying and editing ────────────────────────────────────────────────────

function startCompose(mode, message) {
  state.compose = { mode, message };
  $("compose-label").textContent = mode === "edit"
    ? t("editing")
    : t("replyingTo", { name: message.sent ? t("you") : chatName(state.open) });
  $("compose-text").textContent = message.text || t(`attachment_${message.attachment ?? "other"}`);
  $("compose-bar").hidden = false;
  if (mode === "edit") draft.value = message.text;
  fitDraft();
  draft.focus();
  draft.setSelectionRange(draft.value.length, draft.value.length);
}
function cancelCompose() {
  if (state.compose?.mode === "edit") draft.value = "";
  state.compose = null;
  $("compose-bar").hidden = true;
  fitDraft();
}
$("compose-cancel").addEventListener("click", () => { cancelCompose(); draft.focus(); });

function replaceMessage(peerId, message) {
  const list = state.messages.get(peerId);
  const i = list?.findIndex((m) => m.id === message.id) ?? -1;
  if (i < 0) return;
  list[i] = message;
  if (state.compose?.message.id === message.id) state.compose.message = message;
  if (peerId === state.open) renderTranscript(false);
}
function removeMessage(peerId, messageId) {
  const list = state.messages.get(peerId);
  if (!list) return;
  state.messages.set(peerId, list.filter((m) => m.id !== messageId));
  if (state.compose?.message.id === messageId) cancelCompose();
  if (peerId === state.open) renderTranscript(false);
  toast(t("deletedHint", { name: chatName(peerId) }));
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
  const empty = draft.value.trim() === "";
  $("send").disabled = empty;
  // Nothing typed: the microphone takes the send button's place, as on iOS (not while editing).
  const mic = empty && state.compose?.mode !== "edit";
  $("send").hidden = mic;
  $("mic").hidden = !mic;
}
function sendDraft() {
  const text = draft.value.trim();
  if (!text || !state.open) return;
  const compose = state.compose;
  if (compose?.mode === "edit") {
    if (text !== compose.message.text) invoke("edit_message", { messageId: compose.message.id, text });
  } else if (compose?.mode === "reply") {
    invoke("reply", { peerId: state.open, text, to: compose.message.id });
  } else {
    invoke("send_message", { peerId: state.open, text });
  }
  state.compose = null;
  $("compose-bar").hidden = true;
  draft.value = "";
  fitDraft();
}
// A file goes to Rust as raw bytes, its name and caption in headers; the client prepares it (a
// photo loses its metadata there), seals and uploads it. The draft, if any, is its caption.
const MAX_FILE_BYTES = 95 * 1024 * 1024;
async function sendFile(file) {
  if (!state.open) return;
  if (file.size > MAX_FILE_BYTES) return toast(t("tooLarge", { name: file.name }), true);
  const caption = state.compose?.mode === "edit" ? "" : draft.value.trim();
  if (caption) { draft.value = ""; fitDraft(); }
  toast(t("sending", { name: file.name }));
  const bytes = new Uint8Array(await file.arrayBuffer());
  invoke("send_file", bytes, {
    headers: {
      "x-peer": state.open,
      "x-name": encodeURIComponent(file.name),
      "x-caption": encodeURIComponent(caption),
    },
  }).catch((e) => toast(String(e), true));
}
for (const input of [$("attach-file"), $("attach-photo")]) {
  input.addEventListener("change", (ev) => {
    const [file] = ev.target.files;
    ev.target.value = "";
    if (file) sendFile(file);
  });
}

// ── Stickers ────────────────────────────────────────────────────────────────
// A message names a sticker; its picture comes from the pack, which the client holds, checks
// and fetches. The page only asks for pictures and shows the emoji until one comes.

const stickerImages = new Map(); // "packId:index" → data URI | null (none to be had) | undefined (asked)
const stickerKey = (s) => `${s.packId}:${s.index}`;
const picker = { packs: [], recent: [], catalog: null, catalogError: null };

function stickerArt(s) {
  const art = el("span", "sticker-art");
  art.dataset.sticker = stickerKey(s);
  art.setAttribute("aria-label", t("sticker", { emoji: s.emoji }));
  drawSticker(art, s);
  return art;
}

function drawSticker(art, s) {
  const key = stickerKey(s);
  const url = stickerImages.get(key);
  if (url) {
    const img = document.createElement("img");
    img.src = url;
    img.alt = s.emoji;
    img.draggable = false;
    art.replaceChildren(img);
    return;
  }
  art.replaceChildren(el("span", "emoji", s.emoji));
  if (!stickerImages.has(key)) {
    stickerImages.set(key, undefined);
    invoke("load_sticker", { packId: s.packId, index: s.index, emoji: s.emoji }).catch(() => {});
  }
}

function onStickerImage(e) {
  const key = stickerKey(e);
  stickerImages.set(key, e.url ?? null);
  if (!e.url) return;
  for (const art of document.querySelectorAll(".sticker-art")) {
    if (art.dataset.sticker !== key) continue;
    const img = document.createElement("img");
    img.src = e.url;
    img.alt = art.querySelector(".emoji")?.textContent ?? "";
    img.draggable = false;
    art.replaceChildren(img);
  }
}

function openPicker() {
  const panel = $("picker");
  const at = $("attach").getBoundingClientRect();
  panel.style.left = `${Math.max(16, at.left)}px`;
  panel.style.bottom = `${window.innerHeight - at.top + 8}px`;
  panel.hidden = false;
  $("attach").setAttribute("aria-expanded", "true");
  renderPicker();
  invoke("load_stickers").catch(() => {});
  if (!picker.catalog) invoke("load_sticker_catalog").catch(() => {});
}

function closePicker() {
  $("picker").hidden = true;
  $("attach").setAttribute("aria-expanded", "false");
}

function stickerButton(s) {
  const b = el("button");
  b.type = "button";
  b.title = s.emoji;
  b.append(stickerArt(s));
  b.addEventListener("click", () => {
    if (!state.open) return;
    closePicker();
    invoke("send_sticker", { peerId: state.open, packId: s.packId, index: s.index })
      .catch((e) => toast(String(e), true));
  });
  return b;
}

function stickerSection(title, stickers, action = null) {
  const head = el("h4", "", title);
  if (action) head.append(action);
  const grid = el("div", "sticker-grid");
  grid.append(...stickers.map(stickerButton));
  return [head, grid];
}

function renderPicker() {
  const body = $("picker-stickers");
  const nodes = [];
  if (picker.recent.length > 0) nodes.push(...stickerSection(t("recentStickers"), picker.recent));
  for (const pack of picker.packs) {
    const remove = el("button", "link", t("removePack"));
    remove.type = "button";
    remove.addEventListener("click", (ev) =>
      confirmTwice(ev.currentTarget, "confirmRemovePack", () => invoke("remove_sticker_pack", { packId: pack.packId })));
    nodes.push(...stickerSection(pack.title, pack.stickers, remove));
  }
  if (nodes.length === 0) nodes.push(el("p", "dim small empty-note", t("noStickers")));
  const offered = picker.catalog ?? [];
  if (offered.length > 0 || picker.catalogError) nodes.push(el("h4", "", t("morePacks")));
  for (const pack of offered) {
    const row = el("div", "catalog-row");
    const what = el("div", "what");
    what.append(
      el("span", "", pack.title),
      el("span", "small dim", t("packInfo", { publisher: pack.publisher, count: pack.count, size: bytes(pack.bytes) })),
    );
    const add = el("button", "secondary", t("addPack"));
    add.type = "button";
    add.addEventListener("click", () => {
      add.disabled = true;
      add.textContent = t("loading");
      invoke("add_sticker_pack", { packId: pack.packId }).catch((e) => toast(String(e), true));
    });
    row.append(what, add);
    nodes.push(row);
  }
  if (picker.catalogError) {
    const retry = el("button", "link", t("retry"));
    retry.type = "button";
    retry.addEventListener("click", () => {
      picker.catalogError = null;
      renderPicker();
      invoke("load_sticker_catalog").catch(() => {});
    });
    const row = el("p", "small dim", `${t("catalogFailed")} `);
    row.append(retry);
    nodes.push(row);
  }
  body.replaceChildren(...nodes);
}

function onStickers(e) {
  picker.packs = e.packs;
  picker.recent = e.recent;
  // A pack just added leaves the catalog.
  if (picker.catalog) picker.catalog = picker.catalog.filter((c) => !e.packs.some((p) => p.packId === c.packId));
  if (!$("picker").hidden) renderPicker();
}

function onStickerCatalog(e) {
  picker.catalog = e.error ? null : e.packs;
  picker.catalogError = e.error ?? null;
  if (!$("picker").hidden) renderPicker();
}

$("attach").addEventListener("click", (ev) => {
  ev.stopPropagation();
  if ($("picker").hidden) openPicker();
  else closePicker();
});
$("picker-photo").addEventListener("click", () => { closePicker(); $("attach-photo").click(); });
$("picker-file").addEventListener("click", () => { closePicker(); $("attach-file").click(); });
document.addEventListener("click", (ev) => {
  if (!$("picker").hidden && !ev.target.closest("#picker") && !ev.target.closest("#attach")) closePicker();
});
$("picker").addEventListener("keydown", (ev) => {
  if (ev.key === "Escape") {
    ev.stopPropagation();
    closePicker();
    $("attach").focus();
  }
});
window.addEventListener("resize", closePicker);

draft.addEventListener("input", fitDraft);
draft.addEventListener("keydown", (ev) => {
  // Enter sends, or with the setting off Ctrl+Enter does and Enter breaks the line.
  const sends = flag("sendOnEnter") ? !ev.shiftKey && !ev.ctrlKey && !ev.metaKey : ev.ctrlKey || ev.metaKey;
  if (ev.key === "Enter" && sends && !ev.isComposing && ev.keyCode !== 229) {
    ev.preventDefault();
    sendDraft();
  } else if (ev.key === "Escape" && state.compose) {
    ev.preventDefault();
    ev.stopPropagation();
    cancelCompose();
  } else if (ev.key === "ArrowUp" && draft.value === "" && !state.compose) {
    // The last message of ours, to edit — as in other messengers.
    const mine = (state.messages.get(state.open) ?? []).filter((m) => m.sent && !m.attachment);
    if (mine.length > 0) {
      ev.preventDefault();
      startCompose("edit", mine[mine.length - 1]);
    }
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
  const chat = state.chats.find((c) => c.peerId === peer);
  $("info-share").checked = Boolean(chat?.sharing);
  $("alias-input").value = chat?.alias ?? "";
  $("alias-input").placeholder = t("aliasPlaceholder");
  $("safety").replaceChildren();
  invoke("safety_number", { peerId: peer });
  syncInfoBlock();
  $("dlg-info").showModal();
}
/** The block button and its hint, as the open contact stands. */
function syncInfoBlock() {
  const chat = state.chats.find((c) => c.peerId === state.open);
  $("info-block").textContent = t(chat?.blocked ? "unblockUser" : "blockUser");
  $("info-block-hint").textContent = chat?.blocked ? "" : t("blockHint", { name: chatName(state.open) });
}
$("info-block").addEventListener("click", (ev) => {
  const chat = state.chats.find((c) => c.peerId === state.open);
  if (!chat) return;
  if (chat.blocked) invoke("set_blocked", { peerId: chat.peerId, blocked: false });
  else confirmTwice(ev.currentTarget, "confirmBlock", () => invoke("set_blocked", { peerId: chat.peerId, blocked: true }));
});
$("info-delete").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmDelete", () => {
    invoke("delete_contact", { peerId: state.open });
    $("dlg-info").close();
    closeChat();
  }),
);

// Choices kept in this window (the theme, text, language, notifications); the account's own
// (username, devices, receipts) come from the client as a `settings` event.
const PREF_DEFAULTS = {
  textSize: "standard", lang: "auto", preview: "nameAndMessage",
  sendOnEnter: "1", showTimes: "1", notify: "1",
};
const pref = (k) => store.get(k) ?? PREF_DEFAULTS[k];
const flag = (k) => pref(k) === "1";

function applyPrefs() {
  document.documentElement.dataset.textSize = pref("textSize");
  document.documentElement.dataset.times = flag("showTimes") ? "on" : "off";
}

// Rust shows the notifications; it is told what they may say, in this page's language.
function sendNotificationPrefs() {
  invoke("set_notifications", {
    settings: { enabled: flag("notify"), preview: pref("preview"), newMessage: t("newMessage") },
  }).catch(() => {});
}

function openSettings(pane = "profile") {
  $("set-user").textContent = state.account?.userId ?? "";
  $("set-device").textContent = state.account?.deviceId ?? "";
  $("set-server").textContent = state.account?.server ?? "";
  $("set-version").textContent = state.account?.version ?? "";
  $("set-account-id").textContent = state.account?.userId ?? "";
  renderConnection();

  renderLogs();
  renderUpdate();
  renderRecovery();
  markTheme();
  markPrefs();
  renderAccountSettings();
  renderLock();
  invoke("load_settings");
  showPane(pane);
  $("dlg-settings").showModal();
}

// ── Diagnostics ─────────────────────────────────────────────────────────────

function renderLogs() {
  invoke("logs_info").then((info) => {
    $("set-logs").textContent = info.path;
    $("logs-size").textContent = bytes(info.bytes);
  }).catch(() => {});
  invoke("logs_recent").then((text) => {
    const tail = $("logs-recent");
    tail.textContent = text || t("logsEmpty");
    tail.scrollTop = tail.scrollHeight; // the newest at the bottom, in view
  }).catch(() => {});
}

// Also before sign-in, from the first screens: a device that cannot get in is when they matter.
for (const b of document.querySelectorAll(".save-logs")) {
  b.addEventListener("click", () =>
    invoke("save_logs")
      .then((name) => toast(t("logsSaved", { name }), false, () => invoke("open_downloads").catch((e) => toast(String(e), true))))
      .catch((e) => toast(String(e), true)),
  );
}

$("clear-logs").addEventListener("click", () =>
  invoke("clear_logs").then(() => { toast(t("logsCleared")); renderLogs(); }).catch((e) => toast(String(e), true)),
);

// ── Updates ─────────────────────────────────────────────────────────────────
// Rust checks the feed and installs; the page shows where it stands (bridge.rs `Update`).

let update = null;
let updateToldFor = null;

function onUpdate(e) {
  update = e;
  renderUpdate();
  // Once per version found, wherever the person is.
  if (e.state === "available" && e.version !== updateToldFor) {
    updateToldFor = e.version;
    toast(t("updateAvailable", { version: e.version }), false, () => openSettings("about"));
  }
}

function renderUpdate() {
  const e = update ?? { state: "unknown", install: "itself" };
  const line = {
    unknown: () => t("updateUnknown"),
    checking: () => t("updateChecking"),
    current: () => t("updateCurrent"),
    installing: () => t("updateInstalling", { version: e.version }),
    failed: () => t("updateFailed", { error: e.error ?? "" }),
    available: () => {
      const how = { deb: "updateDeb", rpm: "updateRpm", manual: "updateManual" }[e.install];
      const found = t("updateAvailable", { version: e.version });
      return how ? `${found} ${t(how, { version: e.version })}` : found;
    },
  }[e.state] ?? (() => "");
  $("update-state").textContent = line();
  $("update-state").classList.toggle("error", e.state === "failed");
  const available = e.state === "available";
  $("update-install").hidden = !(available && e.install === "itself");
  $("update-page").hidden = !(available && (e.install === "deb" || e.install === "rpm"));
  $("update-check").disabled = e.state === "checking" || e.state === "installing";
}

$("update-check").addEventListener("click", () => {
  onUpdate({ ...(update ?? { install: "itself" }), state: "checking" });
  invoke("check_update").catch((err) => toast(String(err), true));
});
$("update-install").addEventListener("click", () =>
  invoke("install_update").catch((err) => toast(String(err), true)),
);
$("update-page").addEventListener("click", () =>
  invoke("open_release_page").catch((err) => toast(String(err), true)),
);

// ── Voice notes ─────────────────────────────────────────────────────────────
// Rust records and encodes (voice.rs); the page shows the bar and plays what Rust decoded. As on
// iOS: a tap on the microphone records, the bars show how loud it is; a note plays with its
// waveform, seeks where the bars are let go, and plays at 1, 1.25, 1.5 or 2× — the speed kept.

let recording = null; // {peerId} while a note is recorded
const LIVE_BARS = 40;
const SPEEDS = [1, 1.25, 1.5, 2];
let voiceSpeed = SPEEDS.includes(Number(store.get("voiceSpeed"))) ? Number(store.get("voiceSpeed")) : 1;
const players = new Map(); // mediaKey → HTMLAudioElement
let wantPlay = null; // mediaKey asked to play before its sound was here

$("mic").addEventListener("click", () => {
  if (!state.open || recording) return;
  recording = { peerId: state.open };
  invoke("start_voice", { peerId: state.open }).catch((err) => {
    recording = null;
    toast(String(err), true);
  });
});
$("voice-cancel").addEventListener("click", () => stopRecording(false));
$("voice-send").addEventListener("click", () => stopRecording(true));
document.addEventListener("keydown", (ev) => {
  if (ev.key === "Escape" && recording) {
    ev.preventDefault();
    ev.stopPropagation();
    stopRecording(false);
  }
}, true);

function stopRecording(send) {
  invoke("stop_voice", { send }).catch((err) => toast(String(err), true));
}

function onRecording(e) {
  const live = $("voice-live");
  if (!e.recording) {
    recording = null;
    $("recorder").hidden = true;
    $("composer").hidden = false;
    live.replaceChildren();
    fitDraft();
    return;
  }
  recording = { peerId: e.peerId };
  if (e.peerId !== state.open) return;
  $("composer").hidden = true;
  $("recorder").hidden = false;
  $("voice-time").textContent = duration(e.elapsedMs);
  const bar = el("span");
  bar.style.height = `${Math.max(2, Math.round(e.level * 28))}px`;
  live.append(bar);
  while (live.children.length > LIVE_BARS) live.firstChild.remove();
}

/** `bars` (0..=255) as at most `n` bars, each the mean of those it stands for. */
function fewerBars(bars, n) {
  if (!bars?.length) return new Array(n).fill(64);
  if (bars.length <= n) return bars;
  const step = bars.length / n;
  return Array.from({ length: n }, (_, i) => {
    const slice = bars.slice(Math.floor(i * step), Math.max(Math.floor((i + 1) * step), Math.floor(i * step) + 1));
    return slice.reduce((a, b) => a + b, 0) / slice.length;
  });
}

function voiceNode(m, item, index, entry) {
  const key = mediaKey(m.id, index);
  const box = el("div", "voice");
  const button = el("button", "voice-play");
  button.type = "button";
  const bars = el("div", "voice-bars");
  const heights = fewerBars(item.waveform, 48);
  for (const v of heights) {
    const bar = el("span");
    bar.style.height = `${Math.max(2, Math.round((v / 255) * 28))}px`;
    bars.append(bar);
  }
  const side = el("div", "voice-side");
  const time = el("span", "voice-time");
  const speed = el("button", "voice-speed", `${voiceSpeed}×`);
  speed.type = "button";
  speed.setAttribute("aria-label", t("voiceSpeed"));
  side.append(time, speed);
  box.append(button, bars, side);

  let audio = players.get(key);
  if (!audio && entry?.url) {
    audio = new Audio(entry.url);
    audio.playbackRate = voiceSpeed;
    players.set(key, audio);
  }
  const total = () => (audio && Number.isFinite(audio.duration) ? audio.duration * 1000 : item.durationMs);
  const draw = (fraction = audio && total() ? (audio.currentTime * 1000) / total() : 0) => {
    const lit = Math.round(fraction * heights.length);
    bars.childNodes.forEach((b, i) => b.classList.toggle("played", i < lit));
    const playing = audio && !audio.paused;
    button.textContent = entry?.loading ? "…" : playing ? "❚❚" : "▶";
    button.setAttribute("aria-label", playing ? t("pause") : t("play"));
    time.textContent = duration(playing || fraction > 0 ? fraction * total() : total());
  };
  if (audio) {
    audio.ontimeupdate = () => draw();
    audio.onpause = () => draw();
    audio.onplay = () => draw();
    audio.onended = () => { audio.currentTime = 0; draw(0); };
  }
  const start = () => {
    for (const [k, other] of players) if (k !== key) other.pause();
    audio.play().catch((err) => toast(String(err), true));
  };
  button.addEventListener("click", () => {
    if (!audio) {
      wantPlay = key;
      loadMedia(m, index);
      redrawMedia(m.id, index);
      return;
    }
    if (audio.paused) start();
    else audio.pause();
  });
  speed.addEventListener("click", () => {
    voiceSpeed = SPEEDS[(SPEEDS.indexOf(voiceSpeed) + 1) % SPEEDS.length];
    store.set("voiceSpeed", String(voiceSpeed));
    for (const p of players.values()) p.playbackRate = voiceSpeed;
    for (const b of document.querySelectorAll(".voice-speed")) b.textContent = `${voiceSpeed}×`;
  });
  // Seeking: the bars follow the pointer, the sound moves where it is let go; paused stays paused.
  const at = (ev) => {
    const r = bars.getBoundingClientRect();
    return Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width));
  };
  bars.addEventListener("pointerdown", (ev) => {
    if (!audio) return;
    bars.setPointerCapture(ev.pointerId);
    draw(at(ev));
    const move = (e) => draw(at(e));
    const up = (e) => {
      bars.removeEventListener("pointermove", move);
      bars.removeEventListener("pointerup", up);
      if (Number.isFinite(audio.duration)) audio.currentTime = at(e) * audio.duration;
      draw();
    };
    bars.addEventListener("pointermove", move);
    bars.addEventListener("pointerup", up);
  });
  draw();
  // Asked to play before the sound was fetched: it plays once it is here.
  if (audio && wantPlay === key) {
    wantPlay = null;
    start();
  }
  return box;
}

// ── VEIL ────────────────────────────────────────────────────────────────────

function onVeil(e) {
  for (const b of $("veil-mode").querySelectorAll("[data-value]")) {
    b.setAttribute("aria-selected", String(b.dataset.value === e.mode));
  }
  const states = { off: "veilStateOff", connecting: "veilStateConnecting", up: "veilStateUp", noFront: "veilStateNoFront" };
  const line = t(states[e.state] ?? "veilStateOff", { front: e.front ?? "" });
  $("veil-state").textContent = `${line} ${t("veilFronts", { n: e.fronts })}`;
  $("veil-state").classList.toggle("error", e.state === "noFront");
  // The same, on the first screen before there is an account.
  const start = $("veil-start-state");
  const through = e.mode !== "off";
  start.hidden = !through;
  start.textContent = line;
  start.classList.toggle("error", e.state === "noFront");
  $("veil-start-off").hidden = !through;
}

function onVeilVoucher(e) {
  $("veil-voucher").disabled = false;
  if (e.error) return toast(e.error, true);
  $("veil-voucher-link").value = e.link;
  $("veil-voucher-exp").textContent = t("veilVoucherExp", { time: clock(e.exp * 1000) });
  $("veil-voucher-out").hidden = false;
}

$("veil-start-open").addEventListener("click", () => {
  $("veil-start").hidden = false;
  $("veil-start-open").hidden = true;
  $("veil-start-link").focus();
});
$("veil-start-connect").addEventListener("click", () => {
  const link = $("veil-start-link").value.trim();
  if (!link) return;
  invoke("import_veil_link", { link }).then(() => ($("veil-start-link").value = "")).catch((e) => toast(String(e), true));
});
$("veil-start-off").addEventListener("click", () => invoke("set_veil", { mode: "off" }).catch(() => {}));

for (const b of $("veil-mode").querySelectorAll("[data-value]")) {
  b.addEventListener("click", () => invoke("set_veil", { mode: b.dataset.value }));
}
$("veil-import").addEventListener("click", () => {
  const link = $("veil-link").value.trim();
  if (!link) return;
  invoke("import_veil_link", { link }).then(() => ($("veil-link").value = "")).catch((e) => toast(String(e), true));
});
$("veil-voucher").addEventListener("click", (ev) => {
  ev.currentTarget.disabled = true;
  invoke("issue_veil_voucher").catch((e) => toast(String(e), true));
});
$("veil-voucher-copy").addEventListener("click", async () => {
  await navigator.clipboard.writeText($("veil-voucher-link").value);
  toast(t("copied"));
});

function showPane(name) {
  for (const p of document.querySelectorAll(".settings-pane")) p.hidden = p.dataset.pane !== name;
  for (const b of $("settings-nav").querySelectorAll("[data-pane]")) b.setAttribute("aria-current", String(b.dataset.pane === name));
  if (name === "diagnostics") renderLogs();
  if (name === "veil") invoke("load_veil").catch(() => {});
}
$("settings-nav").addEventListener("click", (ev) => {
  const b = ev.target.closest("[data-pane]");
  if (b) showPane(b.dataset.pane);
});

function markPrefs() {
  for (const group of document.querySelectorAll("[data-pref]")) {
    for (const b of group.querySelectorAll("[data-value]")) b.setAttribute("aria-selected", String(b.dataset.value === pref(group.dataset.pref)));
  }
  for (const box of document.querySelectorAll("[data-pref-flag]")) box.checked = flag(box.dataset.prefFlag);
  document.querySelector('[data-pref="preview"]').classList.toggle("off", !flag("notify"));
}
for (const group of document.querySelectorAll("[data-pref]")) {
  group.addEventListener("click", (ev) => {
    const b = ev.target.closest("[data-value]");
    if (!b) return;
    const key = group.dataset.pref;
    if (pref(key) === b.dataset.value) return;
    store.set(key, b.dataset.value);
    if (key === "lang") return location.reload();
    applyPrefs();
    markPrefs();
    if (key === "preview") sendNotificationPrefs();
  });
}
for (const box of document.querySelectorAll("[data-pref-flag]")) {
  box.addEventListener("change", () => {
    store.set(box.dataset.prefFlag, box.checked ? "1" : "0");
    applyPrefs();
    markPrefs();
    if (box.dataset.prefFlag === "notify") sendNotificationPrefs();
  });
}

// ── The account's settings ──────────────────────────────────────────────────

function onSettings(e) {
  state.settings = e;
  renderAccountSettings();
}

function renderAccountSettings() {
  const s = state.settings;
  const name = s?.username;
  const own = s?.displayName || state.account?.name || "";
  $("set-name").textContent = own;
  $("set-avatar").replaceChildren(avatar(state.account?.userId ?? "", false, s?.avatar ?? undefined));
  $("set-avatar").firstChild.classList.add("large");
  $("avatar-remove").hidden = !s?.avatar;
  if (document.activeElement !== $("name-input")) $("name-input").value = s?.displayName ?? "";
  $("set-username").textContent = name ? `@${name}` : t("noUsername");
  if (document.activeElement !== $("username-input")) $("username-input").value = name ?? "";
  const discoverable = $("set-discoverable");
  discoverable.checked = Boolean(s?.discoverable);
  discoverable.disabled = !s || !name;
  $("discoverable-hint").textContent = s && !name ? t("discoverableNeedsName") : t("discoverableHint");
  if (s && document.activeElement !== $("device-name-input")) $("device-name-input").value = s.deviceName;
  $("set-receipts").checked = s ? s.deliveryReceipts : true;
  $("set-receipts").disabled = !s;
  $("set-fingerprint").textContent = s?.fingerprint ?? "";
  renderStorage();
  renderInvites();

  const list = $("set-devices");
  if (!s) return list.replaceChildren(el("li", "dim", t("loadingDevices")));
  const devices = [...s.devices].sort((a, b) => Number(b.current) - Number(a.current) || a.created - b.created);
  list.replaceChildren(...devices.map((d) => {
    const li = el("li");
    const label = el("div", "device-label");
    label.append(el("span", "", d.name || t("deviceName", { id: d.id.slice(0, 8) })));
    const facts = [d.current ? t("thisDevice") : "", d.primary ? t("mainDevice") : "",
      d.platform ? t(`platform_${d.platform}`) : "", d.name ? d.id.slice(0, 8) : ""].filter(Boolean).join(" · ");
    if (facts) label.append(el("span", "device-badge", facts));
    if (d.created) label.append(el("span", "small dim", t("deviceAdded", { date: new Date(d.created).toLocaleDateString(LANG) })));
    li.append(label);
    if (!d.current) {
      const nearby = el("button", "link", t("historyNearbySend"));
      nearby.type = "button";
      nearby.addEventListener("click", () => {
        showNearby("waiting");
        invoke("offer_history_nearby", { deviceId: d.id });
      });
      li.append(nearby);
      const history = el("button", "link", t("historyExport"));
      history.type = "button";
      history.addEventListener("click", () => {
        toast(t("historyWriting"));
        invoke("export_history", { deviceId: d.id });
      });
      li.append(history);
    }
    if (!d.current && !d.primary) {
      const revoke = el("button", "danger-link", t("revoke"));
      revoke.type = "button";
      revoke.addEventListener("click", (ev) =>
        confirmTwice(ev.currentTarget, "confirmRevoke", () => invoke("revoke_devices", { deviceId: d.id })));
      li.append(revoke);
    }
    return li;
  }));
  $("revoke-others").hidden = !devices.some((d) => !d.current && !d.primary);
}

$("alias-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  if (!state.open) return;
  invoke("set_alias", { peerId: state.open, alias: $("alias-input").value.trim() });
});
$("device-name-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const name = $("device-name-input").value.trim();
  if (name && name !== state.settings?.deviceName) invoke("rename_device", { name });
});

$("info-share").addEventListener("change", (ev) => {
  if (state.open) invoke("share_profile", { peerId: state.open, on: ev.target.checked });
});

$("name-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const name = $("name-input").value.trim();
  if (name !== (state.settings?.displayName ?? "")) invoke("set_display_name", { name });
});

// The picture is made here, as a square JPEG of at most 512 px and 200 KB, the size iOS sends:
// drawing it on a canvas also leaves its metadata behind. The client checks what it gets.
const AVATAR_PX = 512;
const AVATAR_MAX_BYTES = 200 * 1024;
async function avatarJpeg(file) {
  const url = await new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
  const img = await new Promise((resolve, reject) => {
    const i = new Image();
    i.onload = () => resolve(i);
    i.onerror = reject;
    i.src = url;
  });
  const side = Math.min(img.naturalWidth, img.naturalHeight);
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = Math.min(AVATAR_PX, side);
  canvas.getContext("2d").drawImage(img, (img.naturalWidth - side) / 2, (img.naturalHeight - side) / 2,
    side, side, 0, 0, canvas.width, canvas.height);
  for (let q = 0.8; q > 0.05; q -= 0.1) {
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, "image/jpeg", q));
    if (blob && blob.size <= AVATAR_MAX_BYTES) return new Uint8Array(await blob.arrayBuffer());
  }
  throw new Error("too large");
}
$("avatar-choose").addEventListener("click", () => $("avatar-file").click());
$("avatar-file").addEventListener("change", async (ev) => {
  const [file] = ev.target.files;
  ev.target.value = "";
  if (!file) return;
  try {
    await invoke("set_avatar", await avatarJpeg(file));
    toast(t("photoSending"));
  } catch {
    toast(t("photoUnreadable"), true);
  }
});
$("avatar-remove").addEventListener("click", () => invoke("remove_avatar"));

$("username-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const name = $("username-input").value.trim();
  if (name && name !== state.settings?.username) invoke("change_username", { username: name });
});
$("set-discoverable").addEventListener("change", (ev) => invoke("set_discoverable", { on: ev.target.checked }));
$("set-receipts").addEventListener("change", (ev) => invoke("set_delivery_receipts", { on: ev.target.checked }));
$("revoke-others").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmRevokeOthers", () => invoke("revoke_devices", { deviceId: null })));

window.addEventListener("focus", () => invoke("set_focus", { focused: true }).catch(() => {}));
window.addEventListener("blur", () => invoke("set_focus", { focused: false }).catch(() => {}));

$("sign-out").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmSignOut", () => {
    $("dlg-settings").close();
    invoke("logout");
  }),
);

// ── Data and storage ────────────────────────────────────────────────────────

function bytes(n) {
  const units = LANG === "ru" ? ["Б", "КБ", "МБ", "ГБ"] : ["B", "KB", "MB", "GB"];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${i && n < 10 && n % 1 ? n.toFixed(1) : Math.round(n)} ${units[i]}`;
}

function renderStorage() {
  const s = state.settings;
  $("cache-clear").disabled = !s || !s.mediaFiles;
  if (!s) return ($("cache-usage").textContent = "");
  const limit = s.cacheLimit;
  $("cache-usage").textContent = limit
    ? t("cacheUsageOf", { size: bytes(s.mediaBytes), limit: bytes(limit), n: s.mediaFiles })
    : t("cacheUsage", { size: bytes(s.mediaBytes), n: s.mediaFiles });
  const fraction = limit ? Math.min(s.mediaBytes / limit, 1) : 0;
  $("cache-meter").hidden = !limit;
  $("cache-meter").firstElementChild.style.width = `${(fraction * 100).toFixed(1)}%`;
  $("cache-meter").classList.toggle("full", fraction >= 0.85);
  const chosen = { autoDownload: s.autoDownload, cacheLimit: String(limit ?? 0), keepDays: String(s.keepDays ?? 0) };
  for (const group of document.querySelectorAll("[data-media]")) {
    for (const b of group.querySelectorAll("[data-value]")) {
      b.setAttribute("aria-selected", String(b.dataset.value === chosen[group.dataset.media]));
    }
  }
}

for (const b of document.querySelectorAll("[data-bytes]")) b.textContent = bytes(Number(b.dataset.value));
for (const group of document.querySelectorAll("[data-media]")) {
  group.addEventListener("click", (ev) => {
    const b = ev.target.closest("[data-value]");
    const s = state.settings;
    if (!b || !s) return;
    const prefs = { autoDownload: s.autoDownload, cacheLimit: s.cacheLimit ?? 0, keepDays: s.keepDays ?? 0 };
    const key = group.dataset.media;
    prefs[key] = key === "autoDownload" ? b.dataset.value : Number(b.dataset.value);
    invoke("set_media_prefs", prefs);
  });
}
$("cache-clear").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmClearCache", () => invoke("clear_media_cache")));

for (const id of ["set-account-id", "set-fingerprint"]) {
  $(id).addEventListener("click", async (ev) => {
    const text = ev.currentTarget.textContent;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      toast(t("copied"));
    } catch {}
  });
}

function renderConnection() {
  $("set-connection").textContent = t(state.connected ? "online" : "offline");
}

// ── The app lock ────────────────────────────────────────────────────────────

// The client words its refusals in English; the two the lock screen shows, in the page's language.
function lockReason(reason) {
  if (reason === "Wrong code") return t("lockWrong");
  const wait = /^Too many wrong codes: wait (\d+) s$/.exec(reason);
  return wait ? t("lockWait", { n: wait[1] }) : reason;
}

// Rust stops sending the account while locked; what the page already shows goes here.
function onLocked() {
  state.lock.locked = true;
  for (const d of document.querySelectorAll("dialog[open]")) d.close();
  if (state.open) closeChat();
  state.messages.clear();
  state.chats = [];
  state.settings = null;
  renderChats();
  $("lock-error").hidden = true;
  $("lock-code").value = "";
  $("lock-screen").hidden = false;
  $("lock-code").focus();
}
function onUnlocked() {
  state.lock.locked = false;
  $("lock-screen").hidden = true;
  $("lock-code").value = "";
}
$("lock-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const code = $("lock-code").value;
  if (code) invoke("unlock_app", { code });
});
$("lock-forgot").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmLockForgot", () => invoke("logout")));

function renderLock() {
  const on = state.lock.enabled;
  $("lock-status").textContent = t(on ? "lockOn" : "lockOffStatus");
  for (const b of $("lock-delay").querySelectorAll("[data-value]")) {
    b.setAttribute("aria-selected", String(Number(b.dataset.value) === (on ? state.lock.delaySecs : pendingDelay)));
  }
  $("lock-current").hidden = !on;
  $("lock-save").textContent = t(on ? "lockChange" : "lockEnable");
  $("lock-now").hidden = !on;
  $("lock-off").hidden = !on;
}
// The delay chosen before a code is set; with one, a choice is kept at once.
let pendingDelay = 300;
$("lock-delay").addEventListener("click", (ev) => {
  const b = ev.target.closest("[data-value]");
  if (!b) return;
  const delaySecs = Number(b.dataset.value);
  if (!state.lock.enabled) {
    pendingDelay = delaySecs;
    return renderLock();
  }
  invoke("set_app_lock", { current: null, code: null, remove: false, delaySecs });
});
function clearLockFields() {
  for (const id of ["lock-current", "lock-new", "lock-repeat"]) $(id).value = "";
}
$("lock-set-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const on = state.lock.enabled;
  const current = $("lock-current").value;
  const code = $("lock-new").value;
  if (on && !current) return toast(t("lockNeedCurrent"), true);
  if ([...code].length < 4) return toast(t("lockShort"), true);
  if (code !== $("lock-repeat").value) return toast(t("lockMismatch"), true);
  invoke("set_app_lock", {
    current: on ? current : null, code, remove: false,
    delaySecs: on ? state.lock.delaySecs : pendingDelay,
  });
  clearLockFields();
});
$("lock-off").addEventListener("click", () => {
  const current = $("lock-current").value;
  if (!current) return toast(t("lockNeedCurrent"), true);
  invoke("set_app_lock", { current, code: null, remove: true, delaySecs: 0 });
  clearLockFields();
});
$("lock-now").addEventListener("click", () => invoke("lock_now"));

// ── Issued invites ──────────────────────────────────────────────────────────

function renderInvites() {
  const list = $("set-invites");
  const invites = state.settings?.invites ?? [];
  $("revoke-invites").hidden = invites.length < 2;
  if (!invites.length) return list.replaceChildren(el("li", "dim", t("noInvites")));
  list.replaceChildren(...invites.map((i) => {
    const li = el("li");
    const label = el("div", "device-label");
    const left = Math.max(0, Math.round((i.expires - Date.now()) / 60000));
    label.append(el("span", "", t(`inviteKind_${i.kind}`)),
      el("span", "small dim", `${new Date(i.issued).toLocaleTimeString(LANG, { hour: "2-digit", minute: "2-digit" })} · ${t("inviteLeft", { time: `${left < 1 ? "<1" : left} ${t("minutesShort")}` })}`));
    const revoke = el("button", "danger-link", t("revokeInvite"));
    revoke.type = "button";
    revoke.addEventListener("click", (ev) =>
      confirmTwice(ev.currentTarget, "confirmRevokeInvite", () => invoke("revoke_invites", { jtis: [i.jti] })));
    li.append(label, revoke);
    return li;
  }));
}
$("revoke-invites").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmRevokeInvite", () =>
    invoke("revoke_invites", { jtis: (state.settings?.invites ?? []).map((i) => i.jti) })));

// ── History transfer by file ────────────────────────────────────────────────

$("history-import").addEventListener("click", () => $("history-file").click());
$("history-file").addEventListener("change", async (ev) => {
  const [file] = ev.target.files;
  ev.target.value = "";
  if (!file) return;
  toast(t("historyReading"));
  const bytes = new Uint8Array(await file.arrayBuffer());
  invoke("import_history", bytes).catch((e) => toast(String(e), true));
});

// ── History transfer over the local network ─────────────────────────────────

/// The status line of a transfer under way; `null` hides it.
function showNearby(state) {
  const line = $("history-nearby");
  line.hidden = state === null;
  if (state === null) return;
  $("history-nearby-text").textContent = {
    waiting: t("historyNearbyWaiting_send"),
    moving: t("historyNearbyMoving"),
    receiving: t("historyNearbyReceiving"),
  }[state];
}

function onHistoryNearby(e) {
  if (e.state === "receiving") toast(t("historyNearbyReceiving"));
  if (["waiting", "moving", "receiving"].includes(e.state)) return showNearby(e.state);
  showNearby(null);
  if (e.state === "failed") toast(t("historyFailed", { error: e.error }), true);
  else if (e.state === "skipped") toast(t("historyNearbySkipped"));
  // A receive that is done was announced by its import.
  else if (e.sending) toast(t("historyNearbySent", e));
}

$("history-nearby-cancel").addEventListener("click", () => {
  invoke("cancel_history_nearby");
  showNearby(null);
});

// ── Linking a device from here ──────────────────────────────────────────────

let offerTimer = null;
function startOffer() {
  clearInterval(offerTimer);
  $("offer-qr").hidden = true;
  $("offer-qr").removeAttribute("src");
  $("offer-wait").hidden = false;
  $("offer-status").textContent = "";
  $("offer-error").hidden = true;
  $("offer-retry").hidden = true;
  invoke("offer_link");
}
function stopOffer() {
  clearInterval(offerTimer);
  offerTimer = null;
  // The image carries the code's secret: it does not outlive the dialog.
  $("offer-qr").removeAttribute("src");
  $("offer-qr").hidden = true;
}
function onLinkOffer(e) {
  if (!$("dlg-link-offer").open) return invoke("cancel_link_offer");
  $("offer-qr").src = e.image;
  $("offer-qr").hidden = false;
  $("offer-wait").hidden = true;
  const tick = () => {
    const left = Math.max(0, Math.round((e.expiresAt - Date.now()) / 1000));
    const time = `${Math.floor(left / 60)}:${String(left % 60).padStart(2, "0")}`;
    $("offer-status").textContent = `${t("offerValid", { time })} · ${t("offerWaiting")}`;
  };
  tick();
  clearInterval(offerTimer);
  offerTimer = setInterval(tick, 1000);
}
function onLinkOfferEnded(e) {
  stopOffer();
  if (e.device) {
    $("dlg-link-offer").close();
    return toast(t("deviceLinked", { id: e.device.slice(0, 8) }));
  }
  $("offer-wait").hidden = true;
  $("offer-status").textContent = "";
  $("offer-error").textContent = e.error;
  $("offer-error").hidden = false;
  $("offer-retry").hidden = false;
}
$("offer-link").addEventListener("click", () => {
  $("dlg-link-offer").showModal();
  startOffer();
});
$("offer-retry").addEventListener("click", startOffer);
$("offer-close").addEventListener("click", () => $("dlg-link-offer").close());
$("dlg-link-offer").addEventListener("close", () => {
  stopOffer();
  invoke("cancel_link_offer");
});
$("approve-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const code = $("approve-input").value.trim();
  if (!code) return;
  invoke("approve_device", { code });
  $("approve-input").value = "";
});
$("link-copy").addEventListener("click", async () => {
  if (!state.linkCode) return;
  try {
    await navigator.clipboard.writeText(state.linkCode);
    toast(t("copied"));
  } catch {}
});

// ── Leaving: everywhere, or with the account ────────────────────────────────

$("sign-out-everywhere").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmSignOutEverywhere", () => {
    $("dlg-settings").close();
    invoke("logout_everywhere");
  }),
);

// iOS gives a few seconds to stop a deletion once asked for.
const DELETE_COUNTDOWN = 5;
let deleteTimer = null;
function resetDelete() {
  clearInterval(deleteTimer);
  deleteTimer = null;
  $("delete-go").disabled = false;
  $("delete-go").textContent = t("deleteAccount");
  $("delete-cancel").disabled = false;
}
$("delete-account").addEventListener("click", () => {
  resetDelete();
  $("delete-error").hidden = true;
  $("delete-local").hidden = true;
  $("dlg-delete").showModal();
});
$("delete-go").addEventListener("click", () => {
  if (deleteTimer) return resetDelete();
  let left = DELETE_COUNTDOWN;
  $("delete-go").textContent = t("deletingIn", { n: left });
  deleteTimer = setInterval(() => {
    left -= 1;
    if (left > 0) return ($("delete-go").textContent = t("deletingIn", { n: left }));
    clearInterval(deleteTimer);
    deleteTimer = null;
    $("delete-go").textContent = t("deleting");
    $("delete-go").disabled = true;
    $("delete-cancel").disabled = true;
    $("delete-error").hidden = true;
    invoke("delete_account", { localOnly: false });
  }, 1000);
});
$("delete-cancel").addEventListener("click", () => {
  resetDelete();
  $("dlg-delete").close();
});
$("dlg-delete").addEventListener("cancel", (ev) => {
  if ($("delete-cancel").disabled) ev.preventDefault();
  else resetDelete();
});
$("delete-local").addEventListener("click", (ev) =>
  confirmTwice(ev.currentTarget, "confirmDeleteLocally", () => invoke("delete_account", { localOnly: true })));

function onDeleteFailed(reason) {
  resetDelete();
  $("delete-error").textContent = t("deleteFailed", { reason });
  $("delete-error").hidden = false;
  $("delete-local").hidden = false;
}

$("open-logs").addEventListener("click", () => invoke("open_logs").catch((e) => toast(String(e), true)));

// ── Recovery key ────────────────────────────────────────────────────────────

const RECOVERY_TEXT = {
  copyPending: "recoveryCopyPending", ready: "recoveryReady",
  needsPhrase: "recoveryNeedsPhrase", unknown: "recoveryUnknown",
};

function renderRecovery() {
  $("recovery-status").textContent = t(RECOVERY_TEXT[state.recovery]);
  $("recovery-show").hidden = state.recovery !== "copyPending";
  $("recovery-enter").hidden = state.recovery !== "needsPhrase";
  $("open-settings").classList.toggle("attention", state.recovery === "copyPending");
}

function recoveryPane(name) {
  for (const p of document.querySelectorAll(".recovery-pane")) p.hidden = p.dataset.pane !== name;
}

/** The words leave the page when the dialog closes, whichever way it closes. */
function forgetPhrase() {
  $("recovery-words").replaceChildren();
  $("recovery-questions").replaceChildren();
  $("recovery-phrase").value = "";
  $("recovery-quiz-error").hidden = true;
  $("recovery-enter-error").hidden = true;
}
$("dlg-recovery").addEventListener("close", forgetPhrase);

$("recovery-show").addEventListener("click", () => {
  forgetPhrase();
  invoke("reveal_recovery_phrase");
});
$("recovery-enter").addEventListener("click", () => {
  forgetPhrase();
  recoveryPane("enter");
  $("dlg-recovery").showModal();
  $("recovery-phrase").focus();
});

function showWords(words) {
  $("recovery-words").replaceChildren(...words.map((w) => el("li", "", w)));
  $("recovery-words").dataset.count = String(words.length);
  recoveryPane("words");
  if (!$("dlg-recovery").open) $("dlg-recovery").showModal();
}

$("recovery-written").addEventListener("click", () => {
  const count = Number($("recovery-words").dataset.count) || 12;
  const picks = new Set();
  while (picks.size < 3) picks.add(Math.floor(Math.random() * count));
  $("recovery-questions").replaceChildren(
    ...[...picks].sort((a, b) => a - b).map((i) => {
      const label = el("label");
      const input = el("input", "field mono");
      input.dataset.index = String(i);
      input.autocomplete = "off";
      input.spellcheck = false;
      label.append(el("span", "", t("word", { n: i + 1 })), input);
      return label;
    }),
  );
  // Out of sight while answering.
  $("recovery-words").replaceChildren();
  recoveryPane("quiz");
  $("recovery-questions").querySelector("input").focus();
});
$("recovery-back").addEventListener("click", () => invoke("reveal_recovery_phrase"));
$("recovery-quiz").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const answers = [...$("recovery-questions").querySelectorAll("input")].map((i) => [Number(i.dataset.index), i.value]);
  invoke("confirm_recovery_copy", { answers });
});
$("recovery-enter-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  invoke("enter_recovery_phrase", { phrase: $("recovery-phrase").value });
});

function onRecovery(next) {
  const was = state.recovery;
  state.recovery = next;
  renderRecovery();
  if ($("dlg-recovery").open && next === "ready" && was !== "ready") {
    $("dlg-recovery").close();
    toast(t(was === "copyPending" ? "copyConfirmed" : "phraseAccepted"));
  }
}

function onRecoveryFailed(reason) {
  const quiz = !$("recovery-quiz").hidden && $("dlg-recovery").open;
  const box = quiz ? $("recovery-quiz-error") : $("recovery-enter-error");
  if ($("dlg-recovery").open) {
    box.textContent = quiz ? t("quizWrong") : reason;
    box.hidden = false;
  } else {
    toast(reason, true);
  }
}

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
$("open-settings").addEventListener("click", () => openSettings());
$("chat-info").addEventListener("click", openInfo);

// Shortcuts by physical key, so they work on a Russian layout too (Ctrl+K is not Ctrl+Л).
document.addEventListener("keydown", (ev) => {
  if (state.screen !== "main" || state.lock.locked || document.querySelector("dialog[open]")) return;
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
  } else if (ev.key === "Escape" && !$("msg-menu").hidden) closeMessageMenu();
  else if (ev.key === "Escape" && state.open) closeChat();
});

// ── Events from Rust ────────────────────────────────────────────────────────

function onEvent(e) {
  switch (e.type) {
    case "screen": showScreen(e.screen); break;
    case "account": state.account = e; break;
    case "chats":
      state.chats = e.chats;
      renderChats();
      if (state.open) {
        $("chat-name").textContent = chatName(state.open);
        if ($("dlg-info").open) {
          $("info-name").textContent = chatName(state.open);
          syncInfoBlock();
        }
        renderAlarm();
      }
      break;
    case "history":
      state.messages.set(e.peerId, e.messages);
      if (e.peerId === state.open) renderTranscript();
      break;
    case "message": appendMessage(e.peerId, e.message); break;
    case "messageChanged": replaceMessage(e.peerId, e.message); break;
    case "messageDeleted": removeMessage(e.peerId, e.messageId); break;
    case "delivered": markDelivered(e.messageId); break;
    case "media": onMedia(e); break;
    case "stickers": onStickers(e); break;
    case "veil": onVeil(e); break;
    case "recording": onRecording(e); break;
    case "update": onUpdate(e); break;
    case "veilVoucher": onVeilVoucher(e); break;
    case "stickerImage": onStickerImage(e); break;
    case "stickerCatalog": onStickerCatalog(e); break;
    case "mediaSaved": onMediaSaved(e); break;
    case "connection": {
      const dot = $("connection");
      state.connected = e.connected;
      dot.classList.toggle("online", e.connected);
      dot.title = t(e.connected ? "online" : "offline");
      renderConnection();
      break;
    }
    case "deleteFailed": onDeleteFailed(e.reason); break;
    case "lockSetting":
      state.lock.enabled = e.enabled;
      state.lock.delaySecs = e.delaySecs;
      renderLock();
      break;
    case "locked": onLocked(); break;
    case "unlocked": onUnlocked(); break;
    case "unlockFailed":
      $("lock-error").textContent = lockReason(e.reason);
      $("lock-error").hidden = false;
      $("lock-code").select();
      break;
    case "linkOffer": onLinkOffer(e); break;
    case "historyTransfer":
      if (e.error) toast(t("historyFailed", { error: e.error }), true);
      else if (e.exported) toast(t("historyWritten", e), false, () => invoke("open_downloads"));
      else toast(t("historyImported", e));
      break;
    case "historyNearby": onHistoryNearby(e); break;
    case "linkOfferEnded": onLinkOfferEnded(e); break;
    case "registrationStep": setRegistration(e.done, e.total, e.label); break;
    case "linkCode":
      state.linkCode = e.code;
      $("link-copy").hidden = false;
      $("link-copy-hint").hidden = false;
      $("link-qr").src = e.image;
      $("link-qr").hidden = false;
      $("link-wait").hidden = true;
      $("link-error").hidden = true;
      $("link-retry").hidden = true;
      break;
    case "linkFailed":
      state.linkCode = null;
      $("link-copy").hidden = true;
      $("link-copy-hint").hidden = true;
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
    case "unvouched": toast(t("unvouchedToast", { name: chatName(e.peerId) }), true); break;
    case "recovery": onRecovery(e.state); break;
    case "mailboxGap": onMailboxGap(e.gap); break;
    case "recoveryPhrase": showWords(e.words); break;
    case "recoveryFailed": onRecoveryFailed(e.reason); break;
    case "notice": toast(e.text, e.error); break;
    case "settings": onSettings(e); break;
    case "avatar":
      if (e.url) state.avatars.set(e.peerId, e.url); else state.avatars.delete(e.peerId);
      renderChats();
      if (state.open === e.peerId) $("chat-avatar").replaceChildren(avatar(e.peerId, true));
      break;
  }
}

async function start() {
  applyTheme(store.get("theme") ?? "dark");
  applyPrefs();
  sendNotificationPrefs();
  translatePage();
  const events = new Channel();
  events.onmessage = onEvent;
  await invoke("attach", { onEvent: events });
}

start();
