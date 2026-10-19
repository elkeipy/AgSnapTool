const core = window.__TAURI__.core;
const events = window.__TAURI__.event;
const COLUMN_WIDTH = 232;
const CARD_MIN_HEIGHT = 260;
const HOVER_DELAY_MS = 3000;

const LENS_MIN = 1;
const LENS_MAX = 6;
const LENS_STEP = 0.25;

let shell = null;
let shots = [];
let filtered = [];
let shown = 0;
let anchor = null;
let lensScale = LENS_MIN;
let lensX = 0;
let lensY = 0;
let lensPath = null;
let lensPan = null;
let hoverTimer = 0;
let tipX = 0;
let tipY = 0;
const selected = new Set();

const day = document.querySelector("#day");
const scroller = document.querySelector("#scroller");
const board = document.querySelector("#board");
const error = document.querySelector("#shortcut-error");
const menu = document.querySelector("#menu");
const lens = document.querySelector("#lens");
const lensImage = document.querySelector("#lens-image");
const tip = document.querySelector("#tip");

async function boot() {
  shell = await core.invoke("shell_state");
  applyShell();
  await reload(true);
  await events.listen("gallery-refresh", () => reload(false));
  day.addEventListener("change", () => paint(true));
  scroller.addEventListener("scroll", loadIfNearEnd);
  document.querySelector("#folder").addEventListener("click", () => {
    core.invoke("open_folder");
  });
  document.querySelector("#log").addEventListener("click", () => {
    core.invoke("open_log");
  });
  for (const id of ["control", "shift", "alt"]) {
    document.querySelector("#" + id).addEventListener("change", saveShortcut);
  }
  document.querySelector("#days").addEventListener("change", saveDays);
  document.querySelector("#language").addEventListener("change", saveLanguage);
  document.querySelector("#theme").addEventListener("change", saveTheme);
  document.querySelector("#panel-toggle").addEventListener("click", savePanel);
  document.querySelector("#menu-copy").addEventListener("click", () => {
    hideMenu();
    copySelection();
  });
  document.querySelector("#menu-lens").addEventListener("click", () => {
    hideMenu();
    if (lensPath) {
      openLens(lensPath);
    }
  });
  lens.addEventListener("click", (event) => {
    if (event.target === lens) {
      hideLens();
    }
  });
  lens.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      const delta = event.deltaY < 0 ? LENS_STEP : -LENS_STEP;
      lensScale = Math.min(LENS_MAX, Math.max(LENS_MIN, lensScale + delta));
      placeLens();
    },
    { passive: false },
  );
  lensImage.addEventListener("pointerdown", startPan);
  lensImage.addEventListener("pointermove", movePan);
  lensImage.addEventListener("pointerup", endPan);
  lensImage.addEventListener("pointercancel", endPan);
  document.addEventListener("keydown", onKeyDown);
  document.addEventListener("copy", onCopy);
  document.addEventListener("mousedown", (event) => {
    if (!event.target.closest("#menu")) {
      hideMenu();
    }
  });
  board.addEventListener("click", (event) => {
    if (event.target.closest(".card")) {
      return;
    }
    selected.clear();
    anchor = null;
    markSelected();
  });
}

function applyShell() {
  document.querySelector("#product").textContent =
    "AgSnapTool " + shell.version;
  document.querySelector("#hint").textContent = shell.hint;
  document.querySelector("#folder").textContent = shell.folder;
  document.querySelector("#log").textContent = shell.log;
  document.querySelector("#shortcut-label").textContent = shell.shortcut;
  document.querySelector("#retention-label").textContent = shell.retention;
  document.querySelector("#language-label").textContent = shell.language_label;
  document.querySelector("#version-label").textContent = shell.version_label;
  document.querySelector("#days-label").textContent = shell.days_label;
  document.querySelector("#control").checked = shell.control;
  document.querySelector("#shift").checked = shell.shift;
  document.querySelector("#alt").checked = shell.alt;
  document.querySelector("#days").value = String(shell.retention_days);
  const language = document.querySelector("#language");
  language.options[0].textContent = shell.system;
  language.options[1].textContent = shell.korean;
  language.options[2].textContent = shell.english;
  language.value = shell.language;
  document.querySelector("#changelog").textContent = shell.changelog;
  document.querySelector("#menu-copy").textContent = shell.copy_image;
  document.querySelector("#menu-lens").textContent = shell.magnify_image;
  document.querySelector("#theme-label").textContent = shell.theme_label;
  const theme = document.querySelector("#theme");
  theme.options[0].textContent = shell.theme_system;
  theme.options[1].textContent = shell.theme_dark;
  theme.options[2].textContent = shell.theme_light;
  theme.value = shell.theme;
  const themeName = shell.theme === "" ? "system" : shell.theme;
  document.documentElement.dataset.theme = themeName;
  const panel = document.querySelector("#panel");
  panel.classList.toggle("collapsed", !shell.panel_open);
  document.querySelector("#panel-toggle").textContent = shell.panel_open
    ? shell.panel_close_label
    : shell.panel_open_label;
}

async function reload(reset) {
  const gallery = await core.invoke("gallery");
  shots = gallery.shots;
  const selected = day.value;
  const scroll = scroller.scrollTop;
  const kept = shown;
  day.replaceChildren();
  const all = document.createElement("option");
  all.value = "";
  all.textContent = shell.all;
  day.append(all);
  for (const name of gallery.days) {
    const option = document.createElement("option");
    option.value = name;
    option.textContent = name;
    day.append(option);
  }
  if ([...day.options].some((option) => option.value === selected)) {
    day.value = selected;
  }
  paint(reset);
  if (!reset) {
    while (shown < kept && shown < filtered.length) {
      showMore();
    }
    scroller.scrollTop = scroll;
  }
}

function pageSize() {
  const width = scroller.clientWidth || COLUMN_WIDTH;
  const height = scroller.clientHeight || CARD_MIN_HEIGHT * 2;
  const columns = Math.max(1, Math.floor(width / COLUMN_WIDTH));
  const rows = Math.max(2, Math.ceil(height / CARD_MIN_HEIGHT));
  return columns * rows;
}

function paint(reset) {
  filtered = shots.filter(
    (shot) => day.value === "" || shot.day === day.value,
  );
  shown = 0;
  if (reset) {
    selected.clear();
    anchor = null;
  } else {
    const live = new Set(filtered.map((shot) => shot.path));
    for (const path of [...selected]) {
      if (!live.has(path)) {
        selected.delete(path);
      }
    }
    if (anchor && !live.has(anchor)) {
      anchor = null;
    }
  }
  board.replaceChildren();
  if (reset) {
    scroller.scrollTop = 0;
  }
  if (filtered.length === 0) {
    const note = document.createElement("p");
    note.className = "empty";
    note.textContent = shell.empty;
    board.append(note);
    return;
  }
  const marker = document.createElement("div");
  marker.className = "more";
  board.append(marker);
  showMore();
  requestAnimationFrame(fillVisible);
}

function fillVisible() {
  if (shown >= filtered.length || scroller.clientHeight === 0) {
    return;
  }
  if (scroller.scrollHeight <= scroller.clientHeight + 8) {
    showMore();
    requestAnimationFrame(fillVisible);
  }
}

function loadIfNearEnd() {
  const remain =
    scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
  if (remain < CARD_MIN_HEIGHT) {
    showMore();
  }
}

function showMore() {
  const marker = board.querySelector(".more");
  if (!marker) {
    return;
  }
  const next = filtered.slice(shown, shown + pageSize());
  if (next.length === 0) {
    marker.remove();
    return;
  }
  for (const shot of next) {
    board.insertBefore(card(shot), marker);
  }
  shown += next.length;
  if (shown >= filtered.length) {
    marker.remove();
  }
}

function card(shot) {
  const button = document.createElement("button");
  button.className = "card";
  button.type = "button";
  button.dataset.path = shot.path;
  if (selected.has(shot.path)) {
    button.classList.add("selected");
  }
  button.style.setProperty("--tilt", tiltDegrees(shot.path) + "deg");
  const leftTape = document.createElement("span");
  leftTape.className = "tape left";
  const rightTape = document.createElement("span");
  rightTape.className = "tape right";
  const image = document.createElement("img");
  image.alt = shot.day;
  image.loading = "lazy";
  image.draggable = false;
  image.src = core.convertFileSrc(shot.path);
  const caption = document.createElement("span");
  caption.className = "caption";
  caption.textContent = shot.day === "unknown" ? "" : shot.day;
  button.append(leftTape, rightTape, image, caption);
  button.addEventListener("mouseenter", (event) => {
    trackTip(event);
    scheduleTip(shot);
  });
  button.addEventListener("mousemove", trackTip);
  button.addEventListener("mouseleave", cancelTip);
  button.addEventListener("click", (event) => selectShot(shot.path, event));
  button.addEventListener("dblclick", () => {
    core.invoke("open_shot", { path: shot.path });
  });
  button.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    event.stopPropagation();
    if (!selected.has(shot.path)) {
      selected.clear();
      selected.add(shot.path);
      anchor = shot.path;
      markSelected();
    }
    lensPath = shot.path;
    showMenu(event.clientX, event.clientY);
  });
  return button;
}

function selectShot(path, event) {
  if (event.shiftKey && anchor) {
    selectRange(anchor, path, event.ctrlKey);
  } else if (event.ctrlKey) {
    if (selected.has(path)) {
      selected.delete(path);
    } else {
      selected.add(path);
    }
    anchor = path;
  } else {
    selected.clear();
    selected.add(path);
    anchor = path;
  }
  markSelected();
}

function selectRange(fromPath, toPath, keep) {
  const start = filtered.findIndex((shot) => shot.path === fromPath);
  const end = filtered.findIndex((shot) => shot.path === toPath);
  if (start < 0 || end < 0) {
    return;
  }
  const from = Math.min(start, end);
  const to = Math.max(start, end);
  if (!keep) {
    selected.clear();
  }
  for (let index = from; index <= to; index += 1) {
    selected.add(filtered[index].path);
  }
}

function markSelected() {
  for (const button of board.querySelectorAll(".card")) {
    button.classList.toggle("selected", selected.has(button.dataset.path));
  }
}

function chosenPaths() {
  return filtered
    .map((shot) => shot.path)
    .filter((path) => selected.has(path));
}

function copySelection() {
  const paths = chosenPaths();
  if (paths.length === 0) {
    return;
  }
  core.invoke("copy_shots", { paths }).catch((failure) => {
    error.textContent = String(failure);
  });
}

function showMenu(x, y) {
  menu.hidden = false;
  menu.style.left = "0px";
  menu.style.top = "0px";
  const width = menu.offsetWidth;
  const height = menu.offsetHeight;
  const left = Math.max(0, Math.min(x, window.innerWidth - width));
  const top = Math.max(0, Math.min(y, window.innerHeight - height));
  menu.style.left = left + "px";
  menu.style.top = top + "px";
}

function hideMenu() {
  menu.hidden = true;
}

function tiltDegrees(path) {
  let hash = 0;
  for (let index = 0; index < path.length; index += 1) {
    hash = (hash * 33 + path.charCodeAt(index)) >>> 0;
  }
  return ((hash % 41) - 20) / 8;
}

function scheduleTip(shot) {
  clearTimeout(hoverTimer);
  hideTip();
  hoverTimer = setTimeout(() => showTip(shot), HOVER_DELAY_MS);
}

function trackTip(event) {
  tipX = event.clientX + 14;
  tipY = event.clientY + 18;
  if (!tip.hidden) {
    placeTip();
  }
}

function showTip(shot) {
  const label = shot.taken || shot.day;
  if (!label) {
    return;
  }
  tip.textContent = label;
  tip.hidden = false;
  placeTip();
}

function placeTip() {
  tip.style.left = tipX + "px";
  tip.style.top = tipY + "px";
}

function cancelTip() {
  clearTimeout(hoverTimer);
  hideTip();
}

function hideTip() {
  tip.hidden = true;
}

function placeLens() {
  lensImage.style.transform =
    "translate(" + lensX + "px, " + lensY + "px) scale(" + lensScale + ")";
  lensImage.style.cursor = lensScale > LENS_MIN ? "grab" : "default";
}

function startPan(event) {
  if (event.button !== 0 || lensScale <= LENS_MIN) {
    return;
  }
  event.preventDefault();
  lensPan = {
    x: event.clientX - lensX,
    y: event.clientY - lensY,
    pointer: event.pointerId,
  };
  lensImage.setPointerCapture(event.pointerId);
  lensImage.style.cursor = "grabbing";
}

function movePan(event) {
  if (!lensPan || lensPan.pointer !== event.pointerId) {
    return;
  }
  lensX = event.clientX - lensPan.x;
  lensY = event.clientY - lensPan.y;
  placeLens();
  lensImage.style.cursor = "grabbing";
}

function endPan(event) {
  if (!lensPan || lensPan.pointer !== event.pointerId) {
    return;
  }
  lensPan = null;
  placeLens();
}

function openLens(path) {
  lensScale = LENS_MIN;
  lensX = 0;
  lensY = 0;
  lensPan = null;
  lensImage.src = core.convertFileSrc(path);
  placeLens();
  lens.hidden = false;
}

function hideLens() {
  lensPan = null;
  lens.hidden = true;
  placeLens();
}

function isTyping(target) {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement
  );
}

function onKeyDown(event) {
  if (event.key === "Escape") {
    hideMenu();
    hideLens();
    return;
  }
  if (isTyping(event.target) || event.key.toLowerCase() !== "c" || !event.ctrlKey) {
    return;
  }
  if (selected.size === 0) {
    return;
  }
  event.preventDefault();
  copySelection();
}

function onCopy(event) {
  if (isTyping(event.target) || selected.size === 0) {
    return;
  }
  event.preventDefault();
  copySelection();
}

async function saveShortcut() {
  error.textContent = "";
  try {
    shell = await core.invoke("set_shortcut", {
      control: document.querySelector("#control").checked,
      shift: document.querySelector("#shift").checked,
      alt: document.querySelector("#alt").checked,
    });
    applyShell();
  } catch (failure) {
    error.textContent = String(failure);
    shell = await core.invoke("shell_state");
    applyShell();
  }
}

async function saveDays() {
  const days = Number(document.querySelector("#days").value);
  shell = await core.invoke("set_retention_days", { days });
  applyShell();
}

async function saveTheme() {
  shell = await core.invoke("set_theme", {
    tag: document.querySelector("#theme").value,
  });
  applyShell();
}

async function savePanel() {
  const open = document.querySelector("#panel").classList.contains("collapsed");
  shell = await core.invoke("set_panel_open", { open });
  applyShell();
  requestAnimationFrame(fillVisible);
}

async function saveLanguage() {
  shell = await core.invoke("set_language", {
    tag: document.querySelector("#language").value,
  });
  applyShell();
}

boot();
