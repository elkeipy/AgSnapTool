const core = window.__TAURI__.core;
const events = window.__TAURI__.event;
const DEFAULT_GALLERY_ZOOM = 220;
const GALLERY_ZOOM_MIN = 140;
const GALLERY_ZOOM_MAX = 400;
const GALLERY_ZOOM_STEP = 20;
const CARD_MARGIN_X = 16;
const PEEK_DELAY_MS = 500;
const PEEK_SCALE = 1.5;
const PEEK_PAD = 12;
const PEEK_SCALE_TAGS = [
  "1.2",
  "1.3",
  "1.5",
  "2.0",
  "2.5",
  "3.0",
  "3.5",
  "4.0",
  "max",
];
const FRAME_SCALE_MIN = 0.45;
const TAPE_COLORS = ["yellow", "red", "blue"];

const VIEW_ZOOM_MIN = 0.25;
const VIEW_ZOOM_MAX = 8;
const VIEW_ZOOM_STEP = 0.15;
const VIEW_FIT_PAD = 24;
const PANE_OPEN_WIDTH = 210;
const PANE_CLOSED_WIDTH = 44;
const PRESS_SCALE = 0.97;
const SPRING_SETTLE = { damping: 1, duration: 0.4 };
const SPRING_BOUNCE = { damping: 0.8, duration: 0.3 };
const SWITCH_IDS = [
  "control",
  "shift",
  "alt",
  "polaroid",
  "tape",
  "peek-one",
  "peek-two",
  "delete-old",
];
const KIND_RECT = "rect";
const KIND_ELLIPSE = "ellipse";
const KIND_POLYGON = "polygon";
const KIND_ARROW = "arrow";
const KIND_TEXT = "text";
const TOOL_SELECT = "select";
const MIN_SHAPE = 4;
const HANDLE_SIZE = 8;
const TEXT_BOX_WIDTH = 240;
const EDITOR_ZOOM_MIN = VIEW_ZOOM_MIN;
const EDITOR_ZOOM_MAX = VIEW_ZOOM_MAX;
const EDITOR_ZOOM_STEP = VIEW_ZOOM_STEP;
const EDITOR_HISTORY_LIMIT = 80;

let shell = null;
let shots = [];
let filtered = [];
let shown = 0;
let anchor = null;
let lensView = { zoom: 1, panX: 0, panY: 0 };
let lensPath = null;
let lensPan = null;
let lensMoved = false;
const selected = new Set();
const springRuns = new WeakMap();
let paneOpen = null;
let paneSpringTarget = null;
let paneMotion = 0;
let editorPath = null;
let editorUrl = null;
let editorPicture = null;
let editorShapes = [];
let editorSelection = [];
let editorClipboard = [];
let editorPointer = { x: 0, y: 0 };
let editorTool = TOOL_SELECT;
let editorDraft = null;
let editorDrag = null;
let isEditorWindow = false;
let isLensWindow = false;
let editorZoom = 1;
let editorPanX = 0;
let editorPanY = 0;
let editorPan = null;
let editorDrawing = false;
let editorDirty = false;
let editorAllowClose = false;
let editorAskResolve = null;
let spaceHeld = false;
let editorUndo = [];
let editorRedo = [];
const peekTimers = new WeakMap();
const peekCards = new Set();
const peekBlocked = new WeakSet();
let zoomLock = 0;

const day = document.querySelector("#day");
const search = document.querySelector("#search");
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
const editorAsk = document.querySelector("#editor-ask");

async function boot() {
  applyWindowMode(currentWindowLabel(), "");
  if (isEditorWindow) {
    editor.hidden = false;
  }
  if (isLensWindow) {
    lens.hidden = false;
  }
  try {
    if (!isEditorWindow && !isLensWindow) {
      applyWindowMode(currentWindowLabel(), await core.invoke("window_role"));
    }
    shell = await core.invoke("shell_state");
    applyShell();
  } catch (failure) {
    if (editorError) {
      editorError.textContent = String(failure);
    }
  }
  document.addEventListener("keyup", onKeyUp);
  if (isEditorWindow) {
    await bootEditorWindow();
    return;
  }
  if (isLensWindow) {
    await bootLensWindow();
    return;
  }
  await reload(true);
  await events.listen("gallery-refresh", () => reload(false));
  day.addEventListener("change", () => paint(true));
  scroller.addEventListener("scroll", () => {
    hideMenu();
    clearAllPeeks();
    loadIfNearEnd();
  });
  scroller.addEventListener("wheel", onGalleryWheel, { passive: false });
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
  search.addEventListener("input", () => paint(false));
  document
    .querySelector("#delete-old")
    .addEventListener("change", saveDeleteOld);
  document.querySelector("#language").addEventListener("change", saveLanguage);
  document.querySelector("#theme").addEventListener("change", saveTheme);
  document
    .querySelector("#polaroid")
    .addEventListener("change", savePolaroid);
  document.querySelector("#tilt").addEventListener("change", saveTilt);
  document
    .querySelector("#caption-size")
    .addEventListener("change", saveCaptionSize);
  document.querySelector("#tape").addEventListener("change", saveTape);
  document.querySelector("#peek-one").addEventListener("change", savePeek);
  document.querySelector("#peek-two").addEventListener("change", savePeek);
  document
    .querySelector("#peek-one-delay")
    .addEventListener("change", savePeek);
  document
    .querySelector("#peek-two-delay")
    .addEventListener("change", savePeek);
  document
    .querySelector("#peek-one-scale")
    .addEventListener("change", savePeek);
  document
    .querySelector("#peek-two-scale")
    .addEventListener("change", savePeek);
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
  document.querySelector("#menu-md-copy").addEventListener("click", () => {
    hideMenu();
    markdownSelection(false);
  });
  document.querySelector("#menu-md-save").addEventListener("click", () => {
    hideMenu();
    markdownSelection(true);
  });
  document.querySelector("#ocr-ask-close").addEventListener("click", hideOcrAsk);
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
      core.invoke("show_editor", { path: lensPath });
    }
  });
  document.querySelector("#menu-memo").addEventListener("click", () => {
    hideMenu();
    if (lensPath) {
      openNoteAsk(lensPath);
    }
  });
  document.querySelector("#note-save").addEventListener("click", saveNoteAsk);
  document.querySelector("#note-cancel").addEventListener("click", hideNoteAsk);
  document.querySelector("#note-memo").addEventListener("keydown", onNoteAskKey);
  document.querySelector("#note-tags").addEventListener("keydown", onNoteAskKey);
  document.querySelector("#menu-delete").addEventListener("click", () => {
    hideMenu();
    deleteSelection();
  });
  bindLens();
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
  document.querySelector("#delete-old").checked = shell.delete_old;
  document.querySelector("#days").disabled = !shell.delete_old;
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
  document.querySelector("#caption-label").textContent = shell.caption_label;
  document.querySelector("#caption-size").disabled = !shell.polaroid;
  document.querySelector("#caption-size").value = String(shell.caption_size);
  board.style.setProperty("--caption-size", shell.caption_size + "px");
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
  applyGalleryZoomSize(galleryZoom());
  applyPeekShell();
  board.classList.toggle("bare", !tapeLive);
  document.querySelector("#menu-copy").textContent = shell.copy_image;
  document.querySelector("#menu-md-copy").textContent = shell.md_copy;
  document.querySelector("#menu-md-save").textContent = shell.md_save;
  document.querySelector("#ocr-ask-close").textContent = shell.close_label;
  document.querySelector("#menu-lens").textContent = shell.magnify_image;
  document.querySelector("#menu-open").textContent = shell.open_with;
  document.querySelector("#menu-edit").textContent = shell.edit_image;
  document.querySelector("#menu-memo").textContent = shell.memo_image;
  document.querySelector("#note-memo-label").textContent = shell.memo_label;
  document.querySelector("#note-tags-label").textContent = shell.tags_label;
  document.querySelector("#note-save").textContent = shell.editor_save;
  document.querySelector("#note-cancel").textContent = shell.editor_cancel;
  search.placeholder = shell.search_placeholder;
  document.querySelector("#menu-delete").textContent = shell.delete_image;
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
  const width = scroller.clientWidth || columnWidth();
  const height = scroller.clientHeight || cardMinHeight() * 2;
  const columns = Math.max(1, Math.floor(width / columnWidth()));
  const rows = Math.max(2, Math.ceil(height / cardMinHeight()));
  return columns * rows;
}

function paint(reset) {
  const query = search.value;
  filtered = shots.filter(
    (shot) =>
      (day.value === "" || shot.day === day.value) &&
      shotMatchesQuery(shot, query),
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
  if (remain < cardMinHeight()) {
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
    if (day.value === "" && isGalleryDayBreak(lastShownDay(), shot.day)) {
      board.insertBefore(dateRule(shot.day), marker);
    }
    board.insertBefore(card(shot), marker);
    shown += 1;
  }
  if (shown >= filtered.length) {
    marker.remove();
  }
}

function lastShownDay() {
  if (shown === 0) {
    return null;
  }
  return filtered[shown - 1].day;
}

function isGalleryDayBreak(previous, current) {
  return previous !== current;
}

function dateRule(day) {
  const rule = document.createElement("div");
  rule.className = "date-rule";
  const label = document.createElement("span");
  label.className = "date-rule-label";
  label.textContent = day;
  const line = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  line.setAttribute("class", "date-rule-line");
  line.setAttribute("viewBox", "0 0 400 12");
  line.setAttribute("preserveAspectRatio", "none");
  const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
  path.setAttribute("d", dateRulePath(day));
  line.append(path);
  rule.append(label, line);
  return rule;
}

function dateRulePath(day) {
  let seed = 2166136261;
  for (const letter of day) {
    seed ^= letter.charCodeAt(0);
    seed = Math.imul(seed, 16777619);
  }
  const parts = [];
  for (let index = 0; index <= 12; index += 1) {
    seed = Math.imul(seed ^ (seed >>> 13), 1274126177);
    const x = (400 * index) / 12;
    const y = 6 + (((seed >>> 8) % 7) - 3);
    if (index === 0) {
      parts.push(`M${x.toFixed(1)},${y}`);
      continue;
    }
    const control = 6 + (((seed >>> 16) % 5) - 2);
    parts.push(`Q${(x - 16).toFixed(1)},${control} ${x.toFixed(1)},${y}`);
  }
  return parts.join(" ");
}

function card(shot) {
  const button = document.createElement("button");
  button.className = "card";
  button.type = "button";
  button.dataset.path = shot.path;
  if (selected.has(shot.path)) {
    button.classList.add("selected");
  }
  const frame = document.createElement("span");
  frame.className = "frame";
  frame.style.setProperty("--tilt", tiltDegrees(shot.path) + "deg");
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
  const plate = document.createElement("span");
  plate.className = "plate";
  const tags = shotTags(shot);
  if (tags.length > 0) {
    const row = document.createElement("span");
    row.className = "note-tags";
    for (const tag of tags) {
      const chip = document.createElement("span");
      chip.className = "note-tag";
      chip.textContent = tag;
      row.append(chip);
    }
    plate.append(row);
  }
  if (shot.memo) {
    const note = document.createElement("span");
    note.className = "note";
    note.textContent = shot.memo;
    plate.append(note);
  }
  plate.append(caption);
  if (tags.length > 0 || shot.memo) {
    frame.classList.add("has-meta");
  }
  frame.append(...tapes, image, plate);
  button.append(frame);
  bindFrameScale(image, frame);
  bindCardPeek(button);
  button.addEventListener("click", (event) => selectShot(shot.path, event));
  button.addEventListener("dblclick", (event) => {
    if (event.shiftKey) {
      core.invoke("show_lens", { path: shot.path });
      return;
    }
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

let ocrEnglishHintShown = false;

function showOcrAsk(message) {
  document.querySelector("#ocr-ask-text").textContent = message;
  document.querySelector("#ocr-ask").hidden = false;
}

function hideOcrAsk() {
  document.querySelector("#ocr-ask").hidden = true;
}

function shouldShowOcrNotice(result) {
  if (!result || !result.notice) {
    return false;
  }
  if (!result.has_korean) {
    return true;
  }
  if (!result.has_english && !ocrEnglishHintShown) {
    ocrEnglishHintShown = true;
    return true;
  }
  return false;
}

function markdownSelection(save) {
  const paths = chosenPaths();
  if (paths.length === 0) {
    return;
  }
  const name = save ? "save_shot_markdown" : "copy_shot_markdown";
  core.invoke(name, { paths })
    .then((result) => {
      if (shouldShowOcrNotice(result)) {
        showOcrAsk(result.notice);
      }
    })
    .catch((failure) => {
      showOcrAsk(String(failure));
    });
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

function deleteSelection() {
  const paths = chosenPaths();
  if (paths.length === 0) {
    return;
  }
  if (shell && shell.delete_confirm && !window.confirm(shell.delete_confirm)) {
    return;
  }
  core.invoke("delete_shots", { paths }).catch((failure) => {
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

function runMenuShortcut(key) {
  const action = {
    t: "menu-memo",
    c: "menu-copy",
    m: "menu-md-copy",
    s: "menu-md-save",
    v: "menu-lens",
    e: "menu-edit",
    d: "menu-delete",
  }[key.toLowerCase()];
  if (!action) {
    return false;
  }
  document.querySelector("#" + action).click();
  return true;
}

function shotTags(shot) {
  return Array.isArray(shot.tags) ? shot.tags : [];
}

function shotMatchesQuery(shot, query) {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return true;
  }
  if ((shot.memo || "").toLowerCase().includes(needle)) {
    return true;
  }
  if (shotTags(shot).some((tag) => tag.includes(needle))) {
    return true;
  }
  if ((shot.taken || "").toLowerCase().includes(needle)) {
    return true;
  }
  if ((shot.day || "").toLowerCase().includes(needle)) {
    return true;
  }
  const name = shot.path.split(/[/\\]/).pop() || "";
  return name.toLowerCase().includes(needle);
}

function openNoteAsk(path) {
  const shot = shots.find((item) => item.path === path);
  document.querySelector("#note-memo").value = shot ? shot.memo || "" : "";
  document.querySelector("#note-tags").value = shot
    ? shotTags(shot).join(", ")
    : "";
  document.querySelector("#note-ask").dataset.path = path;
  document.querySelector("#note-ask").hidden = false;
  document.querySelector("#note-tags").focus();
}

function hideNoteAsk() {
  const ask = document.querySelector("#note-ask");
  ask.hidden = true;
  delete ask.dataset.path;
}

function onNoteAskKey(event) {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  saveNoteAsk();
}

async function saveNoteAsk() {
  const ask = document.querySelector("#note-ask");
  const path = ask.dataset.path;
  if (!path) {
    hideNoteAsk();
    return;
  }
  const memo = document.querySelector("#note-memo").value;
  const tags = document
    .querySelector("#note-tags")
    .value.split(",")
    .map((tag) => tag.trim())
    .filter((tag) => tag);
  const note = await core.invoke("set_shot_note", { path, memo, tags });
  const shot = shots.find((item) => item.path === path);
  if (shot) {
    shot.memo = note.memo;
    shot.tags = note.tags;
  }
  hideNoteAsk();
  paint(false);
}

function galleryZoom() {
  const stored = shell ? Number(shell.gallery_zoom) : DEFAULT_GALLERY_ZOOM;
  return nextGalleryZoom(
    stored || DEFAULT_GALLERY_ZOOM,
    0,
    GALLERY_ZOOM_MIN,
    GALLERY_ZOOM_MAX,
    GALLERY_ZOOM_STEP,
  );
}

function columnWidth() {
  return galleryZoom() + CARD_MARGIN_X;
}

function cardMinHeight() {
  return Math.round(galleryZoom() * 1.18);
}

function applyGalleryZoomSize(size) {
  if (shell) {
    shell.gallery_zoom = size;
  }
  board.style.setProperty("--card-size", size + "px");
}

async function persistGalleryZoom(size) {
  shell = await core.invoke("set_gallery_zoom", { pixels: size });
  applyGalleryZoomSize(galleryZoom());
}

function onGalleryWheel(event) {
  hideMenu();
  if (!event.ctrlKey) {
    return;
  }
  clearAllPeeks();
  event.preventDefault();
  const now = Date.now();
  if (now - zoomLock < 40) {
    return;
  }
  zoomLock = now;
  const next = nextGalleryZoom(
    galleryZoom(),
    event.deltaY,
    GALLERY_ZOOM_MIN,
    GALLERY_ZOOM_MAX,
    GALLERY_ZOOM_STEP,
  );
  if (next === galleryZoom()) {
    return;
  }
  applyGalleryZoomSize(next);
  persistGalleryZoom(next);
  requestAnimationFrame(fillVisible);
}

function nextGalleryZoom(current, deltaY, min, max, step) {
  const direction = deltaY === 0 ? 0 : deltaY > 0 ? -1 : 1;
  const next = current + direction * step;
  return Math.min(max, Math.max(min, next));
}

function galleryFrameScale(naturalWidth, naturalHeight) {
  if (naturalWidth <= 0 || naturalHeight <= 0) {
    return 1;
  }
  const ratio = naturalHeight / naturalWidth;
  if (ratio >= 1) {
    return 1;
  }
  return Math.max(FRAME_SCALE_MIN, ratio);
}

function bindFrameScale(image, frame) {
  const apply = () => {
    frame.style.setProperty(
      "--frame-scale",
      String(galleryFrameScale(image.naturalWidth, image.naturalHeight)),
    );
  };
  if (image.complete && image.naturalWidth > 0) {
    apply();
    return;
  }
  image.addEventListener("load", apply);
}

function layoutBox(host, width, height) {
  return {
    left: host.left + (host.width - width) / 2,
    top: host.top + (host.height - height) / 2,
    width,
    height,
  };
}

function unscaledFrameBox(frame) {
  return layoutBox(
    frame.parentElement.getBoundingClientRect(),
    frame.offsetWidth,
    frame.offsetHeight,
  );
}

function peekFitScale(box, view, pad) {
  const viewWidth = Math.max(1, view.right - view.left - pad * 2);
  const viewHeight = Math.max(1, view.bottom - view.top - pad * 2);
  if (box.width <= 0 || box.height <= 0) {
    return 1;
  }
  return Math.max(
    1,
    Math.min(viewWidth / box.width, viewHeight / box.height),
  );
}

function peekShift(box, view, scale, pad) {
  const width = box.width * scale;
  const height = box.height * scale;
  const centerX = box.left + box.width / 2;
  const centerY = box.top + box.height / 2;
  const left = centerX - width / 2;
  const top = centerY - height / 2;
  const viewLeft = view.left + pad;
  const viewTop = view.top + pad;
  const viewRight = view.right - pad;
  const viewBottom = view.bottom - pad;
  const viewWidth = viewRight - viewLeft;
  const viewHeight = viewBottom - viewTop;
  let x = 0;
  let y = 0;
  if (width >= viewWidth) {
    x = (viewLeft + viewRight) / 2 - centerX;
  } else if (left < viewLeft) {
    x = viewLeft - left;
  } else if (left + width > viewRight) {
    x = viewRight - left - width;
  }
  if (height >= viewHeight) {
    y = (viewTop + viewBottom) / 2 - centerY;
  } else if (top < viewTop) {
    y = viewTop - top;
  } else if (top + height > viewBottom) {
    y = viewBottom - top - height;
  }
  return { x, y };
}

function clearPeek(button) {
  const timer = peekTimers.get(button);
  if (timer) {
    clearTimeout(timer);
    peekTimers.delete(button);
  }
}

function releasePeek(button) {
  clearPeek(button);
  peekCards.delete(button);
  button.classList.remove("peek");
  button.style.removeProperty("--peek");
  button.style.removeProperty("--peek-x");
  button.style.removeProperty("--peek-y");
}

function clearAllPeeks() {
  for (const button of [...peekCards]) {
    releasePeek(button);
  }
}

function freezePeeks() {
  for (const button of [...peekCards]) {
    peekBlocked.add(button);
    releasePeek(button);
  }
}

function applyPeek(button, scale) {
  const frame = button.querySelector(".frame");
  const box = unscaledFrameBox(frame);
  const shift = peekShift(
    box,
    scroller.getBoundingClientRect(),
    scale,
    PEEK_PAD,
  );
  button.style.setProperty("--peek", String(scale));
  button.style.setProperty("--peek-x", shift.x + "px");
  button.style.setProperty("--peek-y", shift.y + "px");
  button.classList.add("peek");
}

function peekScaleOptionsAfter(tag) {
  const index = PEEK_SCALE_TAGS.indexOf(tag);
  if (index < 0) {
    return PEEK_SCALE_TAGS.slice(1);
  }
  return PEEK_SCALE_TAGS.slice(index + 1);
}

function peekScaleCaption(tag) {
  if (tag === "max") {
    return shell ? shell.peek_scale_max : "max";
  }
  if (shell && shell.language === "ko") {
    return tag + "배";
  }
  return tag + "×";
}

function peekScaleNumber(tag, box, view) {
  if (tag === "max") {
    return peekFitScale(box, view, PEEK_PAD);
  }
  const value = Number(tag);
  return value > 0 ? value : PEEK_SCALE;
}

function peekOneEnabled() {
  return Boolean(shell && shell.peek_one);
}

function peekTwoEnabled() {
  return (
    peekOneEnabled() &&
    Boolean(shell.peek_two) &&
    peekScaleOptionsAfter(shell.peek_one_scale).length > 0
  );
}

function fillPeekScales(select, tags, current) {
  select.replaceChildren();
  for (const tag of tags) {
    const option = document.createElement("option");
    option.value = tag;
    option.textContent = peekScaleCaption(tag);
    select.append(option);
  }
  select.value = tags.indexOf(current) >= 0 ? current : tags[0] || "";
}

function applyPeekShell() {
  const one = document.querySelector("#peek-one");
  const two = document.querySelector("#peek-two");
  const oneDelay = document.querySelector("#peek-one-delay");
  const twoDelay = document.querySelector("#peek-two-delay");
  const oneScale = document.querySelector("#peek-one-scale");
  const twoScale = document.querySelector("#peek-two-scale");
  document.querySelector("#peek-one-label").textContent = shell.peek_one_label;
  document.querySelector("#peek-two-label").textContent = shell.peek_two_label;
  document.querySelector("#peek-one-delay-label").textContent =
    shell.peek_delay_label;
  document.querySelector("#peek-two-delay-label").textContent =
    shell.peek_delay_label;
  document.querySelector("#peek-one-scale-label").textContent =
    shell.peek_scale_label;
  document.querySelector("#peek-two-scale-label").textContent =
    shell.peek_scale_label;
  one.checked = shell.peek_one;
  two.checked = shell.peek_two;
  oneDelay.value = String(shell.peek_one_delay);
  twoDelay.value = String(shell.peek_two_delay);
  fillPeekScales(oneScale, PEEK_SCALE_TAGS, shell.peek_one_scale);
  fillPeekScales(
    twoScale,
    peekScaleOptionsAfter(shell.peek_one_scale),
    shell.peek_two_scale,
  );
  const twoOk = peekTwoEnabled();
  oneDelay.disabled = !shell.peek_one;
  oneScale.disabled = !shell.peek_one;
  two.disabled =
    !shell.peek_one ||
    peekScaleOptionsAfter(shell.peek_one_scale).length === 0;
  twoDelay.disabled = !twoOk;
  twoScale.disabled = !twoOk;
}

async function savePeek() {
  shell = await core.invoke("set_peek", {
    one: document.querySelector("#peek-one").checked,
    two: document.querySelector("#peek-two").checked,
    oneDelay: Number(document.querySelector("#peek-one-delay").value),
    twoDelay: Number(document.querySelector("#peek-two-delay").value),
    oneScale: document.querySelector("#peek-one-scale").value,
    twoScale: document.querySelector("#peek-two-scale").value,
  });
  applyShell();
}

function bindCardPeek(button) {
  button.addEventListener("pointerenter", () => {
    if (peekBlocked.has(button) || !peekOneEnabled()) {
      return;
    }
    clearPeek(button);
    peekCards.add(button);
    const first = setTimeout(() => {
      const box = unscaledFrameBox(button.querySelector(".frame"));
      const view = scroller.getBoundingClientRect();
      applyPeek(button, peekScaleNumber(shell.peek_one_scale, box, view));
      if (!peekTwoEnabled()) {
        return;
      }
      const second = setTimeout(() => {
        const nextBox = unscaledFrameBox(button.querySelector(".frame"));
        applyPeek(
          button,
          peekScaleNumber(
            shell.peek_two_scale,
            nextBox,
            scroller.getBoundingClientRect(),
          ),
        );
      }, Number(shell.peek_two_delay) || PEEK_DELAY_MS);
      peekTimers.set(button, second);
    }, Number(shell.peek_one_delay) || PEEK_DELAY_MS);
    peekTimers.set(button, first);
  });
  button.addEventListener("pointerleave", () => {
    peekBlocked.delete(button);
    releasePeek(button);
  });
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

function clampViewZoom(value) {
  return Math.min(
    VIEW_ZOOM_MAX,
    Math.max(VIEW_ZOOM_MIN, Math.round(value * 100) / 100),
  );
}

function zoomViewAt(view, nextZoom, pointerX, pointerY) {
  const zoom = clampViewZoom(nextZoom);
  const worldX = (pointerX - view.panX) / view.zoom;
  const worldY = (pointerY - view.panY) / view.zoom;
  return {
    zoom: zoom,
    panX: pointerX - worldX * zoom,
    panY: pointerY - worldY * zoom,
  };
}

function fitView(contentW, contentH, stageW, stageH) {
  if (stageW <= 0 || stageH <= 0 || contentW <= 0 || contentH <= 0) {
    return { zoom: 1, panX: 0, panY: 0 };
  }
  const zoom = clampViewZoom(
    Math.min(
      (stageW - VIEW_FIT_PAD) / contentW,
      (stageH - VIEW_FIT_PAD) / contentH,
      1,
    ),
  );
  return {
    zoom: zoom,
    panX: (stageW - contentW * zoom) / 2,
    panY: (stageH - contentH * zoom) / 2,
  };
}

function applyViewTransform(element, view) {
  element.style.transform =
    "translate(" +
    view.panX +
    "px," +
    view.panY +
    "px) scale(" +
    view.zoom +
    ")";
}

function viewAfterWheel(view, event, host) {
  const box = host.getBoundingClientRect();
  const step = event.deltaY < 0 ? VIEW_ZOOM_STEP : -VIEW_ZOOM_STEP;
  return zoomViewAt(
    view,
    view.zoom + step,
    event.clientX - box.left,
    event.clientY - box.top,
  );
}

function beginViewPan(view, event) {
  return {
    x: event.clientX - view.panX,
    y: event.clientY - view.panY,
    pointer: event.pointerId,
  };
}

function moveViewPan(view, pan, event) {
  return {
    zoom: view.zoom,
    panX: event.clientX - pan.x,
    panY: event.clientY - pan.y,
  };
}

function setLensView(view) {
  lensView = view;
  applyViewTransform(lensImage, lensView);
}

function onLensDown(event) {
  if (event.button !== 0) {
    return;
  }
  event.preventDefault();
  lensMoved = false;
  lensPan = beginViewPan(lensView, event);
  lens.setPointerCapture(event.pointerId);
  lensImage.style.cursor = "grabbing";
}

function onLensMove(event) {
  if (!lensPan || lensPan.pointer !== event.pointerId) {
    return;
  }
  const next = moveViewPan(lensView, lensPan, event);
  if (next.panX !== lensView.panX || next.panY !== lensView.panY) {
    lensMoved = true;
  }
  setLensView(next);
  lensImage.style.cursor = "grabbing";
}

function onLensUp(event) {
  if (!lensPan || lensPan.pointer !== event.pointerId) {
    return;
  }
  lensPan = null;
  lensImage.style.cursor = "grab";
  if (!lensMoved && event.target === lens) {
    hideLens();
  }
}

function resetLensView() {
  setLensView(
    fitView(
      lensImage.naturalWidth,
      lensImage.naturalHeight,
      lens.clientWidth,
      lens.clientHeight,
    ),
  );
}

function openLens(path) {
  lensPan = null;
  lensMoved = false;
  lens.hidden = false;
  lensImage.onload = resetLensView;
  lensImage.src = core.convertFileSrc(path);
  if (lensImage.complete) {
    resetLensView();
  }
}

function hideLens() {
  lensPan = null;
  if (isLensWindow) {
    core.invoke("hide_lens");
    return;
  }
  lens.hidden = true;
}

function bindLens() {
  lens.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      setLensView(viewAfterWheel(lensView, event, lens));
    },
    { passive: false },
  );
  lens.addEventListener("pointerdown", onLensDown);
  lens.addEventListener("pointermove", onLensMove);
  lens.addEventListener("pointerup", onLensUp);
  lens.addEventListener("pointercancel", onLensUp);
}

async function bootLensWindow() {
  lens.hidden = false;
  bindLens();
  document.addEventListener("keydown", onKeyDown);
  try {
    const pending = await core.invoke("take_lens_path");
    if (pending) {
      openLens(pending);
    }
  } catch (failure) {
    if (editorError) {
      editorError.textContent = String(failure);
    }
  }
}

function isTyping(target) {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement
  );
}

function onKeyDown(event) {
  if (isEditorWindow) {
    if (event.code === "Space" && !isTyping(event.target)) {
      spaceHeld = true;
      event.preventDefault();
      return;
    }
    if (
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === "s"
    ) {
      event.preventDefault();
      saveEditor();
      return;
    }
    if (
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === "c" &&
      !isTyping(event.target)
    ) {
      event.preventDefault();
      copySelectedShapes();
      return;
    }
    if (
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === "v" &&
      !isTyping(event.target)
    ) {
      event.preventDefault();
      pasteShapesAt(editorPointer);
      return;
    }
    if (
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === "z" &&
      !isTyping(event.target)
    ) {
      event.preventDefault();
      if (event.shiftKey) {
        redoEditor();
      } else {
        undoEditor();
      }
      return;
    }
    if (
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === "y" &&
      !isTyping(event.target)
    ) {
      event.preventDefault();
      redoEditor();
      return;
    }
    handleEditorKey(event);
    return;
  }
  if (isLensWindow) {
    if (event.key === "Escape") {
      hideLens();
    }
    return;
  }
  if (event.key === "Escape") {
    hideHistory();
    hideMenu();
    hideLens();
    hideNoteAsk();
    hideOcrAsk();
    freezePeeks();
    return;
  }
  if (!menu.hidden && !isTyping(event.target) && runMenuShortcut(event.key)) {
    event.preventDefault();
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

async function saveDeleteOld() {
  const enabled = document.querySelector("#delete-old").checked;
  shell = await core.invoke("set_delete_old", { enabled });
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

async function saveCaptionSize() {
  const pixels = Number(document.querySelector("#caption-size").value);
  shell = await core.invoke("set_caption_size", { pixels });
  applyShell();
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
  nameTool("edit-select", shell.editor_select);
  nameTool("edit-rect", shell.editor_rect);
  nameTool("edit-ellipse", shell.editor_ellipse);
  nameTool("edit-polygon", shell.editor_polygon);
  nameTool("edit-arrow", shell.editor_arrow);
  nameTool("edit-text", shell.editor_text);
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
  document.querySelector("#editor-ask-text").textContent =
    shell.editor_unsaved;
  document.querySelector("#editor-ask-save").textContent = shell.editor_save;
  document.querySelector("#editor-ask-discard").textContent =
    shell.editor_discard;
  document.querySelector("#editor-ask-cancel").textContent =
    shell.editor_cancel;
  const head = document.querySelector("#edit-head");
  head.options[0].textContent = shell.editor_head_triangle;
  head.options[1].textContent = shell.editor_head_chevron;
  head.options[2].textContent = shell.editor_head_barbed;
}

async function bootEditorWindow() {
  editor.hidden = false;
  bindEditor();
  document.addEventListener("keydown", onKeyDown);
  document.addEventListener("pointermove", (event) => {
    editorPointer = editorPoint(event);
  });
  try {
    await events.listen("editor-open", (event) => {
      openEditor(event.payload);
    });
    const pending = await core.invoke("take_editor_path");
    if (pending) {
      await openEditor(pending);
    }
  } catch (failure) {
    editorError.textContent = String(failure);
  }
}

function nameTool(id, title) {
  const button = document.querySelector("#" + id);
  button.title = title;
  button.setAttribute("aria-label", title);
}

function bindEditor() {
  for (const button of editor.querySelectorAll("[data-tool]")) {
    button.addEventListener("click", () => {
      setEditorTool(button.dataset.tool);
    });
  }
  document.querySelector("#edit-save").addEventListener("click", saveEditor);
  document.querySelector("#edit-cancel").addEventListener(
    "click",
    closeEditor,
  );
  document.querySelector(".editor-stage").addEventListener(
    "wheel",
    onEditorWheel,
    { passive: false },
  );
  editorText.addEventListener("keydown", onEditorTextKey);
  editorCanvas.addEventListener("pointerdown", onEditorDown);
  editorCanvas.addEventListener("pointermove", onEditorMove);
  editorCanvas.addEventListener("pointerup", onEditorUp);
  editorCanvas.addEventListener("pointercancel", onEditorUp);
  editorCanvas.addEventListener("dblclick", onEditorDouble);
  editorText.addEventListener("blur", (event) => {
    if (event.relatedTarget && editor.contains(event.relatedTarget)) {
      return;
    }
    commitEditorText();
  });
  for (const id of [
    "edit-stroke",
    "edit-stroke-on",
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

function onKeyUp(event) {
  if (event.code === "Space") {
    spaceHeld = false;
  }
}

function currentWindowLabel() {
  try {
    const webview = window.__TAURI__.webviewWindow;
    if (webview && typeof webview.getCurrentWebviewWindow === "function") {
      return webview.getCurrentWebviewWindow().label || "";
    }
  } catch (_err) {
    return "";
  }
  return "";
}

function isEditorLabel(label) {
  return label === "editor" || label.indexOf("editor-") === 0;
}

function isLensLabel(label) {
  return label === "lens" || label.indexOf("lens-") === 0;
}

function applyWindowMode(label, role) {
  isEditorWindow = isEditorLabel(label) || role === "editor";
  isLensWindow = isLensLabel(label) || role === "lens";
  if (isEditorWindow) {
    document.documentElement.dataset.mode = "editor";
    return;
  }
  if (isLensWindow) {
    document.documentElement.dataset.mode = "lens";
    return;
  }
  document.documentElement.dataset.mode = "gallery";
}

function clampEditorZoom(value) {
  return clampViewZoom(value);
}

function zoomEditorView(view, nextZoom, pointerX, pointerY) {
  return zoomViewAt(view, nextZoom, pointerX, pointerY);
}

function editorView() {
  return { zoom: editorZoom, panX: editorPanX, panY: editorPanY };
}

function setEditorView(view) {
  editorZoom = view.zoom;
  editorPanX = view.panX;
  editorPanY = view.panY;
  applyEditorView();
}

function applyEditorView() {
  applyViewTransform(editorCanvas, editorView());
  if (!editorText.hidden && editorDraft && editorDraft.kind === KIND_TEXT) {
    placeTextInput(editorDraft);
  }
}

function resetEditorView() {
  const stage = editor.querySelector(".editor-stage");
  setEditorView(
    fitView(
      editorCanvas.width,
      editorCanvas.height,
      stage ? stage.clientWidth : 0,
      stage ? stage.clientHeight : 0,
    ),
  );
}

function onEditorWheel(event) {
  if (event.target === editorText) {
    return;
  }
  event.preventDefault();
  const stage = editor.querySelector(".editor-stage");
  setEditorView(viewAfterWheel(editorView(), event, stage));
}

function onEditorTextKey(event) {
  if (event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  if (event.ctrlKey || event.metaKey) {
    insertTextNewline();
    return;
  }
  commitEditorText();
}

function insertTextNewline() {
  const start = editorText.selectionStart;
  const end = editorText.selectionEnd;
  const value = editorText.value;
  editorText.value = value.slice(0, start) + "\n" + value.slice(end);
  editorText.selectionStart = start + 1;
  editorText.selectionEnd = start + 1;
}

function handleEditorKey(event) {
  if (event.key === "Escape") {
    if (!editorText.hidden) {
      commitEditorText();
    }
    editorDraft = null;
    setEditorTool(TOOL_SELECT);
    return true;
  }
  if (isTyping(event.target)) {
    return true;
  }
  if (event.key === "Enter" && editorDraft && editorDraft.kind === KIND_POLYGON) {
    event.preventDefault();
    closePolygonDraft();
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
      editorSelection = [];
      editorUndo = [];
      editorRedo = [];
      editorDraft = null;
      editorDrag = null;
      editorDrawing = false;
      editorPan = null;
      editorDirty = false;
      editorAllowClose = false;
      editorCanvas.width = picture.naturalWidth;
      editorCanvas.height = picture.naturalHeight;
      resetEditorView();
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

function editorHistoryPush(undo, redo, snapshot, limit) {
  const nextUndo = undo.concat([snapshot]);
  if (nextUndo.length > limit) {
    nextUndo.shift();
  }
  return { undo: nextUndo, redo: [] };
}

function editorHistoryUndo(undo, redo, current) {
  if (undo.length === 0) {
    return null;
  }
  const nextUndo = undo.slice();
  const snapshot = nextUndo.pop();
  return {
    undo: nextUndo,
    redo: redo.concat([current]),
    snapshot: snapshot,
  };
}

function editorHistoryRedo(undo, redo, current) {
  if (redo.length === 0) {
    return null;
  }
  const nextRedo = redo.slice();
  const snapshot = nextRedo.pop();
  return {
    undo: undo.concat([current]),
    redo: nextRedo,
    snapshot: snapshot,
  };
}

function snapshotEditor() {
  return {
    shapes: editorShapes.map(cloneEditorShape),
    selection: editorSelection.slice(),
  };
}

function restoreEditor(snapshot) {
  editorShapes = snapshot.shapes.map(cloneEditorShape);
  editorSelection = snapshot.selection.filter(
    (index) => index >= 0 && index < editorShapes.length,
  );
  editorDraft = null;
  editorDrag = null;
  editorText.hidden = true;
  syncEditorProps();
  paintEditor();
}

function pushEditorHistory() {
  const next = editorHistoryPush(
    editorUndo,
    editorRedo,
    snapshotEditor(),
    EDITOR_HISTORY_LIMIT,
  );
  editorUndo = next.undo;
  editorRedo = next.redo;
}

function undoEditor() {
  commitEditorText();
  const next = editorHistoryUndo(editorUndo, editorRedo, snapshotEditor());
  if (!next) {
    return;
  }
  editorUndo = next.undo;
  editorRedo = next.redo;
  restoreEditor(next.snapshot);
}

function redoEditor() {
  commitEditorText();
  const next = editorHistoryRedo(editorUndo, editorRedo, snapshotEditor());
  if (!next) {
    return;
  }
  editorUndo = next.undo;
  editorRedo = next.redo;
  restoreEditor(next.snapshot);
}

function markEditorDirty() {
  editorDirty = true;
}

function hasEditorChanges() {
  return editorDirty || Boolean(editorDraft) || !editorText.hidden;
}

function closeEditor() {
  editorPath = null;
  editorPicture = null;
  editorShapes = [];
  editorSelection = [];
  editorUndo = [];
  editorRedo = [];
  editorDraft = null;
  editorDrag = null;
  editorDrawing = false;
  editorPan = null;
  editorDirty = false;
  editorText.hidden = true;
  editorError.textContent = "";
  forgetEditorUrl();
  if (isEditorWindow) {
    editor.hidden = false;
    editorAllowClose = true;
    core.invoke("hide_editor");
    return;
  }
  editor.hidden = true;
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
    strokeOn: document.querySelector("#edit-stroke-on").checked,
    fillOn: document.querySelector("#edit-fill-on").checked,
    width: Number(document.querySelector("#edit-width").value) || 3,
    head: document.querySelector("#edit-head").value,
    headSize: Number(document.querySelector("#edit-head-size").value) || 18,
    font: document.querySelector("#edit-font").value,
    size: Number(document.querySelector("#edit-font-size").value) || 22,
    color: document.querySelector("#edit-color").value,
  };
}

function selectedIndex() {
  if (editorSelection.length === 0) {
    return -1;
  }
  return editorSelection[editorSelection.length - 1];
}

function selectedShape() {
  return editorShapes[selectedIndex()] || null;
}

function selectedShapes() {
  return editorSelection
    .map((index) => editorShapes[index])
    .filter((shape) => Boolean(shape));
}

function resizeCursorForHandle(id) {
  if (id === "n" || id === "s") {
    return "ns-resize";
  }
  if (id === "e" || id === "w") {
    return "ew-resize";
  }
  if (id === "nw" || id === "se") {
    return "nwse-resize";
  }
  if (id === "ne" || id === "sw") {
    return "nesw-resize";
  }
  if (id) {
    return "pointer";
  }
  return "";
}

function nextEditorSelection(current, index, additive) {
  if (index < 0) {
    return additive ? current.slice() : [];
  }
  if (!additive) {
    return [index];
  }
  if (current.indexOf(index) >= 0) {
    return current.filter((item) => item !== index);
  }
  return current.concat([index]);
}

function pasteGroupOffset(boxes, point) {
  if (boxes.length === 0) {
    return { dx: 0, dy: 0 };
  }
  let left = boxes[0].x;
  let top = boxes[0].y;
  for (const box of boxes) {
    if (box.x < left) {
      left = box.x;
    }
    if (box.y < top) {
      top = box.y;
    }
  }
  return { dx: point.x - left, dy: point.y - top };
}

function setPrimarySelected(index) {
  editorSelection = editorSelection
    .filter((item) => item !== index)
    .concat([index]);
}

function cloneEditorShape(shape) {
  return JSON.parse(JSON.stringify(shape));
}

function copySelectedShapes() {
  editorClipboard = selectedShapes().map(cloneEditorShape);
}

function pasteShapesAt(point) {
  if (editorClipboard.length === 0) {
    return;
  }
  pushEditorHistory();
  const shift = pasteGroupOffset(
    editorClipboard.map(shapeBox),
    point,
  );
  const created = [];
  for (const source of editorClipboard) {
    const shape = cloneEditorShape(source);
    shiftShape(shape, shift.dx, shift.dy);
    editorShapes.push(shape);
    created.push(editorShapes.length - 1);
  }
  editorSelection = created;
  markEditorDirty();
  syncEditorProps();
  paintEditor();
}

function updateEditorHoverCursor(point) {
  if (editorTool !== TOOL_SELECT || editorPan || editorDrag) {
    return;
  }
  const picked = hitEditor(point);
  const resize = resizeCursorForHandle(picked.handle);
  if (resize) {
    editorCanvas.style.cursor = resize;
    return;
  }
  editorCanvas.style.cursor = picked.index >= 0 ? "move" : "";
}

function syncEditorProps() {
  const shape = selectedShape();
  const kind = shape ? shape.kind : editorTool;
  const isBox =
    kind === KIND_RECT || kind === KIND_ELLIPSE || kind === KIND_TEXT;
  const isShape =
    kind === KIND_RECT ||
    kind === KIND_ELLIPSE ||
    kind === KIND_POLYGON ||
    kind === KIND_ARROW;
  const isArrow = kind === KIND_ARROW;
  const isText = kind === KIND_TEXT;
  const hasInk = isShape || isText;
  const hasFill =
    kind === KIND_RECT || kind === KIND_ELLIPSE || kind === KIND_POLYGON;
  document.querySelector("#edit-stroke").closest("label").hidden = !hasInk;
  document.querySelector("#edit-fill").closest("label").hidden = !hasFill;
  document.querySelector("#edit-width").closest("label").hidden = !hasInk;
  document.querySelector("#edit-box-w").closest("label").hidden = !isBox;
  document.querySelector("#edit-head").closest("label").hidden = !isArrow;
  document.querySelector("#edit-head-size").closest("label").hidden =
    !isArrow;
  document.querySelector("#edit-font").closest("label").hidden = !isText;
  document.querySelector("#edit-font-size").closest("label").hidden = !isText;
  document.querySelector("#edit-color").closest("label").hidden = !isText;
  hideEmptyClusters();
  syncInkEnabled();
  if (!shape) {
    return;
  }
  document.querySelector("#edit-stroke-on").checked = shape.strokeOn !== false;
  if (shape.stroke) {
    document.querySelector("#edit-stroke").value = shape.stroke;
  }
  if (shape.fill) {
    document.querySelector("#edit-fill").value = shape.fill;
  }
  document.querySelector("#edit-fill-on").checked = Boolean(shape.fillOn);
  syncInkEnabled();
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

function syncInkEnabled() {
  const strokeOn = document.querySelector("#edit-stroke-on").checked;
  const fillOn = document.querySelector("#edit-fill-on").checked;
  document.querySelector("#edit-stroke").disabled = !strokeOn;
  document.querySelector("#edit-fill").disabled = !fillOn;
}

function hideEmptyClusters() {
  for (const cluster of editor.querySelectorAll("[data-inspect]")) {
    const labels = [...cluster.querySelectorAll("label")];
    cluster.hidden = labels.every((node) => node.hidden);
  }
}

function applyEditorStyle() {
  const shape =
    selectedShape() ||
    (editorDraft && editorDraft.kind === KIND_TEXT ? editorDraft : null);
  if (!shape) {
    paintEditor();
    return;
  }
  if (selectedShape()) {
    pushEditorHistory();
  }
  const style = editorStyle();
  if (shape.kind === KIND_TEXT) {
    shape.font = style.font;
    shape.size = style.size;
    shape.color = style.color;
    shape.stroke = style.stroke;
    shape.strokeOn = style.strokeOn;
    shape.width = style.width;
  } else {
    shape.stroke = style.stroke;
    shape.strokeOn = style.strokeOn;
    shape.fill = style.fill;
    shape.fillOn = style.fillOn;
    shape.width = style.width;
  }
  syncInkEnabled();
  if (shape.kind === KIND_ARROW) {
    shape.head = style.head;
    shape.headSize = style.headSize;
  }
  if (
    shape.kind === KIND_RECT ||
    shape.kind === KIND_ELLIPSE ||
    shape.kind === KIND_TEXT
  ) {
    const width = Number(document.querySelector("#edit-box-w").value);
    const height = Number(document.querySelector("#edit-box-h").value);
    if (width >= MIN_SHAPE) {
      shape.w = width;
    }
    if (height >= MIN_SHAPE) {
      shape.h = height;
    }
  }
  if (shape.kind === KIND_TEXT && !editorText.hidden) {
    placeTextInput(shape);
  }
  markEditorDirty();
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

function beginEditorPan(event) {
  editorPan = beginViewPan(editorView(), event);
  editorCanvas.setPointerCapture(event.pointerId);
  editorCanvas.style.cursor = "grabbing";
}

function moveEditorPan(event) {
  setEditorView(moveViewPan(editorView(), editorPan, event));
}

function endEditorPan() {
  editorPan = null;
  editorCanvas.style.cursor = "";
}

function onEditorDown(event) {
  if (event.button === 1 || (spaceHeld && event.button === 0)) {
    beginEditorPan(event);
    return;
  }
  if (event.button !== 0) {
    return;
  }
  const point = editorPoint(event);
  if (editorTool !== TOOL_SELECT) {
    commitEditorText();
  }
  if (editorTool === TOOL_SELECT) {
    const additive = event.ctrlKey || event.metaKey;
    const picked = hitEditor(point);
    if (picked.index < 0 && !picked.handle) {
      if (!additive) {
        editorSelection = [];
        syncEditorProps();
        paintEditor();
      }
      beginEditorPan(event);
      return;
    }
    beginSelectDrag(point, additive);
    if (editorDrag) {
      editorCanvas.setPointerCapture(event.pointerId);
    }
    return;
  }
  if (editorTool === KIND_POLYGON) {
    addPolygonPoint(point);
    return;
  }
  editorDrawing = true;
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
  editorPointer = editorPoint(event);
  if (editorPan) {
    moveEditorPan(event);
    return;
  }
  const point = editorPointer;
  if (!editorDrag && !editorDrawing) {
    updateEditorHoverCursor(point);
  }
  if (editorDrag) {
    moveEditorDrag(point);
    paintEditor();
    return;
  }
  if (!editorDrawing || !editorDraft) {
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
  editorDrawing = false;
  if (editorPan) {
    endEditorPan();
    return;
  }
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
  if (draft && draft.kind === KIND_TEXT) {
    editorDraft = draft;
    placeTextInput(draft);
    paintEditor();
    return;
  }
  editorDraft = null;
  if (draft) {
    pushEditorHistory();
    editorShapes.push(draft);
    editorSelection = [editorShapes.length - 1];
    markEditorDirty();
    syncEditorProps();
  }
  paintEditor();
}

function onEditorDouble(event) {
  if (
    editorTool === KIND_POLYGON ||
    (editorDraft && editorDraft.kind === KIND_POLYGON)
  ) {
    event.preventDefault();
    closePolygonDraft();
    return;
  }
  const shape = selectedShape();
  if (!shape || shape.kind !== KIND_TEXT) {
    return;
  }
  event.preventDefault();
  editorDraft = shape;
  editorShapes.splice(selectedIndex(), 1);
  editorSelection = [];
  placeTextInput(shape);
}

function beginSelectDrag(point, additive) {
  const picked = hitEditor(point);
  if (picked.index >= 0 || picked.handle) {
    pushEditorHistory();
  }
  if (picked.handle) {
    setPrimarySelected(picked.index);
    editorDrag = {
      kind: "handle",
      handle: picked.handle,
      index: picked.index,
      last: point,
    };
    syncEditorProps();
    paintEditor();
    return;
  }
  if (picked.index >= 0) {
    if (additive) {
      editorSelection = nextEditorSelection(
        editorSelection,
        picked.index,
        true,
      );
      editorDrag = null;
    } else if (editorSelection.indexOf(picked.index) < 0) {
      editorSelection = [picked.index];
      editorDrag = { kind: "move", last: point };
    } else {
      setPrimarySelected(picked.index);
      editorDrag = { kind: "move", last: point };
    }
    syncEditorProps();
    paintEditor();
    return;
  }
  if (!additive) {
    editorSelection = [];
  }
  editorDrag = null;
  syncEditorProps();
  paintEditor();
}

function moveEditorDrag(point) {
  if (!editorDrag) {
    return;
  }
  const dx = point.x - editorDrag.last.x;
  const dy = point.y - editorDrag.last.y;
  editorDrag.last = point;
  if (editorDrag.kind === "move") {
    for (const shape of selectedShapes()) {
      shiftShape(shape, dx, dy);
    }
    markEditorDirty();
    return;
  }
  const shape = editorShapes[editorDrag.index];
  if (!shape) {
    return;
  }
  resizeShape(shape, editorDrag.handle, point);
  markEditorDirty();
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
    pushEditorHistory();
    editorDraft.closed = true;
    editorShapes.push(editorDraft);
    editorSelection = [editorShapes.length - 1];
    markEditorDirty();
  }
  editorDraft = null;
  syncEditorProps();
  paintEditor();
}

function placeTextInput(shape) {
  const box = editorCanvas.getBoundingClientRect();
  const stage = editorCanvas.parentElement.getBoundingClientRect();
  const scaleX = box.width / editorCanvas.width;
  const scaleY = box.height / editorCanvas.height;
  const bounds = normalizeBox(shape.x, shape.y, shape.w, shape.h);
  editorText.hidden = false;
  editorText.value = shape.text || "";
  editorText.style.left =
    box.left - stage.left + bounds.x * scaleX + "px";
  editorText.style.top =
    box.top - stage.top + bounds.y * scaleY + "px";
  editorText.style.width = bounds.w * scaleX + "px";
  editorText.style.height = bounds.h * scaleY + "px";
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
    pushEditorHistory();
    editorDraft.text = text;
    editorShapes.push(editorDraft);
    editorSelection = [editorShapes.length - 1];
    markEditorDirty();
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
  if (shape.kind === KIND_TEXT) {
    if (box.w < MIN_SHAPE) {
      box.w = TEXT_BOX_WIDTH;
    }
    if (box.h < MIN_SHAPE) {
      box.h = (shape.size || 22) * 1.6;
    }
    return { ...shape, ...box };
  }
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
  if (editorSelection.length === 0) {
    return;
  }
  pushEditorHistory();
  const drop = new Set(editorSelection);
  editorShapes = editorShapes.filter((_, index) => !drop.has(index));
  editorSelection = [];
  markEditorDirty();
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
  for (let i = editorSelection.length - 1; i >= 0; i -= 1) {
    const index = editorSelection[i];
    const shape = editorShapes[index];
    if (!shape) {
      continue;
    }
    for (const handle of shapeHandles(shape)) {
      if (Math.hypot(point.x - handle.x, point.y - handle.y) <= HANDLE_SIZE) {
        return { index, handle: handle.id };
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
  for (const index of editorSelection) {
    if (editorShapes[index]) {
      drawHandles(ctx, editorShapes[index]);
    }
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
    if (hasStroke(shape)) {
      ctx.strokeRect(box.x, box.y, box.w, box.h);
    }
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
    if (hasStroke(shape)) {
      ctx.stroke();
    }
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
  if (hasStroke(shape)) {
    ctx.stroke();
  }
}

function hasStroke(shape) {
  return shape.strokeOn !== false;
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
  const box = normalizeBox(shape.x, shape.y, shape.w, shape.h);
  if (hasStroke(shape)) {
    ctx.save();
    ctx.strokeStyle = shape.stroke || "#ff3b30";
    ctx.lineWidth = shape.width || 2;
    ctx.strokeRect(box.x, box.y, box.w, box.h);
    ctx.restore();
  } else if (!shape.text) {
    ctx.save();
    ctx.strokeStyle = shape.stroke || "#ff3b30";
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 3]);
    ctx.strokeRect(box.x, box.y, box.w, box.h);
    ctx.restore();
  }
  ctx.fillStyle = shape.color || "#1a1c1f";
  ctx.font =
    (shape.size || 22) + "px \"" + (shape.font || "Malgun Gothic") + "\"";
  ctx.textBaseline = "top";
  const lineHeight = (shape.size || 22) * 1.25;
  const lines = wrapTextLines(ctx, shape.text || "", box.w - 8);
  ctx.save();
  ctx.beginPath();
  ctx.rect(box.x, box.y, box.w, box.h);
  ctx.clip();
  for (let index = 0; index < lines.length; index += 1) {
    ctx.fillText(lines[index], box.x + 4, box.y + 4 + index * lineHeight);
  }
  ctx.restore();
}

function wrapTextLines(ctx, text, maxWidth) {
  const lines = [];
  const limit = Math.max(8, maxWidth);
  for (const paragraph of String(text).split("\n")) {
    let current = "";
    for (const letter of paragraph) {
      const next = current + letter;
      if (current && ctx.measureText(next).width > limit) {
        lines.push(current);
        current = letter;
      } else {
        current = next;
      }
    }
    lines.push(current);
  }
  return lines;
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
  const selected = editorSelection.slice();
  editorSelection = [];
  paintEditor();
  editorSelection = selected;
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
    editorError.textContent = "";
    editorDirty = false;
    paintEditor();
  } catch (failure) {
    editorError.textContent = String(failure);
    paintEditor();
  }
}

boot();
