// LedgerCraft app (no framework, works offline).
"use strict";

const TOKEN = document.querySelector('meta[name="lc-token"]').content;
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
const state = { id: null, settings: null, analysis: null, heads: [], filter: "all", status: null, view: "projects", lastExport: null, adj: null, editing: null };

let pending = 0;
function saving(d) {
  pending += d;
  const el = document.getElementById("sbSave"); if (!el) return;
  el.textContent = pending > 0 ? "Working..." : "All changes saved";
  el.classList.toggle("busy", pending > 0);
}
async function api(method, path, body, raw) {
  if (method !== "GET") saving(1);
  try { return await apiInner(method, path, body, raw); } finally { if (method !== "GET") saving(-1); }
}
async function apiInner(method, path, body, raw) {
  const opt = { method, headers: { "X-LC-Token": TOKEN } };
  if (raw) { opt.body = raw; }
  else if (body !== undefined) { opt.body = JSON.stringify(body); opt.headers["Content-Type"] = "application/json"; }
  const r = await fetch(path, opt);
  const t = await r.text();
  let v; try { v = JSON.parse(t); } catch { v = { error: t }; }
  if (!r.ok) throw new Error(v.error || r.statusText);
  return v;
}

function toast(msg, err) {
  const t = $("#toast");
  t.textContent = msg; t.className = "toast show" + (err ? " err" : "");
  clearTimeout(toast.h); toast.h = setTimeout(() => (t.className = "toast"), 3200);
}

function esc(s) { return String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]); }
function setStatus(el, msg, err) { el.textContent = msg || ""; el.classList.toggle("err", !!err); }
const pid = () => encodeURIComponent(state.id);
const expert = () => $("#expert").checked;
const store = {
  get(k, d) { try { const v = localStorage.getItem("lc." + k); return v === null ? d : JSON.parse(v); } catch { return d; } },
  set(k, v) { try { localStorage.setItem("lc." + k, JSON.stringify(v)); } catch {} },
};

// Ask before anything that removes something.
function confirmBox(title, text, yes = "Delete") {
  const d = $("#confirmDlg");
  $("#cfTitle").textContent = title; $("#cfText").textContent = text; $("#cfYes").textContent = yes;
  $("#cfYes").className = /delete|remove/i.test(yes) ? "danger" : "primary";
  d.returnValue = "";
  d.showModal();
  return new Promise(res => d.addEventListener("close", () => res(d.returnValue === "yes"), { once: true }));
}

// ---- help, tour and "what to do now" -------------------------------------------
const lang = () => HELP[$("#helpLang").value] || HELP.en;
function renderGuides() {
  const on = store.get("help", true);
  $("#helpBtn").checked = on;
  for (const g of $$(".guide")) {
    const steps = lang()[g.dataset.guide] || [];
    g.hidden = !on || !steps.length;
    g.innerHTML = `<p class="gt">${esc(lang()._title)}</p><ol>${steps.map(s => `<li>${s}</li>`).join("")}</ol>`;
  }
}
$("#helpLang").value = store.get("lang", "en");
$("#helpLang").addEventListener("change", () => { store.set("lang", $("#helpLang").value); renderGuides(); renderNext(); });
$("#helpBtn").addEventListener("change", () => { store.set("help", $("#helpBtn").checked); renderGuides(); });

let tourAt = 0;
function showTour(i) {
  const t = lang().tour; tourAt = Math.max(0, Math.min(i, t.length - 1));
  $("#tourStepNo").textContent = `${tourAt + 1} / ${t.length}`;
  $("#tourTitle").textContent = t[tourAt][0];
  $("#tourText").textContent = t[tourAt][1];
  $("#tourDots").innerHTML = t.map((_, k) => `<span class="${k === tourAt ? "on" : ""}"></span>`).join("");
  $("#tourBack").disabled = tourAt === 0;
  $("#tourNext").textContent = tourAt === t.length - 1 ? "Start" : "Next";
}
function openTour() { try { $("#helpMenu").hidePopover(); } catch {} $("#tourHide").checked = store.get("tourSeen", false); showTour(0); $("#tourDlg").showModal(); }
$$("[data-tour]").forEach(b => b.addEventListener("click", openTour));
$("#tourBack").addEventListener("click", () => showTour(tourAt - 1));
$("#tourNext").addEventListener("click", () => {
  if (tourAt === lang().tour.length - 1) $("#tourDlg").close(); else showTour(tourAt + 1);
});
$("#tourHide").addEventListener("change", () => store.set("tourSeen", $("#tourHide").checked));

// Progress shown under each step, and the single next action.
function stepStatus() {
  const st = state.settings, a = state.analysis;
  const mark = (k, text, cls) => { const e = $(`[data-ss="${k}"]`); e.textContent = text; e.className = "ss " + (cls || ""); };
  if (!st) { mark("projects", "Start here"); mark("analysis", ""); for (const k of ["import", "check", "map", "present", "export"]) mark(k, ""); mark("adjust", "Optional"); return; }
  const inp = st.inputs;
  mark("projects", `${st.entity_name}, FY ${st.fy}`, "ok");
  const files = ["tb", "py_tb", "vouchers", "far"].filter(k => inp[k]).length;
  mark("import", inp.tb ? `Done (${files} of 4 files)` : "Trial balance needed", inp.tb ? "ok" : "todo");
  if (!a) { mark("check", inp.tb ? "Not run yet" : "", "todo"); mark("map", ""); }
  else {
    const s = a.summary;
    mark("check", s.must_fix ? `${s.must_fix} must fix, ${s.check} to check` : `Done, ${s.check} to check`, s.must_fix ? "bad" : "ok");
    const un = a.mapping.filter(m => !m.head).length;
    mark("map", un ? `${un} not placed` : "All placed", un ? "bad" : "ok");
  }
  const adj = st.adjustments || [], on = adj.filter(x => x.active).length;
  mark("adjust", adj.length ? `${on} applied${adj.length > on ? `, ${adj.length - on} off` : ""}` : "Optional", adj.length ? "ok" : "");
  mark("present", `In ${st.options.unit}, ${st.options.layout}`);
  mark("analysis", a ? `${a.ratios.length} ratios, ${a.loans.length} loans` : "", a ? "ok" : "");
  mark("export", a ? (a.summary.must_fix ? "Draft only" : "Ready to sign") : "", a && !a.summary.must_fix ? "ok" : "");
}
function nextAction() {
  const L = lang().next, st = state.settings, a = state.analysis;
  if (state.view === "home") return [L.intent, "home"];
  if (!st) return [L.create, "projects"];
  if (!st.inputs.tb) return [L.tb, "import"];
  if (!a) return [L.run, "check"];
  const un = a.mapping.filter(m => !m.head).length;
  if (un) return [L.map(un), "map"];
  if (a.summary.must_fix) return [L.fix(a.summary.must_fix), "check"];
  if (state.intent !== "statements") return [L.analyse, "analysis"];
  return [L.ready, "present"];
}
function renderNext() {
  stepStatus();
  const st = state.settings;
  $("#sbClient").textContent = st ? `${st.entity_name} · ${TYPE_LABEL[st.entity_type] || ""} · FY ${st.fy}` : "No client open";
  $("#sbRules").textContent = st ? (Number(st.fy.slice(0, 4)) >= 2026 ? "Income-tax Act, 2025 · Form 26" : "Income-tax Act, 1961 · Form 3CD") : "";
  const [text, view] = nextAction();
  $("#nextBox").hidden = false;
  $("#nextText").textContent = text;
  $("#nextGo").hidden = view === state.view;
  $("#nextGo").onclick = () => show(view);
}

// ---- what the user wants to do (sets the steps) --------------------------------
const FLOWS = {
  statements: { name: "Financial statements", steps: ["projects", "import", "check", "map", "adjust", "present", "export", "audit"] },
  analysis: { name: "Check and analyse", steps: ["projects", "import", "check", "analysis", "map", "adjust", "audit"] },
  taxaudit: { name: "Tax audit help", steps: ["projects", "import", "check", "analysis", "adjust", "export", "audit"] },
};
state.intent = store.get("intent", "statements");
function applyFlow() {
  const f = FLOWS[state.intent] || FLOWS.statements;
  $("#intentName").textContent = f.name;
  let n = 0;
  for (const b of $$(".steps button[data-view]")) {
    if (b.dataset.view === "home") continue;
    const i = f.steps.indexOf(b.dataset.view);
    b.hidden = i < 0;
    b.style.order = i < 0 ? 99 : i + 2;
    if (i >= 0) b.querySelector("b").textContent = String(i + 1);
    n++;
  }
  $$(".intent").forEach(x => x.classList.toggle("on", x.dataset.intent === state.intent));
}
function chooseIntent(k) {
  state.intent = k; store.set("intent", k); applyFlow();
  show(state.id ? flowNext() : "projects");
}
$$(".intent").forEach(b => b.addEventListener("click", () => chooseIntent(b.dataset.intent)));
$("#homeBtn").addEventListener("click", () => show("home"));
// The first step of the chosen path that still needs doing.
function flowNext() {
  const st = state.settings;
  if (!st) return "projects";
  if (!st.inputs.tb) return "import";
  if (state.intent === "analysis" || state.intent === "taxaudit") return "check";
  return "check";
}
function renderHome() {
  const last = store.get("last", null);
  const box = $("#continueBox");
  box.hidden = !last;
  if (last) {
    box.innerHTML = `<div class="title-row"><div><p class="k">Continue where you left off</p><p class="ct"><b>${esc(last.name)}</b>, FY ${esc(last.fy)} (${esc((FLOWS[last.intent] || FLOWS.statements).name)})</p></div><button type="button" class="primary">Continue</button></div>`;
    box.querySelector("button").onclick = async () => {
      state.intent = last.intent || "statements"; store.set("intent", state.intent); applyFlow();
      try { await openProject(last.id); } catch (e) { toast("That client year is no longer there.", true); store.set("last", null); renderHome(); }
    };
  }
}

// ---- navigation ------------------------------------------------------------
function show(view) {
  state.view = view;
  if (view === "home") renderHome();
  if (view === "analysis") { if (state.analysis) renderAnalysis(); else if (state.settings?.inputs.tb) runChecks(); }
  $$(".steps button[data-view]").forEach(b => b.classList.toggle("active", b.dataset.view === view));
  $$("section[data-panel]").forEach(s => (s.hidden = s.dataset.panel !== view));
  if (view === "present") loadPresent();
  if (view === "export") loadSign();
  if (view === "audit") loadAudit();
  if (view === "adjust") loadAdjustments();
  if ((view === "map" || view === "check") && !state.analysis && state.settings?.inputs.tb) runChecks();
  renderNext();
  window.scrollTo(0, 0);
}
$$(".steps button[data-view]").forEach(b => b.addEventListener("click", () => show(b.dataset.view)));
$$("[data-go]").forEach(b => b.addEventListener("click", () => show(b.dataset.go)));

async function reloadSettings() {
  const p = await api("GET", `/api/projects/${pid()}`); state.settings = p.settings; return p.settings;
}

// ---- status / AI pill ----------------------------------------------------
async function refreshStatus() {
  try {
    const s = await api("GET", "/api/status");
    state.status = s;
    $("#dataDir").textContent = "Data folder: " + s.data_dir;
    $("#sbData").textContent = "Data: " + s.data_dir; $("#sbVer").textContent = `LedgerCraft ${s.version}`;
    const pill = $("#aiPill");
    const have = s.ai.running && s.ai.models.some(m => m.replace(":latest", "") === s.ai.model.replace(":latest", ""));
    pill.textContent = !s.ai.running ? "Local AI: off" : have ? "Local AI: ready" : "Local AI: set up";
    pill.classList.toggle("on", have);
    $("#aiState").textContent = !s.ai.running
      ? "Ollama is not running on this computer. Install it free from ollama.com, start it, then reopen this panel. LedgerCraft works fully without it."
      : `Ollama ${s.ai.version} is running. Models on this computer: ${s.ai.models.length ? s.ai.models.join(", ") : "none yet"}.`;
    const sel = $("#aiModel");
    if (!sel.options.length) {
      const names = new Set();
      for (const m of s.ai.recommended) { names.add(m.name); sel.add(new Option(`${m.name}  (${m.about})`, m.name)); }
      for (const m of s.ai.models) if (!names.has(m) && !names.has(m.replace(":latest", ""))) sel.add(new Option(m, m));
      sel.value = s.ai.model;
    }
  } catch (e) { /* app still usable */ }
}
$("#aiPill").addEventListener("click", () => toggleAi(true));
$("#aiClose").addEventListener("click", () => toggleAi(false));
function toggleAi(open) { $("#aiPanel").hidden = !open; $(".shell").classList.toggle("with-ai", open); if (open) refreshStatus(); }

$("#aiSetup").addEventListener("click", async () => {
  try {
    await api("POST", "/api/ai/setup", { model: $("#aiModel").value });
    $("#aiBar").hidden = false;
    const tick = async () => {
      const p = await api("GET", "/api/ai/setup");
      const pct = p.total ? Math.round((p.completed / p.total) * 100) : 0;
      $("#aiBar span").style.width = pct + "%";
      setStatus($("#aiSetupStatus"), p.error ? p.error : p.done ? "Model ready." : `${p.status} ${pct ? pct + "%" : ""}`, !!p.error);
      if (p.running) setTimeout(tick, 800); else refreshStatus();
    };
    tick();
  } catch (e) { setStatus($("#aiSetupStatus"), e.message, true); }
});
$("#aiAsk").addEventListener("click", async () => {
  if (!state.id) return toast("Open a client first.", true);
  const out = $("#aiAnswer"); out.textContent = "Thinking on this computer...";
  try {
    const r = await api("POST", `/api/projects/${pid()}/ai/ask`, { question: $("#aiQ").value, language: $("#lang").value });
    out.innerHTML = `<span class="ai-label">${esc(r.label)} (${esc(r.model)})</span>${esc(r.text)}`;
  } catch (e) { out.textContent = e.message; }
});

$("#quitBtn").addEventListener("click", async () => {
  try { await api("POST", "/api/quit"); } catch {}
  document.body.innerHTML = '<p style="padding:40px;font-family:sans-serif">LedgerCraft has closed. You can close this tab.</p>';
});

async function openFolder(path) {
  try { await api("POST", "/api/open-folder", { path }); } catch (e) { toast(e.message, true); }
}

// ---- 1. projects ----------------------------------------------------------
function currentFy() {
  const d = new Date(); const y = d.getMonth() >= 3 ? d.getFullYear() : d.getFullYear() - 1;
  return `${y}-${String((y + 1) % 100).padStart(2, "0")}`;
}
$("#npFy").value = currentFy();
const TYPE_LABEL = { firm: "Partnership firm", llp: "LLP", company: "Company", proprietor: "Proprietorship", huf: "HUF", aop: "AOP", boi: "BOI" };
const when = s => esc((s || "").slice(0, 16).replace("T", " "));

async function loadProjects() {
  state.projects = await api("GET", "/api/projects");
  renderProjects();
}
function renderProjects() {
  const all = state.projects || [];
  const q = $("#plSearch").value.trim().toLowerCase(), ty = $("#plType").value;
  const list = all.filter(p => (!ty || p.entity_type === ty) && (!q || `${p.entity_name} ${p.fy}`.toLowerCase().includes(q)))
    .sort((a, b) => a.entity_name.localeCompare(b.entity_name) || b.fy.localeCompare(a.fy));
  $("#plCount").textContent = all.length ? `${new Set(all.map(p => p.entity_name)).size} clients, ${all.length} years` : "";
  const tb = $("#projectList tbody");
  tb.innerHTML = all.length ? (list.length ? "" : `<tr><td colspan="5" class="empty">No client matches.</td></tr>`) : `<tr><td colspan="5" class="empty">No clients yet. Type the name of the business above and press <b>Create</b>.</td></tr>`;
  let prev = null;
  for (const p of list) {
    const tr = document.createElement("tr");
    const cur = p.id === state.id;
    if (cur) tr.className = "current";
    const same = prev === p.entity_name; prev = p.entity_name;
    if (same) tr.classList.add("same");
    tr.innerHTML = `<td>${same ? '<span class="muted small">same client</span>' : `<b>${esc(p.entity_name)}</b>`}</td><td>${esc(TYPE_LABEL[p.entity_type] || p.entity_type)}</td><td><span class="chip">FY ${esc(p.fy)}</span></td><td class="muted">${when(p.modified)}</td>
      <td class="act"><button class="small ${cur ? "" : "primary"}" data-a="open">${cur ? "Open now" : "Open"}</button><button class="small danger-ghost" data-a="del">Delete</button></td>`;
    tr.querySelector('[data-a="open"]').addEventListener("click", () => openProject(p.id));
    tr.querySelector('[data-a="del"]').addEventListener("click", async () => {
      if (!(await confirmBox(`Delete ${p.entity_name}, FY ${p.fy}?`, "It moves to the Recycle Bin with its imports, settings and audit trail. You can restore it later. The remembered mapping for this client stays."))) return;
      try {
        await api("POST", `/api/projects/${encodeURIComponent(p.id)}/delete`);
        if (state.id === p.id) closeProject();
        toast("Moved to the Recycle Bin."); loadProjects(); if (!$("#binWrap").hidden) loadBin();
      } catch (e) { toast(e.message, true); }
    });
    tb.appendChild(tr);
  }
}
$("#plSearch").addEventListener("input", renderProjects);
$("#plType").addEventListener("change", renderProjects);

// Client search in the top bar (Ctrl+K).
function searchClients() {
  const q = $("#clientSearch").value.trim().toLowerCase();
  const ul = $("#searchResults");
  if (!q) { ul.hidden = true; $("#clientSearch").setAttribute("aria-expanded", "false"); return; }
  const hits = (state.projects || []).filter(p => `${p.entity_name} ${p.fy} ${TYPE_LABEL[p.entity_type] || ""}`.toLowerCase().includes(q)).slice(0, 12);
  ul.innerHTML = hits.length ? hits.map((p, i) => `<li role="option" data-id="${esc(p.id)}" class="${i === 0 ? "hi" : ""}"><b>${esc(p.entity_name)}</b><span>FY ${esc(p.fy)} · ${esc(TYPE_LABEL[p.entity_type] || "")}</span></li>`).join("")
    : `<li class="none">No client found. <button type="button" class="small" id="srNew">Create "${esc($("#clientSearch").value.trim())}"</button></li>`;
  ul.hidden = false; $("#clientSearch").setAttribute("aria-expanded", "true");
  $$("li[data-id]", ul).forEach(li => li.addEventListener("mousedown", e => { e.preventDefault(); pickClient(li.dataset.id); }));
  $("#srNew")?.addEventListener("mousedown", e => { e.preventDefault(); const n = $("#clientSearch").value.trim(); closeSearch(); show("projects"); $("#npName").value = n; $("#npName").focus(); });
}
function closeSearch() { $("#searchResults").hidden = true; $("#clientSearch").value = ""; $("#clientSearch").setAttribute("aria-expanded", "false"); }
function pickClient(id) { closeSearch(); openProject(id).catch(e => toast(e.message, true)); }
$("#clientSearch").addEventListener("input", searchClients);
$("#clientSearch").addEventListener("focus", () => { if (!state.projects) loadProjects(); });
$("#clientSearch").addEventListener("blur", () => setTimeout(() => ($("#searchResults").hidden = true), 120));
$("#clientSearch").addEventListener("keydown", e => {
  const items = $$("#searchResults li[data-id]"); let i = items.findIndex(x => x.classList.contains("hi"));
  if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); if (!items.length) return; items[i]?.classList.remove("hi"); i = (i + (e.key === "ArrowDown" ? 1 : -1) + items.length) % items.length; items[i].classList.add("hi"); }
  else if (e.key === "Enter") { e.preventDefault(); if (items[i]) pickClient(items[i].dataset.id); }
  else if (e.key === "Escape") { closeSearch(); e.target.blur(); }
});
// Keyboard: Ctrl+K search, Alt+1..9 steps.
document.addEventListener("keydown", e => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") { e.preventDefault(); $("#clientSearch").focus(); }
  if (e.altKey && /^[0-9]$/.test(e.key)) {
    const vis = $$(".steps button[data-view]").filter(b => !b.hidden && !b.disabled).sort((a, b) => (+a.style.order || 0) - (+b.style.order || 0));
    const b = e.key === "0" ? vis.find(x => x.dataset.view === "home") : vis.filter(x => x.dataset.view !== "home")[+e.key - 1];
    if (b) { e.preventDefault(); show(b.dataset.view); }
  }
});

async function loadBin() {
  const list = await api("GET", "/api/recycle-bin");
  const tb = $("#binList tbody");
  tb.innerHTML = list.length ? "" : `<tr><td colspan="5" class="empty">The Recycle Bin is empty.</td></tr>`;
  for (const p of list) {
    const stamp = p.id.split("~")[2] || "";
    const tr = document.createElement("tr");
    tr.innerHTML = `<td>${esc(p.entity_name)}</td><td>${esc(TYPE_LABEL[p.entity_type] || p.entity_type)}</td><td>${esc(p.fy)}</td><td class="muted">${esc(stamp.replace(/^(\d{4})(\d\d)(\d\d)-(\d\d)(\d\d).*/, "$3-$2-$1 $4:$5"))}</td><td class="act"><button class="small">Restore</button></td>`;
    tr.querySelector("button").addEventListener("click", async () => {
      try { await api("POST", `/api/recycle-bin/${encodeURIComponent(p.id)}/restore`); toast("Restored."); loadBin(); loadProjects(); }
      catch (e) { toast(e.message, true); }
    });
    tb.appendChild(tr);
  }
}
$("#binBtn").addEventListener("click", () => {
  const open = $("#binWrap").hidden;
  $("#binWrap").hidden = !open; $("#binBtn").setAttribute("aria-pressed", String(open));
  $("#binBtn").textContent = open ? "Hide Recycle Bin" : "Recycle Bin";
  if (open) loadBin();
});
$("#openDataDir").addEventListener("click", () => state.status && openFolder(state.status.data_dir));
$("#newProject").addEventListener("submit", async e => {
  e.preventDefault();
  setStatus($("#npError"), "");
  try {
    const r = await api("POST", "/api/projects", { name: $("#npName").value, entity_type: $("#npType").value, fy: $("#npFy").value });
    $("#npName").value = "";
    await openProject(r.id);
  } catch (err) { setStatus($("#npError"), err.message, true); }
});

function closeProject() {
  state.id = null; state.settings = null; state.analysis = null;
  $("#projectName").textContent = "No client open";
  $$(".steps button[data-view]").forEach(b => (b.disabled = !["projects", "home"].includes(b.dataset.view)));
  renderNext();
}
async function openProject(id) {
  state.id = id; state.analysis = null; state.lastExport = null; state.adj = null;
  const st = await reloadSettings();
  $("#projectName").textContent = `${st.entity_name}  |  ${TYPE_LABEL[st.entity_type] || ""}  |  FY ${st.fy}`;
  $$(".steps button").forEach(b => (b.disabled = false));
  applyFlow();
  $("#openExport").disabled = true; $("#eResult").hidden = true;
  renderFiles();
  $("#depBasis").value = st.depreciation_basis;
  loadProjects();
  store.set("last", { id, name: st.entity_name, fy: st.fy, intent: state.intent });
  show(flowNext());
}

// ---- 2. import ------------------------------------------------------------
const FILES = [
  ["tb", "Trial balance (this year)", "Required. Ledger, Group, Opening, Closing (or Debit and Credit)."],
  ["py_tb", "Trial balance (last year)", "For last year's column, opening-balance checks and cash flow."],
  ["vouchers", "Day book (vouchers)", "For cash limits, loans, ageing and partners' capital."],
  ["far", "Fixed asset register", "Sheet 'Fixed Assets'; optional sheet 'IT Opening'."],
  ["accounts_master", "Account master (BUSY)", "Only if the trial balance has no Group column."],
];
function renderFiles() {
  const box = $("#fileRows"); box.innerHTML = "";
  const inp = state.settings.inputs;
  for (const [kind, title, help] of FILES) {
    const have = inp[kind];
    const fromTally = have && String(have).endsWith(".json");
    const row = document.createElement("div"); row.className = "file-row";
    row.innerHTML = `<div><div class="name">${title}</div><div class="state ${have ? "ok" : ""}">${have ? (fromTally ? "Imported from Tally: " + esc(inp.tally_company || "") : "Loaded") : esc(help)}</div></div>
      <div class="act"><label class="btnlike small"><input type="file" accept=".xlsx,.xls,.csv,.xlsm,.ods" hidden><span tabindex="0" role="button">${have ? "Replace" : "Choose file"}</span></label>${have ? '<button type="button" class="small danger-ghost">Remove</button>' : ""}</div>`;
    const input = row.querySelector("input"), span = row.querySelector("[role=button]");
    span.addEventListener("keydown", e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); input.click(); } });
    input.addEventListener("change", async () => {
      const f = input.files[0]; if (!f) return;
      setStatus($("#fStatus"), `Reading ${f.name}...`);
      try {
        const r = await api("POST", `/api/projects/${pid()}/upload?kind=${kind}&name=${encodeURIComponent(f.name)}`, undefined, await f.arrayBuffer());
        setStatus($("#fStatus"), `${f.name}: ${r.contents} loaded.`);
        await reloadSettings(); state.analysis = null; renderFiles(); renderNext();
      } catch (e) { setStatus($("#fStatus"), e.message, true); }
    });
    row.querySelector(".danger-ghost")?.addEventListener("click", async () => {
      if (!(await confirmBox(`Remove ${title.toLowerCase()}?`, "LedgerCraft stops using this file. A copy is kept in the client's folder for the audit trail.", "Remove"))) return;
      try { await api("POST", `/api/projects/${pid()}/remove-input`, { kind }); await reloadSettings(); state.analysis = null; renderFiles(); renderNext(); toast("Removed."); }
      catch (e) { toast(e.message, true); }
    });
    box.appendChild(row);
  }
}
$("#depBasis").addEventListener("change", async () => {
  await api("POST", `/api/projects/${pid()}/settings`, { depreciation_basis: $("#depBasis").value });
  state.analysis = null; toast("Depreciation basis saved.");
});
$("#tFind").addEventListener("click", async () => {
  setStatus($("#tStatus"), "Connecting to Tally...");
  try {
    const c = await api("GET", `/api/tally/companies?host=${encodeURIComponent($("#tHost").value)}&port=${encodeURIComponent($("#tPort").value)}`);
    const sel = $("#tCompany"); sel.innerHTML = "";
    c.forEach(n => sel.add(new Option(n, n)));
    setStatus($("#tStatus"), c.length ? `${c.length} compan${c.length === 1 ? "y" : "ies"} found. Choose one and press Import.` : "Tally answered but no company is open.");
  } catch (e) { setStatus($("#tStatus"), e.message, true); }
});
$("#tImport").addEventListener("click", async () => {
  const company = $("#tCompany").value; if (!company) return setStatus($("#tStatus"), "Press Find companies and choose a company first.", true);
  setStatus($("#tStatus"), "Importing from Tally. Large books take a minute...");
  $("#tImport").disabled = true;
  try {
    const r = await api("POST", `/api/projects/${pid()}/tally`, { host: $("#tHost").value, port: Number($("#tPort").value), company, vouchers: $("#tVouchers").checked });
    setStatus($("#tStatus"), `Imported ${r.ledgers} ledgers, ${r.vouchers} vouchers${r.previous_year ? ", and last year's balances" : ""}.`);
    await reloadSettings(); state.analysis = null; renderFiles(); renderNext();
  } catch (e) { setStatus($("#tStatus"), e.message, true); }
  $("#tImport").disabled = false;
});

// ---- 3. check -------------------------------------------------------------
const SEV = { blocker: "Must fix", warning: "Check", info: "Note" };
async function runChecks() {
  const b = $("#runChecks"); b.disabled = true; b.textContent = "Checking...";
  try {
    state.analysis = await api("POST", `/api/projects/${pid()}/analyse`);
    renderCheck(); renderMap(); if (state.view === "analysis") renderAnalysis();
  } catch (e) { toast(e.message, true); $("#findings").innerHTML = `<li class="empty">${esc(e.message)}</li>`; }
  b.disabled = false; b.textContent = "Run checks again";
  renderNext();
}
$("#runChecks").addEventListener("click", runChecks);
$("#codeFilter button").addEventListener("click", () => { state.codes = null; renderCheck(); });
$$(".filters [data-f]").forEach(b => b.addEventListener("click", () => { state.codes = null; state.filter = b.dataset.f; $$(".filters [data-f]").forEach(x => x.classList.toggle("on", x === b)); renderCheck(); }));
$("#expert").checked = store.get("expert", false);
$("#expert").addEventListener("change", () => { store.set("expert", $("#expert").checked); if (state.analysis) renderCheck(); });

function renderCheck() {
  const a = state.analysis; if (!a) return;
  const s = a.summary;
  $("#tiles").innerHTML = `
    <div class="tile ${s.must_fix ? "blocker" : ""}"><div class="k">Must fix</div><div class="v">${s.must_fix}</div></div>
    <div class="tile ${s.check ? "warning" : ""}"><div class="k">Check</div><div class="v">${s.check}</div></div>
    <div class="tile"><div class="k">Notes</div><div class="v">${s.notes}</div></div>
    <div class="tile"><div class="k">Profit / (loss)</div><div class="v money">${esc(s.profit)}</div></div>
    <div class="tile"><div class="k">Balance sheet total</div><div class="v money">${esc(s.total_assets)}</div></div>
    <div class="tile"><div class="k">Data</div><div class="v sm">${s.ledgers} ledgers, ${s.vouchers} vouchers${s.has_previous_year ? ", last year" : ""}${s.has_far ? ", asset register" : ""}</div></div>`;
  const ul = $("#findings"); ul.innerHTML = "";
  const codes = state.codes;
  const list = a.findings.filter(f => (codes ? codes.includes(f.code) : state.filter === "all" || f.severity === state.filter));
  $("#codeFilter").hidden = !codes;
  if (codes) $("#codeFilter span").textContent = state.codesLabel;
  if (!list.length) ul.innerHTML = `<li class="empty">${a.findings.length ? "Nothing in this filter." : "No issues found. The books pass every check."}</li>`;
  for (const f of list) {
    const li = document.createElement("li"); li.className = "finding";
    li.innerHTML = `<span class="badge ${f.severity}">${SEV[f.severity]}</span><span class="t">${esc(f.title)}</span>
      <span class="fa">${f.ledger ? '<button class="small" type="button" data-a="map">Map this ledger</button>' : ""}<button class="small" type="button" data-a="ai">Explain</button></span>
      <div class="m">${esc(f.message)}</div>
      ${f.suggestion ? `<div class="s"><b>What to do:</b> ${esc(f.suggestion)}</div>` : ""}
      ${expert() && f.legal_ref ? `<div class="ref">Reference: ${esc(f.legal_ref)}</div>` : ""}`;
    li.querySelector('[data-a="map"]')?.addEventListener("click", () => {
      $("#mapSearch").value = f.ledger; $("#mapAttention").checked = false; show("map"); renderMap();
    });
    li.querySelector('[data-a="ai"]').addEventListener("click", async ev => {
      const btn = ev.currentTarget; btn.disabled = true; btn.textContent = "Explaining...";
      let box = li.querySelector(".ai-text"); if (!box) { box = document.createElement("div"); box.className = "ai-text"; li.appendChild(box); }
      try {
        const r = await api("POST", `/api/projects/${pid()}/ai/explain`, { key: f.key, language: $("#lang").value });
        box.innerHTML = `<span class="ai-label">${esc(r.label)} (${esc(r.model)})</span>${esc(r.text)}`;
      } catch (e) { box.textContent = e.message; }
      btn.disabled = false; btn.textContent = "Explain";
    });
    ul.appendChild(li);
  }
  const w = a.warnings || [];
  $("#warnBox").hidden = !w.length;
  $("#warnings").innerHTML = w.map(x => `<li>${esc(x)}</li>`).join("");
}

// ---- 4. map ---------------------------------------------------------------
const TAGS = ["msme", "transporter", "disputed", "doubtful", "exempt"];
const TAG_TIP = { msme: "Supplier is a micro or small enterprise", transporter: "Transporter (cash limit ₹35,000)", disputed: "Amount is disputed", doubtful: "Recovery doubtful", exempt: "Loan from bank / Government (cash-loan limits do not apply)" };
async function loadHeads() { if (!state.heads.length) state.heads = await api("GET", "/api/heads"); }
function reasons(m) {
  const t = (state.analysis?.findings || []).filter(f => f.ledger === m.name && f.severity !== "info").map(f => f.title);
  if (!m.standard_group) t.unshift("Group not recognised");
  if (m.reclassified) t.push("Shown on the other side because of its balance");
  return [...new Set(t)];
}
function needsAttention(m) {
  const keys = new Set((state.analysis?.findings || []).filter(f => f.ledger === m.name).map(f => f.code));
  return !m.head || !m.standard_group || m.reclassified || [...keys].some(k => k.startsWith("MISGROUP") || k === "UNKNOWN_GROUP" || k === "CAPITAL_IN_EXPENSE" || k === "ABNORMAL_BALANCE");
}
async function setHead(m, head, ai) {
  try {
    await api("POST", `/api/projects/${pid()}/mapping`, { ledger: m.name, head, ai: !!ai });
    toast(head ? "Saved. Remembered for this client." : "Back to LedgerCraft's own choice.");
    await runChecks();
  } catch (err) { toast(err.message, true); }
}
async function renderMap() {
  await loadHeads();
  const a = state.analysis; if (!a) return;
  const q = $("#mapSearch").value.toLowerCase(), only = $("#mapAttention").checked;
  const tb = $("#mapTable tbody"); tb.innerHTML = "";
  const rows = a.mapping.filter(m => (!only || needsAttention(m)) && (!q || m.name.toLowerCase().includes(q) || m.group.toLowerCase().includes(q)));
  if (!rows.length) tb.innerHTML = `<tr><td colspan="6" class="empty">${only ? "Nothing needs attention. Untick the box to see every ledger." : "No ledger matches."}</td></tr>`;
  const LIMIT = state.mapLimit || 300;
  for (const m of rows.slice(0, LIMIT)) {
    const tr = document.createElement("tr");
    // Only the chosen line is rendered; the full list is filled when the box is opened (fast with thousands of ledgers).
    const opts = m.head ? `<option value="${m.head}" selected>${esc(m.head_label || m.head)}</option>` : "";
    const why = reasons(m);
    const mine = m.source === "memory";
    tr.innerHTML = `<td>${needsAttention(m) ? '<span class="needs" aria-hidden="true"></span>' : ""}${esc(m.name)}${why.length ? `<div class="src">${esc(why.join("; "))}</div>` : ""}</td>
      <td>${esc(m.group)}${m.standard_group ? "" : ' <span class="badge blocker">group not recognised</span>'}</td>
      <td class="num">${esc(m.amount)}</td>
      <td><select class="head" aria-label="Shown under for ${esc(m.name)}">${m.head ? "" : '<option value="" selected>Not placed: choose a line</option>'}${opts}</select>
        <div class="src">${mine ? "Your choice (remembered)" : m.source === "name_rule" ? "By ledger name" : m.source ? "By group" : ""}${m.reclassified ? ", moved by balance side" : ""}</div></td>
      <td><div class="tags">${TAGS.map(t => `<button type="button" class="tag ${m.tags.includes(t) ? "on" : ""}" data-t="${t}" title="${TAG_TIP[t]}">${t}</button>`).join("")}</div></td>
      <td class="act"><button class="small" type="button" data-a="ai">Suggest</button>${mine ? `<button class="small" type="button" data-a="reset" title="Go back to LedgerCraft's own choice">Reset</button>` : ""}</td>`;
    const sel = tr.querySelector("select");
    const fill = () => {
      if (sel.dataset.full) return; sel.dataset.full = "1";
      sel.innerHTML = (m.head ? "" : '<option value="" selected>Not placed: choose a line</option>') + state.heads.map(h => `<option value="${h.id}" ${h.id === m.head ? "selected" : ""}>${esc(h.label)}</option>`).join("");
    };
    sel.addEventListener("mousedown", fill); sel.addEventListener("focus", fill); sel.addEventListener("keydown", fill);
    sel.addEventListener("change", e => e.target.value && e.target.value !== m.head && setHead(m, e.target.value));
    tr.querySelector('[data-a="reset"]')?.addEventListener("click", () => setHead(m, ""));
    tr.querySelectorAll(".tag").forEach(b => b.addEventListener("click", async () => {
      b.classList.toggle("on");
      const tags = [...tr.querySelectorAll(".tag.on")].map(x => x.dataset.t);
      try { await api("POST", `/api/projects/${pid()}/tags`, { ledger: m.name, tags }); m.tags = tags; toast("Tag saved."); state.analysis = null; }
      catch (err) { toast(err.message, true); }
    }));
    tr.querySelector('[data-a="ai"]').addEventListener("click", async e => {
      const btn = e.currentTarget; btn.disabled = true; btn.textContent = "Thinking...";
      try {
        const r = await api("POST", `/api/projects/${pid()}/ai/map`, { ledger: m.name });
        if (await confirmBox(`${r.label} (${r.model})`, `${r.head_label}\n\n${r.reason}`, "Use this")) await setHead(m, r.head, true);
      } catch (err) { toast(err.message, true); }
      btn.disabled = false; btn.textContent = "Suggest";
    });
    tb.appendChild(tr);
  }
  if (rows.length > LIMIT) {
    const tr = document.createElement("tr");
    tr.innerHTML = `<td colspan="6" class="empty">Showing ${LIMIT} of ${rows.length}. <button type="button" class="small">Show all</button> or search above.</td>`;
    tr.querySelector("button").onclick = () => { state.mapLimit = 1e9; renderMap(); };
    tb.appendChild(tr);
  }
}
let mapTimer;
$("#mapSearch").addEventListener("input", () => { clearTimeout(mapTimer); mapTimer = setTimeout(() => { state.mapLimit = 0; renderMap(); }, 150); });
$("#mapAttention").addEventListener("change", () => { state.mapLimit = 0; renderMap(); });

// ---- 5. adjustments --------------------------------------------------------
// Amounts typed in rupees ("1,23,456.50") become exact paise.
function paise(s) {
  s = String(s || "").replace(/[,\s₹]/g, "");
  if (!s) return 0;
  if (!/^\d+(\.\d{0,2})?$/.test(s)) return NaN;
  const [i, f = ""] = s.split(".");
  return Number(i) * 100 + Number((f + "00").slice(0, 2));
}
const rupees = p => (p / 100).toLocaleString("en-IN", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

async function loadAdjustments() {
  try { state.adj = await api("GET", `/api/projects/${pid()}/adjustments`); }
  catch (e) { state.adj = null; $("#adjTable tbody").innerHTML = `<tr><td colspan="6" class="empty">${esc(e.message)}</td></tr>`; return; }
  const d = state.adj;
  $("#ledgerList").innerHTML = d.ledgers.map(l => `<option value="${esc(l.name)}">${esc(l.group)}${l.stock ? " (closing stock: enter alone)" : ""}, ${esc(l.balance)}</option>`).join("");
  $("#adjNote").textContent = d.has_day_book ? "Each adjustment is also added to the day book as a journal dated the last day of the year, so every check sees it." : "";
  const tb = $("#adjTable tbody"); tb.innerHTML = "";
  if (!d.list.length) tb.innerHTML = `<tr><td colspan="6" class="empty">No adjustments. Press <b>New adjustment</b> (top right) to pass one.</td></tr>`;
  for (const a of d.list) {
    const tr = document.createElement("tr");
    if (!a.active) tr.className = "off";
    const lines = a.lines.filter(l => l.amount).map(l => `<div class="jl"><span class="drcr">${l.amount > 0 ? "Dr" : "Cr"}</span><span>${esc(l.ledger)}${l.new_group ? ` <span class="badge info">new: ${esc(l.new_group)}</span>` : ""}</span><span class="num">${rupees(Math.abs(l.amount))}</span></div>`).join("");
    tr.innerHTML = `<td class="num">${a.id}</td><td>${esc(a.kind_label)}</td><td>${esc(a.narration)}</td><td>${lines}</td><td>${a.active ? '<span class="badge okb">Applied</span>' : '<span class="badge">Switched off</span>'}</td>
      <td class="act"><button class="small" data-a="edit">Edit</button><button class="small" data-a="toggle">${a.active ? "Switch off" : "Switch on"}</button><button class="small danger-ghost" data-a="del">Delete</button></td>`;
    tr.querySelector('[data-a="edit"]').addEventListener("click", () => editAdjustment(a));
    tr.querySelector('[data-a="toggle"]').addEventListener("click", async () => {
      try { await api("POST", `/api/projects/${pid()}/adjustments/${a.id}/active`, { active: !a.active }); afterAdjChange(a.active ? "Switched off." : "Applied."); }
      catch (e) { toast(e.message, true); }
    });
    tr.querySelector('[data-a="del"]').addEventListener("click", async () => {
      if (!(await confirmBox(`Delete adjustment ${a.id}?`, `"${a.narration}" will no longer be applied. The audit trail keeps a record of it.`))) return;
      try { await api("POST", `/api/projects/${pid()}/adjustments/${a.id}/delete`); afterAdjChange("Deleted."); }
      catch (e) { toast(e.message, true); }
    });
    tb.appendChild(tr);
  }
}
async function afterAdjChange(msg) {
  toast(msg); state.analysis = null; await reloadSettings(); await loadAdjustments(); renderNext();
}
function adjLine(l = {}) {
  const tr = document.createElement("tr");
  const amt = l.amount || 0;
  const groups = (state.adj?.groups || []).map(g => `<option ${g === l.new_group ? "selected" : ""}>${esc(g)}</option>`).join("");
  tr.innerHTML = `<td><input class="lg" list="ledgerList" value="${esc(l.ledger || "")}" aria-label="Ledger" placeholder="Type or pick a ledger"></td>
    <td class="num"><input class="dr num" inputmode="decimal" value="${amt > 0 ? (amt / 100).toFixed(2) : ""}" aria-label="Debit"></td>
    <td class="num"><input class="cr num" inputmode="decimal" value="${amt < 0 ? (-amt / 100).toFixed(2) : ""}" aria-label="Credit"></td>
    <td><select class="ng" aria-label="Group for new ledger"><option value="">(ledger is in the books)</option>${groups}</select></td>
    <td class="act"><button type="button" class="small" aria-label="Remove line">Remove</button></td>`;
  tr.querySelector("button").addEventListener("click", () => { tr.remove(); adjTotals(); });
  tr.querySelectorAll("input,select").forEach(i => i.addEventListener("input", adjTotals));
  $("#adjLines tbody").appendChild(tr);
  adjTotals();
}
function adjRead() {
  // Ledgers this entry itself creates are "new" while it is being edited.
  const known = new Set((state.adj?.ledgers || []).filter(l => !l.created_by || l.created_by !== state.editing).map(l => l.name.toLowerCase()));
  return $$("#adjLines tbody tr").map(tr => {
    const ledger = $(".lg", tr).value.trim(), dr = paise($(".dr", tr).value), cr = paise($(".cr", tr).value);
    const isNew = !!ledger && !known.has(ledger.toLowerCase());
    const ng = $(".ng", tr);
    ng.disabled = !isNew; if (!isNew) ng.value = "";
    ng.classList.toggle("need", isNew && !ng.value);
    return { ledger, dr, cr, amount: (dr || 0) - (cr || 0), new_group: isNew ? ng.value || null : null, bad: isNaN(dr) || isNaN(cr) || (dr > 0 && cr > 0) };
  });
}
function adjTotals() {
  const ls = adjRead();
  const stock = new Set((state.adj?.ledgers || []).filter(l => l.stock).map(l => l.name.toLowerCase()));
  const dr = ls.reduce((s, l) => s + (l.dr || 0), 0), cr = ls.reduce((s, l) => s + (l.cr || 0), 0);
  const diff = ls.filter(l => !stock.has(l.ledger.toLowerCase())).reduce((s, l) => s + (l.amount || 0), 0);
  $("#adjDr").textContent = rupees(dr); $("#adjCr").textContent = rupees(cr);
  const bad = ls.some(l => l.bad);
  $("#adjDiff").innerHTML = bad ? '<span class="errt">Use numbers only, and either debit or credit on a line.</span>'
    : diff ? `<span class="errt">Difference ${rupees(Math.abs(diff))} ${diff > 0 ? "Dr" : "Cr"}: debit and credit must agree.</span>`
    : '<span class="okt">Debit equals credit.</span>';
}
function editAdjustment(a) {
  state.editing = a ? a.id : 0;
  $("#adjTitle").textContent = a ? `Edit adjustment ${a.id}` : "New adjustment";
  $("#adjKind").value = a ? a.kind : "provision";
  $("#adjNarr").value = a ? a.narration : "";
  $("#adjLines tbody").innerHTML = "";
  (a ? a.lines : [{}, {}]).forEach(adjLine);
  setStatus($("#adjStatus"), "");
  $("#adjForm").hidden = false; $("#adjNarr").focus();
}
$("#adjNew").addEventListener("click", async () => { if (!state.adj) await loadAdjustments(); editAdjustment(null); });
$("#adjAddLine").addEventListener("click", () => adjLine());
$("#adjCancel").addEventListener("click", () => { $("#adjForm").hidden = true; });
$("#adjForm").addEventListener("submit", async e => {
  e.preventDefault();
  const ls = adjRead();
  if (ls.some(l => l.bad)) return setStatus($("#adjStatus"), "Use numbers only, and either debit or credit on a line.", true);
  const body = { id: state.editing || 0, kind: $("#adjKind").value, narration: $("#adjNarr").value.trim(), active: true,
    lines: ls.filter(l => l.ledger || l.amount).map(l => ({ ledger: l.ledger, amount: l.amount, new_group: l.new_group })) };
  try {
    await api("POST", `/api/projects/${pid()}/adjustments`, body);
    $("#adjForm").hidden = true; afterAdjChange("Adjustment saved and applied.");
  } catch (err) { setStatus($("#adjStatus"), err.message, true); }
});

// ---- analysis ------------------------------------------------------------------
// Section numbers: Income-tax Act, 1961 up to FY 2025-26; Income-tax Act, 2025 from Tax Year 2026-27.
const TAX_ITEMS = [
  ["CASH_PAYMENT_LIMIT", "Cash payments above the limit", "s.40A(3)", "s.36"],
  ["CASH_ASSET_PURCHASE", "Assets bought in cash", "s.43(1)", "actual-cost rule"],
  ["CASH_RECEIPT_LIMIT", "Cash receipts of ₹2 lakh or more", "s.269ST", "s.186"],
  ["LOAN_ACCEPTED_CASH", "Loans taken in cash", "s.269SS", "s.185"],
  ["LOAN_ACCEPTED_JOURNAL", "Loans taken by journal entry", "s.269SS", "s.185"],
  ["LOAN_REPAID_CASH", "Loans repaid in cash", "s.269T", "s.188"],
  ["NEGATIVE_CASH", "Cash balance below zero", "", ""],
];
const newAct = () => Number(String(state.settings?.fy || "").slice(0, 4)) >= 2026;
const pct = (c, p) => (p ? (((c - p) / Math.abs(p)) * 100).toFixed(1) + "%" : "");
function renderAnalysis() {
  const a = state.analysis, box = $("#anBody");
  if (!a) { box.innerHTML = '<p class="empty">Run the checks first.</p>'; return; }
  const cy = a.key.cy, py = a.key.py;
  const KEY = [["revenue", "Revenue from operations"], ["other_income", "Other income"], ["profit", "Profit / (loss) for the year"], ["employee", "Employee costs"], ["finance_costs", "Interest and finance costs"], ["depreciation", "Depreciation"],
    ["total_assets", "Total assets"], ["ppe", "Fixed assets (net)"], ["inventories", "Stock"], ["receivables", "Debtors"], ["cash_bank", "Cash and bank"], ["payables", "Creditors"], ["borrowings", "Borrowings"]];
  const money = p => (p < 0 ? `(${rupees(-p)})` : rupees(p));
  let h = `<div class="two"><div class="box"><h2>Key figures (₹)</h2><table class="grid dense"><thead><tr><th>Item</th><th class="num">This year</th>${py ? '<th class="num">Last year</th><th class="num">Change</th>' : ""}</tr></thead><tbody>` +
    KEY.map(([k, l]) => `<tr><td>${l}</td><td class="num">${money(cy[k])}</td>${py ? `<td class="num">${money(py[k])}</td><td class="num">${pct(cy[k], py[k])}</td>` : ""}</tr>`).join("") + "</tbody></table></div>";
  // Tax-audit sensitive items, from the findings.
  h += `<div class="box"><h2>Items for tax audit</h2><table class="grid dense"><thead><tr><th>Item</th><th>Section</th><th class="num">Cases</th><th class="num">Amount (₹)</th><th class="act"></th></tr></thead><tbody>`;
  for (const [code, label, oldSec, newSec] of TAX_ITEMS) {
    const sec = newAct() ? newSec : oldSec;
    const fs = a.findings.filter(f => f.code === code);
    const total = fs.reduce((s, f) => s + Math.abs(f.amount || 0), 0);
    h += `<tr class="${fs.length ? "" : "nil"}"><td>${label}</td><td class="muted">${sec}</td><td class="num">${fs.length}</td><td class="num">${fs.length ? rupees(total) : "-"}</td><td class="act">${fs.length ? `<button type="button" class="small" data-code="${code}" data-label="${esc(label)}">See list</button>` : ""}</td></tr>`;
  }
  h += `</tbody></table><p class="muted small">${newAct() ? "Sections of the Income-tax Act, 2025; reported in Form 26 (tax audit under section 63)." : "Sections of the Income-tax Act, 1961; reported in Form 3CA/3CB with Form 3CD."} Loans count only the principal accepted or repaid; interest and TDS are kept apart.</p></div></div>`;
  // Loans
  const loans = a.loans || [];
  h += `<div class="box"><h2>Loans and deposits taken (₹)</h2>` + (loans.length ? `<div class="scroll"><table class="grid dense"><thead><tr><th>Lender</th><th class="num">Opening</th><th class="num">Taken by bank</th><th class="num">Taken in cash</th><th class="num">By journal</th><th class="num">Interest</th><th class="num">Repaid by bank</th><th class="num">Repaid in cash</th><th class="num">Closing</th><th class="num">Highest balance</th></tr></thead><tbody>` +
    loans.map(l => `<tr><td>${esc(l.ledger)}${l.exempt ? ' <span class="badge">bank / exempt</span>' : ""}</td>${[l.opening, l.accepted_bank, l.accepted_cash, l.accepted_journal, l.interest_credited, l.repaid_bank, l.repaid_cash, l.closing, l.max_outstanding].map(v => `<td class="num ${v && (v === l.accepted_cash || v === l.repaid_cash) && !l.exempt ? "hl" : ""}">${v ? rupees(v) : "-"}</td>`).join("")}</tr>`).join("") + "</tbody></table></div>" : '<p class="muted">No loan ledgers found (a day book is needed for movements).</p>') + "</div>";
  // Ratios
  if (a.ratios.length) {
    const f = v => (v == null ? "-" : v.toFixed(2));
    h += `<div class="box"><h2>Ratios</h2><table class="grid dense"><thead><tr><th>Ratio</th><th>Formula</th><th class="num">This year</th><th class="num">Last year</th><th class="num">Change</th></tr></thead><tbody>` +
      a.ratios.map(r => `<tr class="${r.needs_explanation ? "flag" : ""}"><td>${esc(r.name)}</td><td class="muted small">${esc(r.numerator)} / ${esc(r.denominator)}</td><td class="num">${f(r.cy)}${r.unit === "%" ? "%" : ""}</td><td class="num">${f(r.py)}${r.unit === "%" && r.py != null ? "%" : ""}</td><td class="num">${r.variance_pct == null ? "-" : r.variance_pct.toFixed(1) + "%"}${r.needs_explanation ? ' <span class="badge warning">over 25%</span>' : ""}</td></tr>`).join("") + "</tbody></table></div>";
  }
  // Ageing
  for (const [k, title] of [["receivables", "Ageing of debtors (₹)"], ["payables", "Ageing of creditors (₹)"]]) {
    const ag = a.ageing[k];
    if (!ag || !ag.rows.length) continue;
    h += `<div class="box"><h2>${title}</h2><div class="scroll"><table class="grid dense"><thead><tr><th>Category</th>${ag.bucket_labels.map(b => `<th class="num">${esc(b)}</th>`).join("")}</tr></thead><tbody>` +
      ag.rows.map(r => `<tr><td>${esc(r.category)}</td>${r.buckets.map(v => `<td class="num">${v ? rupees(Math.abs(v)) : "-"}</td>`).join("")}</tr>`).join("") + "</tbody></table></div></div>";
  }
  box.innerHTML = h;
  $$("[data-code]", box).forEach(b => b.addEventListener("click", () => {
    state.codes = [b.dataset.code]; state.codesLabel = b.dataset.label; show("check"); renderCheck();
  }));
}
$("#anRerun").addEventListener("click", runChecks);
$("#anWorkbook").addEventListener("click", async () => {
  const b = $("#anWorkbook"); b.disabled = true; b.textContent = "Saving...";
  try {
    const r = await api("POST", `/api/projects/${pid()}/export`, { mode: "draft", folder: "", files: { pdf: false, xlsx: false, html: false, auditor_workbook: true, json: false } });
    state.lastExport = r.dir; $("#openExport").disabled = false;
    toast("Auditor workbook saved.");
    if (await confirmBox("Auditor workbook saved", r.dir, "Open folder")) openFolder(r.dir);
  } catch (e) { toast(e.message, true); }
  b.disabled = false; b.textContent = "Save auditor workbook";
});

// ---- 6. presentation ----------------------------------------------------------
const isCo = () => state.settings?.entity_type === "company";
const toggleOn = v => v === "on" || (v === "auto" && isCo());
async function loadPresent() {
  const st = await reloadSettings();
  fillOpts(st.options);
  if (!state.analysis) { try { state.analysis = await api("POST", `/api/projects/${pid()}/analyse`); } catch (e) { toast(e.message, true); } }
  // Profit sharing (non-company).
  $("#psBox").hidden = isCo();
  const ps = $("#psRows"); ps.innerHTML = "";
  if (!isCo() && state.analysis) {
    const owners = state.analysis.mapping.filter(m => m.standard_group === "Capital Account" && !/drawing/i.test(m.name));
    for (const m of owners) {
      const cur = (st.profit_sharing.find(x => x[0] === m.name) || [m.name, 1])[1];
      ps.insertAdjacentHTML("beforeend", `<div class="ps-row"><div class="field"><label>${esc(m.name)}</label></div><div class="field"><input type="number" min="0" step="1" value="${cur}" data-owner="${esc(m.name)}" aria-label="Share of ${esc(m.name)}"></div></div>`);
    }
    if (!owners.length) ps.innerHTML = '<p class="muted small">No capital accounts found.</p>';
  }
  renderRatioBox();
  refreshPreview();
}
function fillOpts(o) {
  $("#oUnit").value = o.unit; $("#oDec").value = String(o.decimals);
  $$('input[name=layout]').forEach(r => (r.checked = r.value === o.layout));
  $("#oPy").checked = o.show_previous_year; $("#oNil").checked = o.hide_nil_lines; $("#oRel").checked = o.reletter;
  $("#oCover").checked = o.cover_page; $("#oPol").checked = o.accounting_policies; $("#oAge").checked = o.ageing; $("#oParty").checked = o.party_wise_details;
  $("#oPpe").checked = o.ppe_schedule; $("#oItDep").checked = o.tax_depreciation_annexure;
  $("#oCfOn").checked = toggleOn(o.cash_flow); $("#oRatiosOn").checked = toggleOn(o.ratios);
  const hidden = new Set(o.hidden_sections || []);
  $$("[data-sec]").forEach(c => (c.checked = !hidden.has(c.dataset.sec)));
  $("#oDetails").value = (o.entity_details || []).join("\n");
}
function renderRatioBox() {
  const o = state.settings.options;
  const rr = $("#ratioRows"); rr.innerHTML = "";
  const need = $("#oRatiosOn").checked ? (state.analysis?.ratios || []).filter(r => r.needs_explanation) : [];
  $("#ratioBox").hidden = !need.length;
  for (const r of need) {
    const div = document.createElement("div"); div.className = "ratio-row";
    div.innerHTML = `<div class="rh"><b>${esc(r.name)}</b><span class="muted">${r.variance_pct == null ? "" : (r.variance_pct > 0 ? "+" : "") + r.variance_pct.toFixed(1) + "%"}</span></div><textarea rows="2" data-ratio="${esc(r.name)}">${esc(o.ratio_explanations[r.name] || "")}</textarea><div><button class="small" type="button">Draft with AI</button></div>`;
    div.querySelector("button").addEventListener("click", async e => {
      const b = e.currentTarget; b.disabled = true;
      try { const x = await api("POST", `/api/projects/${pid()}/ai/ratio`, { name: r.name }); div.querySelector("textarea").value = x.text; toast("Draft added. Review before saving."); }
      catch (err) { toast(err.message, true); }
      b.disabled = false;
    });
    rr.appendChild(div);
  }
}
$("#oRatiosOn").addEventListener("change", renderRatioBox);
function refreshPreview() { $("#preview").src = `/api/projects/${pid()}/preview?t=${TOKEN}&ts=${Date.now()}`; }
$("#refreshPreview").addEventListener("click", refreshPreview);
function readOpts() {
  const o = state.settings.options;
  return {
    ...o,
    unit: $("#oUnit").value, decimals: Number($("#oDec").value),
    layout: ($$('input[name=layout]').find(r => r.checked) || {}).value || "boxed",
    show_previous_year: $("#oPy").checked, hide_nil_lines: $("#oNil").checked, reletter: $("#oRel").checked,
    cover_page: $("#oCover").checked, accounting_policies: $("#oPol").checked, ageing: $("#oAge").checked, party_wise_details: $("#oParty").checked,
    ppe_schedule: $("#oPpe").checked, tax_depreciation_annexure: $("#oItDep").checked,
    cash_flow: $("#oCfOn").checked ? "on" : "off", ratios: $("#oRatiosOn").checked ? "on" : "off",
    hidden_sections: $$("[data-sec]").filter(c => !c.checked).map(c => c.dataset.sec),
    entity_details: $("#oDetails").value.split("\n").map(x => x.trim()).filter(Boolean),
    ratio_explanations: { ...o.ratio_explanations, ...Object.fromEntries($$("#ratioRows textarea").map(t => [t.dataset.ratio, t.value.trim()]).filter(x => x[1])) },
  };
}
async function saveOpts(options) {
  const profit_sharing = $$("#psRows input").map(i => [i.dataset.owner, Math.max(0, Math.round(Number(i.value) || 0))]);
  setStatus($("#oStatus"), "Saving...");
  try {
    await api("POST", `/api/projects/${pid()}/settings`, { options, profit_sharing });
    state.settings.options = options; state.settings.profit_sharing = profit_sharing; state.analysis = null;
    setStatus($("#oStatus"), "Saved."); toast("Saved."); refreshPreview(); renderNext();
  } catch (err) { setStatus($("#oStatus"), err.message, true); }
}
$("#opts").addEventListener("submit", e => { e.preventDefault(); saveOpts(readOpts()); });
$("#resetOpts").addEventListener("click", async () => {
  if (!(await confirmBox("Reset to the standard choices?", "Units, table style and what to print go back to the usual settings. Address lines and ratio reasons are kept.", "Reset"))) return;
  const o = state.settings.options;
  const std = { ...o, unit: "rupees", decimals: 2, layout: "boxed", show_previous_year: true, hide_nil_lines: true, reletter: true, cover_page: true,
    accounting_policies: true, cash_flow: "auto", ratios: "auto", ageing: true, ppe_schedule: true, tax_depreciation_annexure: true, party_wise_details: false, hidden_sections: [] };
  fillOpts(std); renderRatioBox(); saveOpts(std);
});

// ---- 7. sign and export -----------------------------------------------------
function sigRow(sg = {}) {
  const d = document.createElement("div"); d.className = "sig-row";
  d.innerHTML = `<div class="field"><label>Name</label><input data-k="name" value="${esc(sg.name || "")}"></div>
    <div class="field"><label>Designation</label><input data-k="designation" value="${esc(sg.designation || "")}" placeholder="Partner / Director"></div>
    <div class="field"><label>ID type</label><input data-k="id_label" value="${esc(sg.id_label || "")}" placeholder="DIN"></div>
    <div class="field"><label>ID number</label><input data-k="id" value="${esc(sg.id || "")}"></div>
    <button type="button" class="small danger-ghost" aria-label="Remove signatory">Remove</button>`;
  d.querySelector("button").addEventListener("click", () => d.remove());
  $("#sigRows").appendChild(d);
}
async function loadSign() {
  const form = $("#signForm"); form.inert = true; form.setAttribute("aria-busy", "true");
  const st = await reloadSettings();
  const s = st.signoff;
  $("#sFirm").value = s.auditor_firm; $("#sFrn").value = s.frn; $("#sPartner").value = s.auditor_partner; $("#sMno").value = s.membership_no;
  $("#sUdin").value = s.udin; $("#sPlace").value = s.place; $("#sDate").value = s.date;
  $("#sigRows").innerHTML = ""; (s.signatories.length ? s.signatories : [{}]).forEach(sigRow);
  const files = store.get("files", null);
  if (files) $$("[data-file]").forEach(c => (c.checked = files[c.dataset.file] ?? c.checked));
  if (!state.analysis) { try { state.analysis = await api("POST", `/api/projects/${pid()}/analyse`); } catch {} }
  const blocked = state.analysis && state.analysis.summary.must_fix > 0;
  $("#modeFinal").disabled = !!blocked;
  if (blocked) $$('input[name=mode]').forEach(r => (r.checked = r.value === "draft"));
  setStatus($("#eStatus"), blocked ? `Final copy is locked until ${state.analysis.summary.must_fix} "Must fix" item(s) are cleared. Draft export is available.` : "");
  form.inert = false; form.removeAttribute("aria-busy"); form.dataset.ready = "1";
  renderNext();
}
$("#addSig").addEventListener("click", () => sigRow());
$("#openExport").addEventListener("click", () => state.lastExport && openFolder(state.lastExport));
$("#signForm").addEventListener("submit", async e => {
  e.preventDefault();
  const signoff = {
    auditor_firm: $("#sFirm").value.trim(), frn: $("#sFrn").value.trim(), auditor_partner: $("#sPartner").value.trim(), membership_no: $("#sMno").value.trim(),
    udin: $("#sUdin").value.trim(), place: $("#sPlace").value.trim(), date: $("#sDate").value.trim(),
    signatories: $$("#sigRows .sig-row").map(r => Object.fromEntries($$("input", r).map(i => [i.dataset.k, i.value.trim()]))).filter(x => x.name || x.designation),
  };
  const files = Object.fromEntries($$("[data-file]").map(c => [c.dataset.file, c.checked]));
  store.set("files", files);
  setStatus($("#eStatus"), "Exporting...");
  try {
    await api("POST", `/api/projects/${pid()}/settings`, { signoff });
    const mode = ($$('input[name=mode]').find(r => r.checked) || {}).value;
    const r = await api("POST", `/api/projects/${pid()}/export`, { mode, folder: $("#eFolder").value, files });
    setStatus($("#eStatus"), "Done.");
    state.lastExport = r.dir; $("#openExport").disabled = false;
    const box = $("#eResult"); box.hidden = false;
    box.innerHTML = `<div class="title-row"><h2>Saved</h2><button type="button" class="primary small" id="openExport2">Open folder</button></div><p>Folder: <b>${esc(r.dir)}</b></p><ul>${(r.files || []).map(f => `<li>${esc(f[0])}</li>`).join("")}</ul>` +
      (r.warnings?.length ? `<h2>Before signing</h2><ul>${r.warnings.map(w => `<li>${esc(w)}</li>`).join("")}</ul>` : "");
    $("#openExport2").addEventListener("click", () => openFolder(r.dir));
  } catch (err) { setStatus($("#eStatus"), err.message, true); }
});

// ---- 8. audit ---------------------------------------------------------------
const short = h => (h ? String(h).slice(0, 10) + "..." : "");
const amt = p => `₹${rupees(Math.abs(p))} ${p >= 0 ? "Dr" : "Cr"}`;
function describe(e) {
  const d = e.details || {};
  switch (e.action) {
    case "project_created": return `Started ${d.entity}, FY ${d.fy}`;
    case "project_deleted": return "Moved to the Recycle Bin";
    case "project_restored": return "Restored from the Recycle Bin";
    case "file_imported": return `${d.original_name}: ${d.contents} (fingerprint ${short(d.sha256)})`;
    case "file_removed": return `Stopped using ${d.file} (${d.kind})`;
    case "tally_import": return `From Tally company "${d.company}": ${d.ledgers} ledgers, ${d.vouchers} vouchers${d.previous_year ? ", last year's balances" : ""}`;
    case "checks_run": return `Must fix ${d.must_fix}, Check ${d.check}, Notes ${d.notes} (rules ${d.rules_version})`;
    case "mapping_changed": {
      const lbl = id => (state.heads.find(h => h.id === id) || {}).label || id;
      return `${d.ledger}: ${d.from ? lbl(d.from) : "automatic"} to ${d.to ? lbl(d.to) : "automatic"}${d.ai_suggested ? " (AI suggestion accepted)" : ""}`;
    }
    case "tags_changed": return `${d.ledger}: tags ${(d.to || []).join(", ") || "removed"}`;
    case "adjustment_added": case "adjustment_changed": {
      const a = d.to || {};
      return `No. ${a.id}: ${a.narration} (${(a.lines || []).filter(l => l.amount).map(l => `${l.ledger} ${amt(l.amount)}`).join("; ")})`;
    }
    case "adjustment_switched": return `No. ${d.id} ${d.active ? "applied" : "switched off"}: ${d.narration}`;
    case "adjustment_deleted": return `No. ${d.id} deleted: ${(d.was || {}).narration || ""}`;
    case "settings_changed": {
      const parts = [];
      for (const [k, v] of Object.entries(d)) {
        if (v && typeof v.from === "object" && v.from && v.to && !Array.isArray(v.to)) {
          const ch = Object.keys(v.to).filter(x => JSON.stringify(v.to[x]) !== JSON.stringify(v.from[x]));
          parts.push(`${k.replace(/_/g, " ")}: ${ch.map(x => `${x.replace(/_/g, " ")} ${JSON.stringify(v.from[x])} to ${JSON.stringify(v.to[x])}`).join(", ")}`);
        } else parts.push(`${k.replace(/_/g, " ")} changed`);
      }
      return parts.join("; ");
    }
    case "exported": return `${d.mode === "final" ? "Final signing copy" : "Draft"}: ${(d.files || []).length} files to ${d.folder}${d.udin ? ", UDIN " + d.udin : ""}`;
    case "ai_suggestion": return `AI ${d.action} (${d.response && d.response.model}): ${String((d.response && (d.response.text || d.response.head_label)) || "").slice(0, 120)}`;
    default: return "";
  }
}
async function loadAudit() {
  await loadHeads();
  const r = await api("GET", `/api/projects/${pid()}/audit`);
  const c = $("#chain");
  c.className = "chain " + (r.status.intact ? "ok" : "bad");
  c.textContent = r.status.intact ? `Audit trail intact: ${r.status.events} events, each sealed with the fingerprint of the one before.` : `Audit trail damaged: ${r.status.problem}.`;
  const tb = $("#auditTable tbody"); tb.innerHTML = "";
  for (const e of r.events.slice().reverse()) {
    const tr = document.createElement("tr");
    tr.innerHTML = `<td class="num">${e.seq}</td><td>${esc(e.ts.slice(0, 19).replace("T", " "))}</td><td>${esc(e.actor)}</td><td>${esc(e.action.replace(/_/g, " "))}</td><td><div>${esc(describe(e))}</div><details><summary class="small muted">Full record</summary><div class="details">${esc(JSON.stringify(e.details))}</div><div class="details">Seal ${esc(e.hash)}</div></details></td>`;
    tb.appendChild(tr);
  }
}
$("#reloadAudit").addEventListener("click", loadAudit);

// ---- start ------------------------------------------------------------------
renderGuides();
applyFlow();
show("home");
loadProjects().catch(e => toast(e.message, true));
refreshStatus();
if (!store.get("tourSeen", false)) { openTour(); store.set("tourSeen", true); }
