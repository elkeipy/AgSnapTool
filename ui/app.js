const core = window.__TAURI__.core;
const events = window.__TAURI__.event;
const COLUMN_WIDTH = 232;
const CARD_MIN_HEIGHT = 260;
const TAPE_COLORS = ["yellow", "red", "blue"];

const LENS_MIN = 1;
const LENS_MAX = 6;
const LENS_STEP = 0.25;
const PANE_OPEN_WIDTH = 210;
const PANE_CLOSED_WIDTH = 44;
const PRESS_SCALE = 0.97;
const SPRING_SETTLE = { damping: 1, duration: 0.4 };
const SPRING_BOUNCE = { damping: 0.8, duration: 0.3 };
const SWITCH_IDS = ["control", "shift", "alt", "polaroid", "tape"];

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
const selected = new Set();
const springRuns = new WeakMap();
let paneOpen = null;
let paneSpringTarget = null;
let paneMotion = 0;

const day = document.querySelector("#day");
const scroller = document.querySelector("#scroller");
const board = document.querySelector("#board");
const error = document.querySelector("#shortcut-error");
const menu = document.querySelector("#menu");
const lens = document.querySelector("#lens");
const lensImage = document.querySelector("#lens-image");
const history = document.querySelector("#history");

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
  document
    .querySelector("#polaroid")
    .addEventListener("change", savePolaroid);
  document.querySelector("#tilt").addEventListener("change", saveTilt);
  document.querySelector("#tape").addEventListener("change", saveTape);
  document
    .querySelector("#tape-width")
    .addEventListener("change", saveTapeWidth);
  document
    .querySelector("#tape-height")
    .addEventListener("change", saveTapeHeight);
  document
    .querySelector("#history-open")
    .addEventListener("click", showHistory);
  document
    .querySelector("#history-close")
    .addEventListener("click", hideHistory);
  history.addEventListener("click", (event) => {
    if (event.target === history) {
      hideHistory();
    }
  });
  document.querySelector("#panel-toggle").addEventListener("click", savePanel);
  document.querySelector("#panel").addEventListener(
    "change",
    (event) => {
      const target = event.target;
      if (target instanceof HTMLInputElement && target.type === "checkbox") {
        placeKnob(target, true);
      }
    },
    true,
  );
  bindPanePress();
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
  document.querySelector("#menu-open").addEventListener("click", () => {
    hideMenu();
    if (lensPath) {
      core.invoke("open_shot", { path: lensPath });
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
  document.querySelector("#history-title").textContent = shell.history_label;
  document.querySelector("#history-close").textContent = shell.close_label;
  document.querySelector("#history-open").textContent = shell.history_label;
  document.querySelector("#polaroid-label").textContent = shell.polaroid_label;
  document.querySelector("#polaroid").checked = shell.polaroid;
  document.querySelector("#tilt-label").textContent = shell.tilt_label;
  document.querySelector("#tilt").disabled = !shell.polaroid;
  document.querySelector("#tilt").value = shell.polaroid
    ? String(shell.tilt)
    : "0";
  document.querySelector("#tape-label").textContent = shell.tape_label;
  document.querySelector("#tape").disabled = !shell.polaroid;
  document.querySelector("#tape").checked =
    shell.polaroid && shell.tape_visible;
  const tapeLive = shell.polaroid && shell.tape_visible;
  document.querySelector("#tape-width-label").textContent =
    shell.tape_width_label;
  document.querySelector("#tape-width").disabled = !tapeLive;
  document.querySelector("#tape-width").value = String(shell.tape_width);
  document.querySelector("#tape-height-label").textContent =
    shell.tape_height_label;
  document.querySelector("#tape-height").disabled = !tapeLive;
  document.querySelector("#tape-height").value = String(shell.tape_height);
  board.style.setProperty("--tape-width", shell.tape_width + "px");
  board.style.setProperty("--tape-height", shell.tape_height + "px");
  board.classList.toggle("bare", !tapeLive);
  document.querySelector("#menu-copy").textContent = shell.copy_image;
  document.querySelector("#menu-lens").textContent = shell.magnify_image;
  document.querySelector("#menu-open").textContent = shell.open_with;
  document.querySelector("#theme-label").textContent = shell.theme_label;
  const theme = document.querySelector("#theme");
  theme.options[0].textContent = shell.theme_system;
  theme.options[1].textContent = shell.theme_dark;
  theme.options[2].textContent = shell.theme_light;
  theme.options[3].textContent = shell.theme_flat;
  theme.options[4].textContent = shell.theme_material;
  theme.options[5].textContent = shell.theme_fluent;
  theme.options[6].textContent = shell.theme_glass;
  theme.value = shell.theme;
  const themeName = shell.theme === "" ? "system" : shell.theme;
  document.documentElement.dataset.theme = themeName;
  for (const id of SWITCH_IDS) {
    placeKnob(document.querySelector("#" + id), false);
  }
  syncPane();
}

function prefersReducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function springBucket(target) {
  let runs = springRuns.get(target);
  if (!runs) {
    runs = new Map();
    springRuns.set(target, runs);
  }
  return runs;
}

function cancelSpring(target, key) {
  const runs = springRuns.get(target);
  if (!runs) {
    return;
  }
  const run = runs.get(key);
  if (!run) {
    return;
  }
  cancelAnimationFrame(run.frame);
  runs.delete(key);
}

function springGoal(target, key) {
  const run = springRuns.get(target)?.get(key);
  return run ? run.to : null;
}

function springTo(target, key, from, to, spec, onFrame, onDone) {
  if (prefersReducedMotion()) {
    cancelSpring(target, key);
    onFrame(to);
    if (onDone) {
      onDone();
    }
    return;
  }
  const runs = springBucket(target);
  const previous = runs.get(key);
  if (previous) {
    cancelAnimationFrame(previous.frame);
  }
  const state = {
    value: previous ? previous.value : from,
    velocity: previous ? previous.velocity : 0,
    to,
    frame: 0,
  };
  const omega = (2 * Math.PI) / spec.duration;
  const zeta = spec.damping;
  const started = performance.now();
  let last = started;
  const epsilon = Math.max(0.0008, Math.abs(to - state.value) * 0.004);
  const step = (now) => {
    const dt = Math.min(0.032, (now - last) / 1000);
    last = now;
    const accel =
      omega * omega * (to - state.value) -
      2 * zeta * omega * state.velocity;
    state.velocity += accel * dt;
    state.value += state.velocity * dt;
    onFrame(state.value);
    const timedOut = now - started > (spec.duration + 0.45) * 1000;
    const settled =
      Math.abs(to - state.value) <= epsilon &&
      Math.abs(state.velocity) <= epsilon * 12;
    if (!settled && !timedOut) {
      state.frame = requestAnimationFrame(step);
      return;
    }
    state.value = to;
    state.velocity = 0;
    onFrame(to);
    runs.delete(key);
    if (onDone) {
      onDone();
    }
  };
  runs.set(key, state);
  state.frame = requestAnimationFrame(step);
}

function switchFace(input) {
  return input.closest(".switch") || input;
}

function placeKnob(input, animate) {
  const face = switchFace(input);
  const to = input.checked ? 1 : 0;
  const goal = springGoal(face, "knob");
  if (goal !== null && Math.abs(goal - to) < 0.001) {
    return;
  }
  const raw = face.style.getPropertyValue("--knob");
  const from = raw === "" ? to : Number(raw);
  if (!animate || Math.abs(from - to) < 0.001) {
    cancelSpring(face, "knob");
    face.style.setProperty("--knob", String(to));
    return;
  }
  springTo(face, "knob", from, to, SPRING_SETTLE, (value) => {
    face.style.setProperty("--knob", value.toFixed(4));
  });
}

function paneWidth(panel) {
  const width = panel.getBoundingClientRect().width;
  if (width >= PANE_CLOSED_WIDTH - 1) {
    return width;
  }
  return panel.classList.contains("collapsed")
    ? PANE_CLOSED_WIDTH
    : PANE_OPEN_WIDTH;
}

function paneReveal(panel) {
  const raw = getComputedStyle(panel).getPropertyValue("--pane-reveal").trim();
  const value = Number(raw);
  return Number.isFinite(value) ? value : 1;
}

function writePane(panel, width, reveal) {
  panel.style.setProperty("--pane-width", width.toFixed(2) + "px");
  panel.style.setProperty("--pane-reveal", String(reveal));
}

function restPane(open) {
  const panel = document.querySelector("#panel");
  const body = document.querySelector("#panel-body");
  cancelSpring(panel, "width");
  cancelSpring(panel, "reveal");
  paneSpringTarget = null;
  writePane(
    panel,
    open ? PANE_OPEN_WIDTH : PANE_CLOSED_WIDTH,
    open ? 1 : 0,
  );
  panel.classList.toggle("collapsed", !open);
  body.inert = !open;
}

function movePane(open) {
  const panel = document.querySelector("#panel");
  const body = document.querySelector("#panel-body");
  const motion = ++paneMotion;
  paneSpringTarget = open;
  body.inert = !open;
  const widthTo = open ? PANE_OPEN_WIDTH : PANE_CLOSED_WIDTH;
  const revealTo = open ? 1 : 0;
  if (open) {
    writePane(panel, paneWidth(panel), paneReveal(panel));
    panel.classList.remove("collapsed");
  }
  let pending = 2;
  const finish = () => {
    pending -= 1;
    if (pending > 0 || motion !== paneMotion) {
      return;
    }
    if (paneSpringTarget === open) {
      paneSpringTarget = null;
    }
    writePane(panel, widthTo, revealTo);
    panel.classList.toggle("collapsed", !open);
  };
  springTo(panel, "width", paneWidth(panel), widthTo, SPRING_SETTLE, (value) => {
    panel.style.setProperty("--pane-width", value.toFixed(2) + "px");
  }, finish);
  springTo(
    panel,
    "reveal",
    paneReveal(panel),
    revealTo,
    SPRING_SETTLE,
    (value) => {
      panel.style.setProperty("--pane-reveal", value.toFixed(4));
    },
    finish,
  );
}

function syncPane() {
  const open = shell.panel_open;
  document.querySelector("#panel-toggle").textContent = open
    ? shell.panel_close_label
    : shell.panel_open_label;
  const changed = paneOpen !== null && paneOpen !== open;
  paneOpen = open;
  if (!changed) {
    if (paneSpringTarget === open) {
      return;
    }
    restPane(open);
    return;
  }
  movePane(open);
}

function pressControl(node) {
  if (!(node instanceof Element)) {
    return null;
  }
  const direct = node.closest("#panel-toggle, #history-open, .switch");
  if (direct) {
    return direct;
  }
  const row = node.closest(".pane-row");
  return row ? row.querySelector(".switch") : null;
}

function controlDisabled(control) {
  if (control instanceof HTMLInputElement) {
    return control.disabled;
  }
  const input = control.querySelector("input, button");
  return input ? input.disabled : false;
}

function releasePress(control) {
  const raw = control.style.getPropertyValue("--press");
  const from = raw === "" ? PRESS_SCALE : Number(raw);
  springTo(control, "press", from, 1, SPRING_BOUNCE, (value) => {
    control.style.setProperty("--press", value.toFixed(4));
  });
}

function bindPanePress() {
  const panel = document.querySelector("#panel");
  let held = null;
  const release = () => {
    if (!held) {
      return;
    }
    const control = held;
    held = null;
    releasePress(control);
  };
  panel.addEventListener("pointerdown", (event) => {
    const control = pressControl(event.target);
    if (!control || event.button !== 0 || controlDisabled(control)) {
      return;
    }
    held = control;
    cancelSpring(control, "press");
    control.style.setProperty("--press", String(PRESS_SCALE));
  });
  panel.addEventListener("pointerup", release);
  panel.addEventListener("pointercancel", release);
  panel.addEventListener("pointerout", (event) => {
    if (!held || held.contains(event.relatedTarget)) {
      return;
    }
    release();
  });
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
  const tapes = tapePieces(shot.path);
  const image = document.createElement("img");
  image.alt = shot.day;
  image.loading = "lazy";
  image.draggable = false;
  image.src = core.convertFileSrc(shot.path);
  const caption = document.createElement("span");
  caption.className = "caption";
  caption.textContent =
    shot.taken || (shot.day === "unknown" ? "" : shot.day);
  button.append(...tapes, image, caption);
  button.addEventListener("click", (event) => selectShot(shot.path, event));
  button.addEventListener("dblclick", () => {
    openLens(shot.path);
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

function hashOf(text) {
  let hash = 0;
  for (let index = 0; index < text.length; index += 1) {
    hash = (hash * 33 + text.charCodeAt(index)) >>> 0;
  }
  return hash;
}

function tiltDegrees(path) {
  if (!shell || !shell.polaroid) {
    return 0;
  }
  const max = Number(shell.tilt);
  if (!max) {
    return 0;
  }
  const hash = hashOf(path);
  const unit = ((hash % 1000) / 999) * 2 - 1;
  return Math.round(unit * max * 10) / 10;
}

function mixedHash(text) {
  const hash = hashOf(text);
  return (hash ^ (hash >>> 16)) >>> 0;
}

function tapePieces(path) {
  const color = TAPE_COLORS[mixedHash(path + ":color") % TAPE_COLORS.length];
  const places =
    mixedHash(path + ":place") % 3 === 0
      ? ["center"]
      : ["left", "right"];
  return places.map((place) => tapeSpan(path, place, color));
}

function tapeSpan(path, place, color) {
  const span = document.createElement("span");
  const angle = (mixedHash(path + ":" + place) % 37) - 18;
  span.className = "tape " + place + " " + color;
  span.style.transform = "rotate(" + angle + "deg)";
  return span;
}

function showHistory() {
  history.hidden = false;
}

function hideHistory() {
  history.hidden = true;
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
    hideHistory();
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

async function savePolaroid() {
  const enabled = document.querySelector("#polaroid").checked;
  shell = await core.invoke("set_polaroid", { enabled });
  applyShell();
  paint(false);
}

async function saveTilt() {
  const degrees = Number(document.querySelector("#tilt").value);
  shell = await core.invoke("set_tilt", { degrees });
  applyShell();
  paint(false);
}

async function saveTape() {
  const visible = document.querySelector("#tape").checked;
  shell = await core.invoke("set_tape_visible", { visible });
  applyShell();
}

async function saveTapeWidth() {
  const pixels = Number(document.querySelector("#tape-width").value);
  shell = await core.invoke("set_tape_width", { pixels });
  applyShell();
}

async function saveTapeHeight() {
  const pixels = Number(document.querySelector("#tape-height").value);
  shell = await core.invoke("set_tape_height", { pixels });
  applyShell();
}

async function saveTheme() {
  shell = await core.invoke("set_theme", {
    tag: document.querySelector("#theme").value,
  });
  applyShell();
}

async function savePanel() {
  const panel = document.querySelector("#panel");
  const shown =
    paneSpringTarget === null
      ? !panel.classList.contains("collapsed")
      : paneSpringTarget;
  shell = await core.invoke("set_panel_open", { open: !shown });
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
