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
const KIND_RECT = "rect";
const KIND_ELLIPSE = "ellipse";
const KIND_POLYGON = "polygon";
const KIND_ARROW = "arrow";
const KIND_TEXT = "text";
const TOOL_SELECT = "select";
const MIN_SHAPE = 4;
const HANDLE_SIZE = 8;
const TEXT_BOX_WIDTH = 240;

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
let editorPath = null;
let editorUrl = null;
let editorPicture = null;
let editorShapes = [];
let editorSelected = -1;
let editorTool = TOOL_SELECT;
let editorDraft = null;
let editorDrag = null;

const day = document.querySelector("#day");
const scroller = document.querySelector("#scroller");
const board = document.querySelector("#board");
const error = document.querySelector("#shortcut-error");
const menu = document.querySelector("#menu");
const lens = document.querySelector("#lens");
const lensImage = document.querySelector("#lens-image");
const history = document.querySelector("#history");
const editor = document.querySelector("#editor");
const editorCanvas = document.querySelector("#edit-canvas");
const editorText = document.querySelector("#edit-text-input");
const editorError = document.querySelector("#edit-error");

async function boot() {
  shell = await core.invoke("shell_state");
  applyShell();
  await reload(true);
  await events.listen("gallery-refresh", () => reload(false));
  day.addEventListener("change", () => paint(true));
  scroller.addEventListener("scroll", () => {
    hideMenu();
    loadIfNearEnd();
  });
  scroller.addEventListener("wheel", hideMenu, { passive: true });
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
  document.querySelector("#menu-edit").addEventListener("click", () => {
    hideMenu();
    if (lensPath) {
      openEditor(lensPath);
    }
  });
  bindEditor();
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
  document.querySelector("#menu-edit").textContent = shell.edit_image;
  applyEditorShell();
  document.querySelector("#theme-label").textContent = shell.theme_label;
  const theme = document.querySelector("#theme");
  theme.options[0].textContent = shell.theme_system;
  theme.options[1].textContent = shell.theme_dark;
  theme.options[2].textContent = shell.theme_light;
  theme.options[3].textContent = shell.theme_flat;
  theme.options[4].textContent = shell.theme_material;
  theme.options[5].textContent = shell.theme_glass;
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
  if (isEditorOpen() && handleEditorKey(event)) {
    return;
  }
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
  if (isEditorOpen() || isTyping(event.target) || selected.size === 0) {
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

function applyEditorShell() {
  if (!shell || !editor) {
    return;
  }
  document.querySelector("#edit-select").textContent = shell.editor_select;
  document.querySelector("#edit-rect").textContent = shell.editor_rect;
  document.querySelector("#edit-ellipse").textContent = shell.editor_ellipse;
  document.querySelector("#edit-polygon").textContent = shell.editor_polygon;
  document.querySelector("#edit-arrow").textContent = shell.editor_arrow;
  document.querySelector("#edit-text").textContent = shell.editor_text;
  document.querySelector("#edit-stroke-label").textContent =
    shell.editor_stroke;
  document.querySelector("#edit-fill-label").textContent = shell.editor_fill;
  document.querySelector("#edit-width-label").textContent = shell.editor_width;
  document.querySelector("#edit-size-label").textContent = shell.editor_size;
  document.querySelector("#edit-head-label").textContent = shell.editor_head;
  document.querySelector("#edit-head-size-label").textContent =
    shell.editor_head_size;
  document.querySelector("#edit-font-label").textContent = shell.editor_font;
  document.querySelector("#edit-font-size-label").textContent =
    shell.editor_font_size;
  document.querySelector("#edit-color-label").textContent = shell.editor_color;
  document.querySelector("#edit-hint").textContent = shell.editor_hint;
  document.querySelector("#edit-save").textContent = shell.editor_save;
  document.querySelector("#edit-cancel").textContent = shell.editor_cancel;
  const head = document.querySelector("#edit-head");
  head.options[0].textContent = shell.editor_head_triangle;
  head.options[1].textContent = shell.editor_head_chevron;
  head.options[2].textContent = shell.editor_head_barbed;
}

function bindEditor() {
  for (const button of editor.querySelectorAll("[data-tool]")) {
    button.addEventListener("click", () => {
      setEditorTool(button.dataset.tool);
    });
  }
  document.querySelector("#edit-save").addEventListener("click", saveEditor);
  document.querySelector("#edit-cancel").addEventListener("click", closeEditor);
  editorCanvas.addEventListener("pointerdown", onEditorDown);
  editorCanvas.addEventListener("pointermove", onEditorMove);
  editorCanvas.addEventListener("pointerup", onEditorUp);
  editorCanvas.addEventListener("dblclick", onEditorDouble);
  editorText.addEventListener("blur", commitEditorText);
  editorText.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      commitEditorText();
    }
  });
  for (const id of [
    "edit-stroke",
    "edit-fill",
    "edit-fill-on",
    "edit-width",
    "edit-box-w",
    "edit-box-h",
    "edit-head",
    "edit-head-size",
    "edit-font",
    "edit-font-size",
    "edit-color",
  ]) {
    document.querySelector("#" + id).addEventListener("input", applyEditorStyle);
    document
      .querySelector("#" + id)
      .addEventListener("change", applyEditorStyle);
  }
}

function isEditorOpen() {
  return Boolean(editor && !editor.hidden);
}

function handleEditorKey(event) {
  if (event.key === "Escape") {
    if (!editorText.hidden) {
      commitEditorText();
      return true;
    }
    if (editorDraft) {
      editorDraft = null;
      paintEditor();
      return true;
    }
    closeEditor();
    return true;
  }
  if (event.key === "Enter" && editorDraft && editorDraft.kind === KIND_POLYGON) {
    event.preventDefault();
    closePolygonDraft();
    return true;
  }
  if (isTyping(event.target)) {
    return true;
  }
  if (event.key === "Delete" || event.key === "Backspace") {
    event.preventDefault();
    removeSelectedShape();
    return true;
  }
  return false;
}

async function openEditor(path) {
  hideLens();
  hideHistory();
  editorError.textContent = "";
  try {
    const raw = await core.invoke("read_shot", { path });
    const bytes = raw instanceof Uint8Array ? raw : new Uint8Array(raw);
    forgetEditorUrl();
    editorUrl = URL.createObjectURL(
      new Blob([bytes], { type: "image/png" }),
    );
    const picture = new Image();
    picture.onload = () => {
      editorPath = path;
      editorPicture = picture;
      editorShapes = [];
      editorSelected = -1;
      editorDraft = null;
      editorDrag = null;
      editorCanvas.width = picture.naturalWidth;
      editorCanvas.height = picture.naturalHeight;
      setEditorTool(TOOL_SELECT);
      editor.hidden = false;
      paintEditor();
    };
    picture.onerror = () => {
      editorError.textContent = shell ? shell.edit_image : "";
    };
    picture.src = editorUrl;
  } catch (failure) {
    editorError.textContent = String(failure);
    editor.hidden = false;
  }
}

function closeEditor() {
  editor.hidden = true;
  editorPath = null;
  editorPicture = null;
  editorShapes = [];
  editorSelected = -1;
  editorDraft = null;
  editorDrag = null;
  editorText.hidden = true;
  editorError.textContent = "";
  forgetEditorUrl();
}

function forgetEditorUrl() {
  if (editorUrl) {
    URL.revokeObjectURL(editorUrl);
    editorUrl = null;
  }
}

function setEditorTool(tool) {
  if (editorDraft && editorDraft.kind === KIND_POLYGON) {
    closePolygonDraft();
  }
  editorTool = tool;
  editor.dataset.tool = tool;
  editor.classList.toggle("tool-select", tool === TOOL_SELECT);
  for (const button of editor.querySelectorAll("[data-tool]")) {
    button.classList.toggle("active", button.dataset.tool === tool);
  }
  syncEditorProps();
}

function editorStyle() {
  return {
    stroke: document.querySelector("#edit-stroke").value,
    fill: document.querySelector("#edit-fill").value,
    fillOn: document.querySelector("#edit-fill-on").checked,
    width: Number(document.querySelector("#edit-width").value) || 3,
    head: document.querySelector("#edit-head").value,
    headSize: Number(document.querySelector("#edit-head-size").value) || 18,
    font: document.querySelector("#edit-font").value,
    size: Number(document.querySelector("#edit-font-size").value) || 22,
    color: document.querySelector("#edit-color").value,
  };
}

function selectedShape() {
  return editorShapes[editorSelected] || null;
}

function syncEditorProps() {
  const shape = selectedShape();
  const kind = shape ? shape.kind : editorTool;
  const isBox = kind === KIND_RECT || kind === KIND_ELLIPSE;
  const isShape =
    isBox || kind === KIND_POLYGON || kind === KIND_ARROW;
  const isArrow = kind === KIND_ARROW;
  const isText = kind === KIND_TEXT;
  document.querySelector("#edit-stroke").closest("label").hidden = !isShape;
  document.querySelector("#edit-fill").closest("label").hidden =
    !isBox && kind !== KIND_POLYGON;
  document.querySelector("#edit-width").closest("label").hidden = !isShape;
  document.querySelector("#edit-box-w").closest("label").hidden = !isBox;
  document.querySelector("#edit-head").closest("label").hidden = !isArrow;
  document.querySelector("#edit-head-size").closest("label").hidden =
    !isArrow;
  document.querySelector("#edit-font").closest("label").hidden = !isText;
  document.querySelector("#edit-font-size").closest("label").hidden = !isText;
  document.querySelector("#edit-color").closest("label").hidden = !isText;
  if (!shape) {
    return;
  }
  if (shape.stroke) {
    document.querySelector("#edit-stroke").value = shape.stroke;
  }
  if (shape.fill) {
    document.querySelector("#edit-fill").value = shape.fill;
  }
  document.querySelector("#edit-fill-on").checked = Boolean(shape.fillOn);
  if (shape.width) {
    document.querySelector("#edit-width").value = String(shape.width);
  }
  if (isBox) {
    document.querySelector("#edit-box-w").value = String(
      Math.round(Math.abs(shape.w)),
    );
    document.querySelector("#edit-box-h").value = String(
      Math.round(Math.abs(shape.h)),
    );
  }
  if (isArrow) {
    document.querySelector("#edit-head").value = shape.head || "triangle";
    document.querySelector("#edit-head-size").value = String(
      shape.headSize || 18,
    );
  }
  if (isText) {
    document.querySelector("#edit-font").value = shape.font;
    document.querySelector("#edit-font-size").value = String(shape.size);
    document.querySelector("#edit-color").value = shape.color;
  }
}

function applyEditorStyle() {
  const shape = selectedShape();
  if (!shape) {
    paintEditor();
    return;
  }
  const style = editorStyle();
  if (shape.kind === KIND_TEXT) {
    shape.font = style.font;
    shape.size = style.size;
    shape.color = style.color;
  } else {
    shape.stroke = style.stroke;
    shape.fill = style.fill;
    shape.fillOn = style.fillOn;
    shape.width = style.width;
  }
  if (shape.kind === KIND_ARROW) {
    shape.head = style.head;
    shape.headSize = style.headSize;
  }
  if (shape.kind === KIND_RECT || shape.kind === KIND_ELLIPSE) {
    const width = Number(document.querySelector("#edit-box-w").value);
    const height = Number(document.querySelector("#edit-box-h").value);
    if (width >= MIN_SHAPE) {
      shape.w = width;
    }
    if (height >= MIN_SHAPE) {
      shape.h = height;
    }
  }
  paintEditor();
}

function editorPoint(event) {
  const box = editorCanvas.getBoundingClientRect();
  if (box.width === 0 || box.height === 0) {
    return { x: 0, y: 0 };
  }
  return {
    x: ((event.clientX - box.left) * editorCanvas.width) / box.width,
    y: ((event.clientY - box.top) * editorCanvas.height) / box.height,
  };
}

function onEditorDown(event) {
  if (event.button !== 0) {
    return;
  }
  const point = editorPoint(event);
  if (editorTool === TOOL_SELECT) {
    beginSelectDrag(point);
    editorCanvas.setPointerCapture(event.pointerId);
    return;
  }
  if (editorTool === KIND_TEXT) {
    beginText(point);
    return;
  }
  if (editorTool === KIND_POLYGON) {
    addPolygonPoint(point);
    return;
  }
  const style = editorStyle();
  if (editorTool === KIND_ARROW) {
    editorDraft = {
      kind: KIND_ARROW,
      x1: point.x,
      y1: point.y,
      x2: point.x,
      y2: point.y,
      ...style,
    };
  } else {
    editorDraft = {
      kind: editorTool,
      x: point.x,
      y: point.y,
      w: 0,
      h: 0,
      ...style,
    };
  }
  editorCanvas.setPointerCapture(event.pointerId);
  paintEditor();
}

function onEditorMove(event) {
  const point = editorPoint(event);
  if (editorDrag) {
    moveEditorDrag(point);
    paintEditor();
    return;
  }
  if (!editorDraft) {
    return;
  }
  if (editorDraft.kind === KIND_ARROW) {
    editorDraft.x2 = point.x;
    editorDraft.y2 = point.y;
  } else if (editorDraft.kind !== KIND_POLYGON) {
    editorDraft.w = point.x - editorDraft.x;
    editorDraft.h = point.y - editorDraft.y;
  }
  paintEditor();
}

function onEditorUp() {
  if (editorDrag) {
    editorDrag = null;
    syncEditorProps();
    paintEditor();
    return;
  }
  if (!editorDraft || editorDraft.kind === KIND_POLYGON) {
    return;
  }
  const draft = normalizeDraft(editorDraft);
  editorDraft = null;
  if (draft) {
    editorShapes.push(draft);
    editorSelected = editorShapes.length - 1;
    syncEditorProps();
  }
  paintEditor();
}

function onEditorDouble(event) {
  if (editorTool === KIND_POLYGON || (editorDraft && editorDraft.kind === KIND_POLYGON)) {
    event.preventDefault();
    closePolygonDraft();
  }
}

function beginSelectDrag(point) {
  const picked = hitEditor(point);
  editorSelected = picked.index;
  editorDrag = picked.handle
    ? { kind: "handle", handle: picked.handle, last: point }
    : picked.index >= 0
      ? { kind: "move", last: point }
      : null;
  syncEditorProps();
  paintEditor();
}

function moveEditorDrag(point) {
  const shape = selectedShape();
  if (!shape || !editorDrag) {
    return;
  }
  const dx = point.x - editorDrag.last.x;
  const dy = point.y - editorDrag.last.y;
  editorDrag.last = point;
  if (editorDrag.kind === "move") {
    shiftShape(shape, dx, dy);
    return;
  }
  resizeShape(shape, editorDrag.handle, point);
}

function addPolygonPoint(point) {
  if (!editorDraft || editorDraft.kind !== KIND_POLYGON) {
    editorDraft = {
      kind: KIND_POLYGON,
      points: [point],
      closed: false,
      ...editorStyle(),
    };
  } else {
    editorDraft.points.push(point);
  }
  paintEditor();
}

function closePolygonDraft() {
  if (!editorDraft || editorDraft.kind !== KIND_POLYGON) {
    return;
  }
  if (editorDraft.points.length >= 3) {
    editorDraft.closed = true;
    editorShapes.push(editorDraft);
    editorSelected = editorShapes.length - 1;
  }
  editorDraft = null;
  syncEditorProps();
  paintEditor();
}

function beginText(point) {
  commitEditorText();
  editorDraft = {
    kind: KIND_TEXT,
    x: point.x,
    y: point.y,
    w: TEXT_BOX_WIDTH,
    h: 32,
    text: "",
    ...editorStyle(),
  };
  placeTextInput(editorDraft);
}

function placeTextInput(shape) {
  const box = editorCanvas.getBoundingClientRect();
  const stage = editorCanvas.parentElement.getBoundingClientRect();
  const scaleX = box.width / editorCanvas.width;
  const scaleY = box.height / editorCanvas.height;
  editorText.hidden = false;
  editorText.value = shape.text || "";
  editorText.style.left =
    box.left - stage.left + shape.x * scaleX + "px";
  editorText.style.top =
    box.top - stage.top + shape.y * scaleY + "px";
  editorText.style.width = shape.w * scaleX + "px";
  editorText.style.font =
    (shape.size || 22) + "px \"" + (shape.font || "Malgun Gothic") + "\"";
  editorText.style.color = shape.color || "#1a1c1f";
  editorText.focus();
}

function commitEditorText() {
  if (editorText.hidden || !editorDraft || editorDraft.kind !== KIND_TEXT) {
    editorText.hidden = true;
    return;
  }
  const text = editorText.value.trim();
  editorText.hidden = true;
  if (text) {
    editorDraft.text = text;
    editorShapes.push(editorDraft);
    editorSelected = editorShapes.length - 1;
  }
  editorDraft = null;
  syncEditorProps();
  paintEditor();
}

function normalizeDraft(shape) {
  if (shape.kind === KIND_ARROW) {
    const length = Math.hypot(shape.x2 - shape.x1, shape.y2 - shape.y1);
    return length >= MIN_SHAPE ? shape : null;
  }
  const box = normalizeBox(shape.x, shape.y, shape.w, shape.h);
  if (box.w < MIN_SHAPE || box.h < MIN_SHAPE) {
    return null;
  }
  return { ...shape, ...box };
}

function normalizeBox(x, y, width, height) {
  const left = width < 0 ? x + width : x;
  const top = height < 0 ? y + height : y;
  return { x: left, y: top, w: Math.abs(width), h: Math.abs(height) };
}

function removeSelectedShape() {
  if (editorSelected < 0) {
    return;
  }
  editorShapes.splice(editorSelected, 1);
  editorSelected = -1;
  syncEditorProps();
  paintEditor();
}

function shiftShape(shape, dx, dy) {
  if (shape.kind === KIND_ARROW) {
    shape.x1 += dx;
    shape.y1 += dy;
    shape.x2 += dx;
    shape.y2 += dy;
    return;
  }
  if (shape.kind === KIND_POLYGON) {
    for (const point of shape.points) {
      point.x += dx;
      point.y += dy;
    }
    return;
  }
  shape.x += dx;
  shape.y += dy;
}

function resizeShape(shape, handle, point) {
  if (shape.kind === KIND_ARROW) {
    if (handle === "a") {
      shape.x1 = point.x;
      shape.y1 = point.y;
    } else {
      shape.x2 = point.x;
      shape.y2 = point.y;
    }
    return;
  }
  if (shape.kind === KIND_POLYGON) {
    const index = Number(handle);
    const vertex = shape.points[index];
    if (vertex) {
      vertex.x = point.x;
      vertex.y = point.y;
    }
    return;
  }
  const box = normalizeBox(shape.x, shape.y, shape.w, shape.h);
  let left = box.x;
  let top = box.y;
  let right = box.x + box.w;
  let bottom = box.y + box.h;
  if (handle.includes("w")) {
    left = point.x;
  }
  if (handle.includes("e")) {
    right = point.x;
  }
  if (handle.includes("n")) {
    top = point.y;
  }
  if (handle.includes("s")) {
    bottom = point.y;
  }
  const next = normalizeBox(left, top, right - left, bottom - top);
  if (next.w >= MIN_SHAPE && next.h >= MIN_SHAPE) {
    shape.x = next.x;
    shape.y = next.y;
    shape.w = next.w;
    shape.h = next.h;
  }
}

function hitEditor(point) {
  if (editorSelected >= 0) {
    const handles = shapeHandles(editorShapes[editorSelected]);
    for (const handle of handles) {
      if (Math.hypot(point.x - handle.x, point.y - handle.y) <= HANDLE_SIZE) {
        return { index: editorSelected, handle: handle.id };
      }
    }
  }
  for (let index = editorShapes.length - 1; index >= 0; index -= 1) {
    if (shapeHits(editorShapes[index], point)) {
      return { index, handle: "" };
    }
  }
  return { index: -1, handle: "" };
}

function shapeHandles(shape) {
  if (shape.kind === KIND_ARROW) {
    return [
      { id: "a", x: shape.x1, y: shape.y1 },
      { id: "b", x: shape.x2, y: shape.y2 },
    ];
  }
  if (shape.kind === KIND_POLYGON) {
    return shape.points.map((point, index) => ({
      id: String(index),
      x: point.x,
      y: point.y,
    }));
  }
  const box = shapeBox(shape);
  return [
    { id: "nw", x: box.x, y: box.y },
    { id: "n", x: box.x + box.w / 2, y: box.y },
    { id: "ne", x: box.x + box.w, y: box.y },
    { id: "e", x: box.x + box.w, y: box.y + box.h / 2 },
    { id: "se", x: box.x + box.w, y: box.y + box.h },
    { id: "s", x: box.x + box.w / 2, y: box.y + box.h },
    { id: "sw", x: box.x, y: box.y + box.h },
    { id: "w", x: box.x, y: box.y + box.h / 2 },
  ];
}

function shapeBox(shape) {
  if (shape.kind === KIND_ARROW) {
    const left = Math.min(shape.x1, shape.x2);
    const top = Math.min(shape.y1, shape.y2);
    return {
      x: left,
      y: top,
      w: Math.abs(shape.x2 - shape.x1),
      h: Math.abs(shape.y2 - shape.y1),
    };
  }
  if (shape.kind === KIND_POLYGON) {
    const xs = shape.points.map((point) => point.x);
    const ys = shape.points.map((point) => point.y);
    const left = Math.min(...xs);
    const top = Math.min(...ys);
    return {
      x: left,
      y: top,
      w: Math.max(...xs) - left,
      h: Math.max(...ys) - top,
    };
  }
  return normalizeBox(shape.x, shape.y, shape.w, shape.h);
}

function shapeHits(shape, point) {
  if (shape.kind === KIND_ARROW) {
    return (
      distanceToSegment(point, shape.x1, shape.y1, shape.x2, shape.y2) <= 8
    );
  }
  if (shape.kind === KIND_POLYGON) {
    const points = shape.points;
    for (let index = 1; index < points.length; index += 1) {
      const previous = points[index - 1];
      const current = points[index];
      if (
        distanceToSegment(
          point,
          previous.x,
          previous.y,
          current.x,
          current.y,
        ) <= 8
      ) {
        return true;
      }
    }
    return shape.closed && pointInPolygon(point, points);
  }
  const box = shapeBox(shape);
  return (
    point.x >= box.x &&
    point.x <= box.x + box.w &&
    point.y >= box.y &&
    point.y <= box.y + box.h
  );
}

function distanceToSegment(point, x1, y1, x2, y2) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const span = dx * dx + dy * dy;
  if (span === 0) {
    return Math.hypot(point.x - x1, point.y - y1);
  }
  let t = ((point.x - x1) * dx + (point.y - y1) * dy) / span;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(point.x - (x1 + t * dx), point.y - (y1 + t * dy));
}

function pointInPolygon(point, points) {
  let inside = false;
  for (let i = 0, j = points.length - 1; i < points.length; j = i, i += 1) {
    const a = points[i];
    const b = points[j];
    const crosses =
      a.y > point.y !== b.y > point.y &&
      point.x < ((b.x - a.x) * (point.y - a.y)) / (b.y - a.y) + a.x;
    if (crosses) {
      inside = !inside;
    }
  }
  return inside;
}

function paintEditor() {
  const ctx = editorCanvas.getContext("2d");
  ctx.clearRect(0, 0, editorCanvas.width, editorCanvas.height);
  if (editorPicture) {
    ctx.drawImage(editorPicture, 0, 0);
  }
  for (const shape of editorShapes) {
    drawShape(ctx, shape);
  }
  if (editorDraft) {
    drawShape(ctx, editorDraft);
  }
  const selected = selectedShape();
  if (selected) {
    drawHandles(ctx, selected);
  }
}

function drawShape(ctx, shape) {
  ctx.save();
  ctx.lineJoin = "round";
  ctx.lineCap = "round";
  ctx.lineWidth = shape.width || 3;
  ctx.strokeStyle = shape.stroke || "#ff3b30";
  ctx.fillStyle = shape.fill || "#ffd60a";
  if (shape.kind === KIND_RECT) {
    const box = normalizeBox(shape.x, shape.y, shape.w, shape.h);
    if (shape.fillOn) {
      ctx.fillRect(box.x, box.y, box.w, box.h);
    }
    ctx.strokeRect(box.x, box.y, box.w, box.h);
  } else if (shape.kind === KIND_ELLIPSE) {
    const box = normalizeBox(shape.x, shape.y, shape.w, shape.h);
    ctx.beginPath();
    ctx.ellipse(
      box.x + box.w / 2,
      box.y + box.h / 2,
      box.w / 2,
      box.h / 2,
      0,
      0,
      Math.PI * 2,
    );
    if (shape.fillOn) {
      ctx.fill();
    }
    ctx.stroke();
  } else if (shape.kind === KIND_POLYGON) {
    drawPolygon(ctx, shape);
  } else if (shape.kind === KIND_ARROW) {
    drawArrow(ctx, shape);
  } else if (shape.kind === KIND_TEXT) {
    drawTextBox(ctx, shape);
  }
  ctx.restore();
}

function drawPolygon(ctx, shape) {
  if (shape.points.length === 0) {
    return;
  }
  ctx.beginPath();
  ctx.moveTo(shape.points[0].x, shape.points[0].y);
  for (let index = 1; index < shape.points.length; index += 1) {
    ctx.lineTo(shape.points[index].x, shape.points[index].y);
  }
  if (shape.closed) {
    ctx.closePath();
    if (shape.fillOn) {
      ctx.fill();
    }
  }
  ctx.stroke();
}

function drawArrow(ctx, shape) {
  const angle = Math.atan2(shape.y2 - shape.y1, shape.x2 - shape.x1);
  const head = shape.headSize || 18;
  ctx.beginPath();
  ctx.moveTo(shape.x1, shape.y1);
  ctx.lineTo(shape.x2, shape.y2);
  ctx.stroke();
  const left = {
    x: shape.x2 - head * Math.cos(angle - 0.4),
    y: shape.y2 - head * Math.sin(angle - 0.4),
  };
  const right = {
    x: shape.x2 - head * Math.cos(angle + 0.4),
    y: shape.y2 - head * Math.sin(angle + 0.4),
  };
  if (shape.head === "chevron") {
    ctx.beginPath();
    ctx.moveTo(left.x, left.y);
    ctx.lineTo(shape.x2, shape.y2);
    ctx.lineTo(right.x, right.y);
    ctx.stroke();
    return;
  }
  ctx.beginPath();
  ctx.moveTo(shape.x2, shape.y2);
  ctx.lineTo(left.x, left.y);
  if (shape.head === "barbed") {
    ctx.lineTo(
      shape.x2 - head * 0.55 * Math.cos(angle),
      shape.y2 - head * 0.55 * Math.sin(angle),
    );
  }
  ctx.lineTo(right.x, right.y);
  ctx.closePath();
  ctx.fillStyle = shape.stroke || "#ff3b30";
  ctx.fill();
}

function drawTextBox(ctx, shape) {
  ctx.fillStyle = shape.color || "#1a1c1f";
  ctx.font =
    (shape.size || 22) + "px \"" + (shape.font || "Malgun Gothic") + "\"";
  ctx.textBaseline = "top";
  const lines = String(shape.text || "").split("\n");
  const lineHeight = (shape.size || 22) * 1.25;
  for (let index = 0; index < lines.length; index += 1) {
    ctx.fillText(lines[index], shape.x, shape.y + index * lineHeight);
  }
}

function drawHandles(ctx, shape) {
  ctx.save();
  ctx.fillStyle = "#fff";
  ctx.strokeStyle = "#111";
  ctx.lineWidth = 1;
  for (const handle of shapeHandles(shape)) {
    ctx.beginPath();
    ctx.rect(
      handle.x - HANDLE_SIZE / 2,
      handle.y - HANDLE_SIZE / 2,
      HANDLE_SIZE,
      HANDLE_SIZE,
    );
    ctx.fill();
    ctx.stroke();
  }
  ctx.restore();
}

async function saveEditor() {
  if (!editorPath || !editorPicture) {
    return;
  }
  commitEditorText();
  if (editorDraft && editorDraft.kind === KIND_POLYGON) {
    closePolygonDraft();
  }
  const selected = editorSelected;
  editorSelected = -1;
  paintEditor();
  editorSelected = selected;
  try {
    const blob = await new Promise((resolve, reject) => {
      editorCanvas.toBlob((value) => {
        if (value) {
          resolve(value);
        } else {
          reject(new Error("blob"));
        }
      }, "image/png");
    });
    const png = Array.from(new Uint8Array(await blob.arrayBuffer()));
    await core.invoke("save_edited_shot", {
      source: editorPath,
      png,
    });
    closeEditor();
    await reload(true);
  } catch (failure) {
    editorError.textContent = String(failure);
    paintEditor();
  }
}

boot();
