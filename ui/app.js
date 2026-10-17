const core = window.__TAURI__.core;
const events = window.__TAURI__.event;
const COLUMN_WIDTH = 232;
const CARD_MIN_HEIGHT = 140;

let shell = null;
let shots = [];
let filtered = [];
let shown = 0;

const day = document.querySelector("#day");
const scroller = document.querySelector("#scroller");
const board = document.querySelector("#board");
const error = document.querySelector("#shortcut-error");

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
  const image = document.createElement("img");
  image.alt = shot.day;
  image.loading = "lazy";
  image.src = core.convertFileSrc(shot.path);
  button.append(image);
  button.addEventListener("click", () => {
    core.invoke("open_shot", { path: shot.path });
  });
  return button;
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

async function saveLanguage() {
  shell = await core.invoke("set_language", {
    tag: document.querySelector("#language").value,
  });
  applyShell();
}

boot();
