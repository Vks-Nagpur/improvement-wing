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
    // Open on the first visit to a screen, folded after that (the user can reopen it).
    const seen = store.get("guideSeen", {})[g.dataset.guide];
    g.innerHTML = `<details${seen ? "" : " open"}><summary class="gt">${esc(lang()._title)}</summary><ol>${steps.map(s => `<li>${s}</li>`).join("")}</ol></details>`;
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
    const un = a.mapping.filter(m => !m.head).length, pend = a.mapping.filter(m => ["suggested", "review"].includes(m.status)).length;
    mark("map", un ? `${un} not placed` : pend ? `${pend} to confirm` : "All confirmed", un ? "bad" : pend ? "todo" : "ok");
  }
  const adj = st.adjustments || [], on = adj.filter(x => x.active).length;
  mark("adjust", adj.length ? `${on} applied${adj.length > on ? `, ${adj.length - on} off` : ""}` : "Optional", adj.length ? "ok" : "");
  mark("present", `In ${st.options.unit}, ${st.options.layout}`);
  const dd = st.options.disclosures || {};
  const nd = ["share_classes", "contingent_liabilities", "commitments", "related_parties", "notes", "extra_policies"].reduce((n, k) => n + (dd[k] || []).length, 0) + Object.keys(dd.policy_text || {}).length;
  const open = openDisclosures(st);
  mark("disclose", open ? `${open} section${open === 1 ? "" : "s"} not answered` : `All answered${nd ? `, ${nd} items` : ""}`, open ? "todo" : "ok");
  mark("analysis", a ? `${a.ratios.length} ratios, ${a.loans.length} loans` : "", a ? "ok" : "");
  const fb = a ? a.summary.must_fix + (a.blockers || []).length + (a.legal && a.legal.statements && !a.legal.statements.ready ? 1 : 0) : 0;
  mark("export", a ? (fb ? "Draft only" : "Ready to sign") : "", a && !fb ? "ok" : "");
}
// Disclosure sections still to be answered (never assumed nil).
function openDisclosures(st) {
  const dd = st.options.disclosures || {};
  const req = ["contingent", "related_parties", "msme"].concat(st.entity_type === "company" ? ["share_capital"] : []);
  const has = { share_capital: (dd.share_classes || []).length, contingent: (dd.contingent_liabilities || []).length + (dd.commitments || []).length, related_parties: (dd.related_parties || []).length, msme: 0 };
  return req.filter(k => (dd.pending_review || []).includes(k) || (!has[k] && !(dd.answers || {})[k])).length;
}
function nextAction() {
  const L = lang().next, st = state.settings, a = state.analysis;
  if (state.view === "home") return [L.intent, "home"];
  if (!st) return [L.create, "projects"];
  if (!st.inputs.tb) return [L.tb, "import"];
  if (!a) return [L.run, "check"];
  const un = a.mapping.filter(m => !m.head || ["suggested", "review"].includes(m.status)).length;
  if (un && state.intent === "statements") return [L.map(un), "map"];
  if (a.summary.must_fix) return [L.fix(a.summary.must_fix), "check"];
  if (state.intent !== "statements") return [L.analyse, "analysis"];
  return [L.ready, "present"];
}
function renderNext() {
  stepStatus();
  const st = state.settings;
  $("#sbClient").textContent = st ? `${st.entity_name} · ${TYPE_LABEL[st.entity_type] || ""} · ${st.period ? `${st.period.start} to ${st.period.end}` : `FY ${st.fy}`}` : "No client open";
  $("#sbRules").textContent = st ? (Number(st.fy.slice(0, 4)) >= 2026 ? "Income-tax Act, 2025 · Form 26" : "Income-tax Act, 1961 · Form 3CD") + (state.rules ? ` · Rules ${state.rules.pinned.rules}${state.rules.update ? " (newer available)" : ""}` : "") : "";
  const [text, view] = nextAction();
  $("#nextBox").hidden = false;
  $("#nextText").textContent = text;
  $("#nextGo").hidden = view === state.view;
  $("#nextGo").onclick = () => show(view);
}

// ---- what the user wants to do (sets the steps) --------------------------------
const FLOWS = {
  statements: { name: "Financial statements", steps: ["projects", "import", "check", "analysis", "map", "bank", "portal", "adjust", "disclose", "present", "export", "audit"] },
  analysis: { name: "Check and analyse", steps: ["projects", "import", "check", "analysis", "bank", "portal", "map", "adjust", "audit"] },
  taxaudit: { name: "Tax audit help", steps: ["projects", "import", "check", "analysis", "bank", "portal", "adjust", "export", "audit"] },
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
  // "Next" buttons lead to the following step of the chosen path.
  for (const b of $$("[data-next]")) {
    const here = b.closest("section[data-panel]").dataset.panel;
    const nx = f.steps[f.steps.indexOf(here) + 1];
    b.hidden = !nx;
    if (nx) { b.dataset.target = nx; b.textContent = `Next: ${$(`.steps button[data-view="${nx}"] .sn`).textContent.toLowerCase()}`; }
  }
}
$$("[data-next]").forEach(b => b.addEventListener("click", () => b.dataset.target && show(b.dataset.target)));
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
  // Remember that this screen's help was shown once.
  const gs = store.get("guideSeen", {});
  if (!gs[view]) { setTimeout(() => { const m = store.get("guideSeen", {}); m[view] = true; store.set("guideSeen", m); }, 0); }
  if (view === "present") loadPresent();
  if (view === "export") loadSign();
  if (view === "audit") loadAudit();
  if (view === "legal") loadLegal().catch(e => toast(e.message, true));
  if (view === "adjust") loadAdjustments();
  if (view === "disclose") loadDisclosures();
  if (view === "bank") loadBank();
  if (view === "portal") loadPortal();
  if ((view === "map" || view === "check") && !state.analysis && state.settings?.inputs.tb) runChecks();
  if (view === "check") loadRules();
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
function nextFy(fy) { const y = Number(fy.slice(0, 4)) + 1; return `${y}-${String((y + 1) % 100).padStart(2, "0")}`; }
function hasNext(p) { return (state.projects || []).some(x => x.entity_name === p.entity_name && x.fy === nextFy(p.fy)); }
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
      <td class="act"><button class="small ${cur ? "" : "primary"}" data-a="open">${cur ? "Open now" : "Open"}</button>${hasNext(p) ? "" : '<button class="small" data-a="next" title="Create the next financial year from this one">Start next year</button>'}<button class="small danger-ghost" data-a="del">Delete</button></td>`;
    tr.querySelector('[data-a="open"]').addEventListener("click", () => openProject(p.id));
    tr.querySelector('[data-a="next"]')?.addEventListener("click", async () => {
      const ny = nextFy(p.fy);
      if (!(await confirmBox(`Start FY ${ny} for ${p.entity_name}?`, `FY ${p.fy}'s final balances (with its adjustments) become last year's figures. The fixed asset register, settings, signing details, tags and disclosures are carried forward; every disclosure section must be answered again. Then import FY ${ny}'s books.`, "Start next year"))) return;
      try { const r = await api("POST", `/api/projects/${encodeURIComponent(p.id)}/roll-forward`); toast(`FY ${ny} created.`); await loadProjects(); openProject(r.id); }
      catch (e) { toast(e.message, true); }
    });
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
  state.id = id; state.analysis = null; state.lastExport = null; state.adj = null; state.rules = null;
  const st = await reloadSettings();
  $("#projectName").textContent = `${st.entity_name}  |  ${TYPE_LABEL[st.entity_type] || ""}  |  FY ${st.fy}`;
  $$(".steps button").forEach(b => (b.disabled = false));
  applyFlow();
  $("#openExport").disabled = true; $("#eResult").hidden = true;
  renderFiles(); renderBranches();
  $("#depBasis").value = st.depreciation_basis;
  $("#depConfirmRow").hidden = st.depreciation_basis !== "income_tax_rates";
  $("#depConfirm").checked = !!st.options.depreciation_basis_confirmed;
  loadProjects();
  store.set("last", { id, name: st.entity_name, fy: st.fy, intent: state.intent });
  show(flowNext());
  loadRules();
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
// Branches: each has its own trial balance and day book.
function renderBranches() {
  const box = $("#branchRows"); box.innerHTML = "";
  const list = state.settings.inputs.branches || [];
  $("#tInto").innerHTML = '<option value="">Head office</option>' + list.map(b => `<option value="${esc(b.name)}">Branch: ${esc(b.name)}</option>`).join("");
  if (!list.length) { box.innerHTML = '<p class="list-empty muted small">No branches. Head office books only.</p>'; return; }
  for (const b of list) {
    const row = document.createElement("div"); row.className = "file-row";
    const tb = b.tb ? (b.tally_company ? `Trial balance from Tally: ${esc(b.tally_company)}` : "Trial balance loaded") : "Trial balance needed";
    row.innerHTML = `<div><div class="name">${esc(b.name)}</div><div class="state ${b.tb ? "ok" : ""}">${tb}${b.vouchers ? ", day book loaded" : ""}</div></div>
      <div class="act"><label class="btnlike small"><input type="file" data-k="branch_tb" accept=".xlsx,.xls,.csv,.xlsm,.ods" hidden><span tabindex="0" role="button">${b.tb ? "Replace" : "Choose"} trial balance</span></label><label class="btnlike small"><input type="file" data-k="branch_vouchers" accept=".xlsx,.xls,.csv,.xlsm,.ods" hidden><span tabindex="0" role="button">${b.vouchers ? "Replace" : "Choose"} day book</span></label><button type="button" class="small danger-ghost">Remove</button></div>`;
    $$("input[type=file]", row).forEach(input => {
      input.parentElement.querySelector("[role=button]").addEventListener("keydown", e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); input.click(); } });
      input.addEventListener("change", async () => {
        const f = input.files[0]; if (!f) return;
        setStatus($("#fStatus"), `Reading ${f.name}...`);
        try {
          const r = await api("POST", `/api/projects/${pid()}/upload?kind=${input.dataset.k}&branch=${encodeURIComponent(b.name)}&name=${encodeURIComponent(f.name)}`, undefined, await f.arrayBuffer());
          setStatus($("#fStatus"), `${b.name}: ${f.name}: ${r.contents} loaded.`);
          await reloadSettings(); state.analysis = null; renderBranches(); renderNext();
        } catch (e) { setStatus($("#fStatus"), e.message, true); }
      });
    });
    row.querySelector(".danger-ghost").addEventListener("click", async () => {
      if (!(await confirmBox(`Remove branch ${b.name}?`, "Its books are no longer added to the head office. The files stay in the client's folder.", "Remove"))) return;
      try { await api("POST", `/api/projects/${pid()}/branches/${encodeURIComponent(b.name)}/remove`); await reloadSettings(); state.analysis = null; renderBranches(); toast("Branch removed."); }
      catch (e) { toast(e.message, true); }
    });
    box.appendChild(row);
  }
}
$("#branchAdd").addEventListener("submit", async e => {
  e.preventDefault();
  try { await api("POST", `/api/projects/${pid()}/branches`, { name: $("#branchName").value }); $("#branchName").value = ""; await reloadSettings(); state.analysis = null; renderBranches(); toast("Branch added. Now import its trial balance."); }
  catch (err) { toast(err.message, true); }
});
$("#depConfirm").addEventListener("change", async () => {
  await api("POST", `/api/projects/${pid()}/settings`, { depreciation_basis_confirmed: $("#depConfirm").checked });
  state.analysis = null; await reloadSettings(); toast("Saved.");
});
$("#depBasis").addEventListener("change", async () => {
  await api("POST", `/api/projects/${pid()}/settings`, { depreciation_basis: $("#depBasis").value });
  $("#depConfirmRow").hidden = $("#depBasis").value !== "income_tax_rates"; $("#depConfirm").checked = false;
  state.analysis = null; await reloadSettings(); toast("Depreciation basis saved.");
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
    const r = await api("POST", `/api/projects/${pid()}/tally`, { host: $("#tHost").value, port: Number($("#tPort").value), company, vouchers: $("#tVouchers").checked, branch: $("#tInto").value });
    setStatus($("#tStatus"), `${r.branch ? `Branch ${r.branch}: imported` : "Imported"} ${r.ledgers} ledgers, ${r.vouchers} vouchers${r.previous_year ? ", and last year's balances" : ""}.`);
    await reloadSettings(); state.analysis = null; renderFiles(); renderBranches(); renderNext();
  } catch (e) { setStatus($("#tStatus"), e.message, true); }
  $("#tImport").disabled = false;
});

// ---- 3. check -------------------------------------------------------------
const SEV = { blocker: "Must fix", review: "Review", warning: "Check", info: "Note" };
// Rule and format packs pinned to this year (TRUTH-MODEL §8).
async function loadRules() {
  if (!state.id) return;
  try { state.rules = await api("GET", `/api/projects/${pid()}/rules`); } catch { state.rules = null; return; }
  const r = state.rules, bar = $("#rulesBar");
  bar.hidden = false;
  bar.classList.toggle("update", r.update);
  bar.innerHTML = `<span>This year uses rule pack <b>${esc(r.pinned.rules)}</b> and the format <b>${esc(r.pinned.format)}</b> (${esc(r.pinned.format_status)}). ${r.pinned.unverified_rules} legal reference${r.pinned.unverified_rules === 1 ? " is" : "s are"} not yet verified against the official text.</span>` +
    (r.update ? `<span class="spacer"></span><button type="button" class="small primary" id="rulesReview">Newer rules available: review</button>` : "");
  $("#rulesReview")?.addEventListener("click", async () => {
    const list = r.changes.length ? r.changes.slice(0, 20).join(", ") + (r.changes.length > 20 ? ", …" : "") : "no rule changes";
    const ok = await confirmBox(`Move this year to rule pack ${r.available.rules}?`,
      `Now: ${r.pinned.rules}. New: ${r.available.rules}.\nChanged rules: ${list}.\nFormat changed: ${r.format_changed ? "yes" : "no"}.\n\nEarlier exports stay as they were. The move is recorded in the audit trail.`, "Move to new rules");
    if (!ok) return;
    try { await api("POST", `/api/projects/${pid()}/rules/migrate`); toast("Moved to the new rules."); state.analysis = null; await loadRules(); runChecks(); }
    catch (e) { toast(e.message, true); }
  });
  renderNext();
}
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

// Why a rule fired: how it was detected, what LedgerCraft cannot know,
// possible exceptions, the reference and its verification status.
function whyFired(f) {
  const rows = [];
  if (f.detection_basis) rows.push(`<p><b>How it was detected:</b> ${esc(f.detection_basis)}</p>`);
  if ((f.unknown_facts || []).length) rows.push(`<p><b>Not known to LedgerCraft (you decide):</b></p><ul>${f.unknown_facts.map(x => `<li>${esc(x)}</li>`).join("")}</ul>`);
  if ((f.possible_exceptions || []).length) rows.push(`<p><b>Possible exceptions:</b></p><ul>${f.possible_exceptions.map(x => `<li>${esc(x)}</li>`).join("")}</ul>`);
  if (f.legal_ref) rows.push(`<p><b>Reference:</b> ${esc(f.legal_ref)}${f.verification_status ? ` <span class="badge ${f.verification_status === "verified" ? "okb" : f.verification_status === "not verified" ? "warning" : "info"}">${esc(f.verification_status)}</span>` : ""}</p>`);
  rows.push(`<p><b>Blocks a final copy:</b> ${f.blocks_final ? "yes" : "no"}${f.professional_review_required ? " · needs your professional judgement" : ""}</p>`);
  if (!f.detection_basis && !f.legal_ref && !expert()) return "";
  return `<details class="why"><summary>Why this was flagged</summary>${rows.join("")}</details>`;
}
function renderCheck() {
  const a = state.analysis; if (!a) return;
  const s = a.summary;
  $("#tiles").innerHTML = `
    <div class="tile ${s.must_fix ? "blocker" : ""}"><div class="k">Must fix</div><div class="v">${s.must_fix}</div></div>
    <div class="tile ${s.review ? "review" : ""}"><div class="k">Review (your judgement)</div><div class="v">${s.review || 0}</div></div>
    <div class="tile ${s.check ? "warning" : ""}"><div class="k">Check</div><div class="v">${s.check}</div></div>
    <div class="tile"><div class="k">Notes</div><div class="v">${s.notes}</div></div>
    <div class="tile"><div class="k">Profit / (loss)</div><div class="v money">${esc(s.profit)}</div></div>
    <div class="tile"><div class="k">Balance sheet total</div><div class="v money">${esc(s.total_assets)}</div></div>
    <div class="tile"><div class="k">Data</div><div class="v sm">${s.ledgers} ledgers, ${s.vouchers} vouchers${s.has_previous_year ? ", last year" : ""}${s.has_far ? ", asset register" : ""}${s.units && s.units.length > 1 ? `; consolidated: ${s.units.length} units` : ""}</div></div>`;
  const ul = $("#findings"); ul.innerHTML = "";
  const codes = state.codes;
  const list = a.findings.filter(f => (codes ? codes.includes(f.code) : state.filter === "all" || f.severity === state.filter));
  $("#codeFilter").hidden = !codes;
  if (codes) $("#codeFilter span").textContent = state.codesLabel;
  if (!list.length) ul.innerHTML = `<li class="empty">${a.findings.length ? "Nothing in this filter." : "No issues found. The books pass every check."}</li>`;
  for (const f of list) {
    const li = document.createElement("li"); li.className = "finding";
    li.innerHTML = `<span class="badge ${f.severity}">${SEV[f.severity]}</span><span class="t">${esc(f.title)}</span>
      <span class="fa">${f.ledger ? '<button class="small" type="button" data-a="map">Map this ledger</button>' : f.code.startsWith("MAPPING_") ? '<button class="small primary" type="button" data-a="maps">Open Map ledgers</button>' : ""}<button class="small" type="button" data-a="ai">Explain</button></span>
      <div class="m">${esc(f.message)}</div>
      ${f.suggestion ? `<div class="s"><b>What to do:</b> ${esc(f.suggestion)}</div>` : ""}
      ${whyFired(f)}`;
    li.querySelector('[data-a="maps"]')?.addEventListener("click", () => { $("#mapSearch").value = ""; $("#mapAttention").checked = true; show("map"); renderMap(); });
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
  return !m.head || !m.standard_group || m.reclassified || ["suggested", "review", "unmapped"].includes(m.status) || [...keys].some(k => k.startsWith("MISGROUP") || k === "UNKNOWN_GROUP" || k === "CAPITAL_IN_EXPENSE" || k === "ABNORMAL_BALANCE");
}
async function setHead(m, head, ai) {
  try {
    await api("POST", `/api/projects/${pid()}/mapping`, { ledger: m.name, head, ai: !!ai });
    toast(head ? "Saved. Remembered for this client." : "Back to LedgerCraft's own choice.");
    await runChecks();
  } catch (err) { toast(err.message, true); }
}
const STATUS = { rule: ["Certain", "info"], suggested: ["Confirm", "warning"], review: ["Review", "warning"], confirmed: ["Your choice", "okb"], unmapped: ["Not placed", "blocker"] };
async function confirmMap(body) {
  try {
    const r = await api("POST", `/api/projects/${pid()}/mapping/confirm`, body);
    toast(`${r.confirmed} placement${r.confirmed === 1 ? "" : "s"} confirmed and remembered.`);
    await runChecks();
  } catch (e) { toast(e.message, true); }
}
$("#mapConfirmAll").addEventListener("click", async () => {
  const n = (state.analysis?.mapping || []).filter(m => ["suggested", "review"].includes(m.status)).length;
  if (await confirmBox(`Confirm ${n} placement${n === 1 ? "" : "s"}?`, "Each ledger stays where it is shown now. You confirm that you have looked at them. This is recorded in the audit trail with the full list.", "Confirm all")) confirmMap({ all: true });
});
async function renderMap() {
  await loadHeads();
  const a = state.analysis; if (!a) return;
  const q = $("#mapSearch").value.toLowerCase(), only = $("#mapAttention").checked;
  const tb = $("#mapTable tbody"); tb.innerHTML = "";
  const rows = a.mapping.filter(m => (!only || needsAttention(m)) && (!q || m.name.toLowerCase().includes(q) || m.group.toLowerCase().includes(q)));
  const pend = a.mapping.filter(m => ["suggested", "review"].includes(m.status)).length;
  const rule = a.mapping.filter(m => m.status === "rule").length, conf = a.mapping.filter(m => m.status === "confirmed").length, un = a.mapping.filter(m => !m.head).length;
  $("#mapSummary").innerHTML = `<span class="badge okb">${conf} confirmed by you</span> <span class="badge info">${rule} certain by group</span> <span class="badge warning">${pend} waiting for your confirmation</span>${un ? ` <span class="badge blocker">${un} not placed</span>` : ""}`;
  $("#mapConfirmAll").disabled = !pend;
  if (!rows.length) tb.innerHTML = `<tr><td colspan="8" class="empty">${only ? "Nothing needs attention. Untick the box to see every ledger." : "No ledger matches."}</td></tr>`;
  const LIMIT = state.mapLimit || 300;
  for (const m of rows.slice(0, LIMIT)) {
    const tr = document.createElement("tr");
    // Only the chosen line is rendered; the full list is filled when the box is opened (fast with thousands of ledgers).
    const opts = m.head ? `<option value="${m.head}" selected>${esc(m.head_label || m.head)}</option>` : "";
    const why = reasons(m);
    const mine = m.source === "memory";
    tr.innerHTML = `<td>${needsAttention(m) ? '<span class="needs" aria-hidden="true"></span>' : ""}${esc(m.name)}${why.length ? `<div class="src">${esc(why.join("; "))}</div>` : ""}</td>
      <td>${esc(m.group)}${m.standard_group ? "" : ' <span class="badge blocker">group not recognised</span>'}</td>
      <td class="num">${esc(m.amount)}</td><td class="num muted">${m.py ? esc(m.py) : m.new_this_year ? '<span class="badge">new</span>' : "-"}</td>
      <td><select class="head" aria-label="Shown under for ${esc(m.name)}">${m.head ? "" : '<option value="" selected>Not placed: choose a line</option>'}${opts}</select>
</td>
      <td><span class="badge ${STATUS[m.status]?.[1] || ""}">${STATUS[m.status]?.[0] || ""}</span><div class="src">${esc(m.status_reason || "")}</div></td>
      <td><div class="tags">${TAGS.map(t => `<button type="button" class="tag ${m.tags.includes(t) ? "on" : ""}" data-t="${t}" title="${TAG_TIP[t]}">${t}</button>`).join("")}</div></td>
      <td class="act">${["suggested", "review"].includes(m.status) ? '<button class="small primary" type="button" data-a="ok">Confirm</button>' : ""}<button class="small" type="button" data-a="ai">Suggest</button>${mine ? `<button class="small" type="button" data-a="reset" title="Go back to LedgerCraft's own choice">Reset</button>` : ""}</td>`;
    const sel = tr.querySelector("select");
    const fill = () => {
      if (sel.dataset.full) return; sel.dataset.full = "1";
      sel.innerHTML = (m.head ? "" : '<option value="" selected>Not placed: choose a line</option>') + state.heads.filter(h => h.id === m.head || (state.settings?.entity_type === "company" ? h.id !== "PARTNERS_REMUNERATION" : h.id !== "MANAGERIAL_REMUNERATION")).map(h => `<option value="${h.id}" ${h.id === m.head ? "selected" : ""}>${esc(h.label)}</option>`).join("");
    };
    sel.addEventListener("mousedown", fill); sel.addEventListener("focus", fill); sel.addEventListener("keydown", fill);
    sel.addEventListener("change", e => e.target.value && e.target.value !== m.head && setHead(m, e.target.value));
    tr.querySelector('[data-a="reset"]')?.addEventListener("click", () => setHead(m, ""));
    tr.querySelector('[data-a="ok"]')?.addEventListener("click", () => confirmMap({ ledgers: [m.name] }));
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
    tr.innerHTML = `<td colspan="7" class="empty">Showing ${LIMIT} of ${rows.length}. <button type="button" class="small">Show all</button> or search above.</td>`;
    tr.querySelector("button").onclick = () => { state.mapLimit = 1e9; renderMap(); };
    tb.appendChild(tr);
  }
}
let mapTimer;
$("#mapSearch").addEventListener("input", () => { clearTimeout(mapTimer); mapTimer = setTimeout(() => { state.mapLimit = 0; renderMap(); }, 150); });
$("#mapAttention").addEventListener("change", () => { state.mapLimit = 0; renderMap(); });

// ---- bank reconciliation -------------------------------------------------------
async function loadBank() {
  const box = $("#bankBody");
  let d;
  try { d = await api("GET", `/api/projects/${pid()}/bankrec`); }
  catch (e) { box.innerHTML = `<p class="empty">${esc(e.message)}</p>`; return; }
  if (!d.ledgers.length) { box.innerHTML = '<p class="empty">No bank ledgers in this trial balance.</p>'; return; }
  const m = p => (p < 0 ? `(${rupees(-p)})` : rupees(p));
  const sum = a => a.reduce((t, x) => t + x.amount, 0);
  box.innerHTML = "";
  for (const l of d.ledgers) {
    const r = d.recs.find(x => x.ledger === l.name);
    const card = document.createElement("div"); card.className = "box";
    let body = `<div class="title-row"><h2>${esc(l.name)}</h2><label class="btnlike small"><input type="file" accept=".xlsx,.xls,.csv,.xlsm,.ods" hidden><span tabindex="0" role="button">${r ? "Replace" : "Upload"} bank statement</span></label></div>
      <p class="muted small">Balance as per books: ${esc(l.balance)}</p>`;
    if (r) {
      const diff = r.difference;
      const rows = [
        ["Balance as per books", r.book_balance, true],
        [`Less: deposited, not yet credited by the bank (${r.deposited_not_cleared.length})`, -sum(r.deposited_not_cleared)],
        [`Add: payments not yet presented (${r.issued_not_presented.length})`, -sum(r.issued_not_presented)],
        [`Add: credited by the bank, not in the books (${r.credited_by_bank_only.length})`, sum(r.credited_by_bank_only)],
        [`Less: debited by the bank, not in the books (${r.debited_by_bank_only.length})`, sum(r.debited_by_bank_only)],
        ["Balance as per bank (worked out)", r.computed_statement_balance, true],
      ];
      if (r.statement_balance != null) rows.push(["Balance shown by the statement", r.statement_balance, true]);
      body += `<table class="grid dense brs"><tbody>${rows.map(([t, v, b]) => `<tr class="${b ? "strong" : ""}"><td>${esc(t)}</td><td class="num">${m(v)}</td></tr>`).join("")}</tbody></table>
        <p class="${diff == null ? "muted" : diff === 0 ? "okt" : "errt"} small mt">${diff == null ? "The statement has no balance column: compare the worked-out balance with the statement yourself." : diff === 0 ? `Reconciled: ${r.matched.length} entries matched, nothing unexplained.` : `Difference of ${m(diff)} not explained by the open items. Check the period of the statement and entries outside the matching window.`}</p>`;
      const list = (title, items, isBook) => items.length ? `<details><summary>${esc(title)} (${items.length})</summary><table class="grid dense"><tbody>${items.map(x => `<tr><td>${esc(x.date)}</td><td>${esc(isBook ? x.voucher : `${x.reference} ${x.narration}`)}</td><td class="num">${m(Math.abs(x.amount))}</td></tr>`).join("")}</tbody></table></details>` : "";
      body += list("Deposited, not yet credited by the bank", r.deposited_not_cleared, true) + list("Payments not yet presented", r.issued_not_presented, true) + list("Credited by the bank only", r.credited_by_bank_only, false) + list("Debited by the bank only", r.debited_by_bank_only, false);
    }
    card.innerHTML = body;
    const input = card.querySelector("input[type=file]");
    card.querySelector("[role=button]").addEventListener("keydown", e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); input.click(); } });
    input.addEventListener("change", async () => {
      const f = input.files[0]; if (!f) return;
      try {
        const res = await api("POST", `/api/projects/${pid()}/upload?kind=bank&branch=${encodeURIComponent(l.name)}&name=${encodeURIComponent(f.name)}`, undefined, await f.arrayBuffer());
        toast(`${l.name}: ${res.contents} read.`); loadBank();
      } catch (e) { toast(e.message, true); }
    });
    box.appendChild(card);
  }
}

// ---- GST 2B and 26AS ------------------------------------------------------------
function reconTable(r, gst) {
  const m = p => (p < 0 ? `(${rupees(-p)})` : rupees(p));
  const off = r.rows.filter(x => x.difference !== 0).length;
  const head = gst ? ["Ledger in the books", "Supplier on 2B (GSTIN)", "ITC in books", "ITC in 2B", "Difference"] : ["Ledger in the books", "Deductor on 26AS (TAN)", "TDS in books", "TDS in 26AS", "Difference"];
  const rows = r.rows.map(x => `<tr class="${x.difference ? "flag" : ""}"><td>${x.ledger ? esc(x.ledger) : '<span class="muted">not in the books</span>'}${x.matched_by === "close" ? ' <span class="badge warning">name looks alike: check</span>' : ""}</td>
    <td>${x.portal_name ? `${esc(x.portal_name)} <span class="muted small">${esc(x.portal_id || "")}</span>` : `<span class="badge warning">${gst ? "not in 2B: supplier may not have filed" : "not in 26AS"}</span>`}</td>
    <td class="num">${m(x.books_tax)}</td><td class="num">${m(x.portal_tax)}</td><td class="num">${x.difference ? m(x.difference) : "-"}</td></tr>`).join("");
  return `<p class="${off ? "errt" : "okt"} small">${off ? `${off} part${off === 1 ? "y differs" : "ies differ"}.` : "Books agree with the portal for every party."} Books ${m(r.books_total)}, portal ${m(r.portal_total)}.</p>
    <div class="scroll"><table class="grid dense"><thead><tr>${head.map((h, i) => `<th class="${i > 1 ? "num" : ""}">${h}</th>`).join("")}</tr></thead><tbody>${rows}</tbody></table></div>`;
}
async function loadPortal() {
  try {
    const d = await api("GET", `/api/projects/${pid()}/portal`);
    if (d.gst) $("#gstBody").innerHTML = reconTable(d.gst, true);
    if (d.tds) $("#tdsBody").innerHTML = reconTable(d.tds, false);
  } catch (e) { toast(e.message, true); }
}
$$("[data-portal]").forEach(input => {
  input.parentElement.querySelector("[role=button]").addEventListener("keydown", e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); input.click(); } });
  input.addEventListener("change", async () => {
    const f = input.files[0]; if (!f) return;
    try {
      const r = await api("POST", `/api/projects/${pid()}/upload?kind=${input.dataset.portal}&name=${encodeURIComponent(f.name)}`, undefined, await f.arrayBuffer());
      toast(`${f.name}: ${r.contents} read.`); loadPortal();
    } catch (e) { toast(e.message, true); }
  });
});

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
const compactRs = p => { const r = p / 100, a = Math.abs(r); return "₹ " + (a >= 1e7 ? (r / 1e7).toFixed(2) + " Cr" : a >= 1e5 ? (r / 1e5).toFixed(2) + " L" : r.toLocaleString("en-IN", { maximumFractionDigits: 0 })); };
const pct = (c, p) => (p ? (((c - p) / Math.abs(p)) * 100).toFixed(1) + "%" : "");
// ---- dashboard (bento) --------------------------------------------------------
const reduceMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;
// This year against last year as two thin bars (shared scale).
function pair(c, p) {
  const top = Math.max(Math.abs(c || 0), Math.abs(p || 0)) || 1;
  const bar = (v, cls, l) => `<span class="mb-row"><span class="mb-l">${l}</span><span class="mb-t"><i class="${cls}" style="--w:${(Math.abs(v || 0) / top).toFixed(3)}"></i></span><span class="mb-v">${esc(compactRs(v || 0))}</span></span>`;
  return `<div class="mbar">${bar(c, "cy", "This year")}${p == null ? "" : bar(p, "py", "Last year")}</div>`;
}
// "▲ ₹1.20 Cr (+25.3%) on last year": direction only, no judgement (neutral ink).
function vsLy(c, p) {
  if (p == null) return '<span class="kd muted">No last year figures</span>';
  const d = (c || 0) - p;
  if (!d) return '<span class="kd">Same as last year</span>';
  const pc = p ? ` (${d > 0 ? "+" : "-"}${Math.abs((d / Math.abs(p)) * 100).toFixed(1)}%)` : " (new)";
  return `<span class="kd">${d > 0 ? "▲" : "▼"} ${esc(compactRs(Math.abs(d)))}${pc} on last year</span>`;
}
// Facts that call for a decision or an explanation, from this year against last year.
function pointsToLook(a) {
  const c = a.key.cy, p = a.key.py;
  if (!p) return null;
  const gr = k => (p[k] ? ((c[k] - p[k]) / Math.abs(p[k])) * 100 : null);
  const mat = 0.005 * Math.max(Math.abs(c.revenue || 0), Math.abs(c.total_assets || 0));
  const big = k => Math.abs((c[k] || 0) - (p[k] || 0)) >= mat;
  const f = x => `${Math.abs(x).toFixed(1)}%`;
  const out = [];
  const add = (dir, text, go) => out.push({ dir, text, go });
  if (p.profit > 0 && c.profit < 0) add("down", `Profit of ${compactRs(p.profit)} last year turned into a loss of ${compactRs(-c.profit)}.`);
  else if (gr("profit") != null && Math.abs(gr("profit")) >= 25 && big("profit")) add(c.profit > p.profit ? "up" : "down", `Profit ${c.profit > p.profit ? "rose" : "fell"} ${f(gr("profit"))}: ${compactRs(p.profit)} to ${compactRs(c.profit)}.`);
  if (c.revenue && p.revenue) {
    const mc = (c.profit / c.revenue) * 100, mp = (p.profit / p.revenue) * 100;
    if (Math.abs(mc - mp) >= 2) add(mc > mp ? "up" : "down", `Profit margin moved from ${mp.toFixed(1)}% to ${mc.toFixed(1)}% of revenue.`);
  }
  if (gr("revenue") != null && Math.abs(gr("revenue")) >= 25) add(gr("revenue") > 0 ? "up" : "down", `Revenue ${gr("revenue") > 0 ? "rose" : "fell"} ${f(gr("revenue"))}: ${compactRs(p.revenue)} to ${compactRs(c.revenue)}.`);
  const faster = (k, label, tail, go) => {
    const g = gr(k), r = gr("revenue");
    if (g != null && r != null && g - r >= 15 && big(k)) add("up", `${label} grew faster than revenue (${g >= 0 ? "+" : "-"}${f(g)} against ${r >= 0 ? "+" : "-"}${f(r)}): ${tail}`, go);
  };
  faster("receivables", "Debtors", "more of the year's sales is still to be collected. See the ageing.", "analysis");
  faster("inventories", "Stock", "stock held is higher for the level of sales.");
  faster("payables", "Creditors", "more of the year's purchases is still unpaid.");
  faster("employee", "Employee costs", "they take a larger share of revenue.");
  for (const [k, l] of [["borrowings", "Borrowings"], ["finance_costs", "Interest and finance costs"], ["cash_bank", "Cash and bank"]]) {
    const g = gr(k);
    if (g != null && Math.abs(g) >= 25 && big(k)) add(g > 0 ? "up" : "down", `${l} ${g > 0 ? "rose" : "fell"} ${f(g)}: ${compactRs(p[k])} to ${compactRs(c[k])}.`);
    else if (!p[k] && c[k] && big(k)) add("up", `${l} of ${compactRs(c[k])} this year; none last year.`);
  }
  const fl = a.ratios.filter(r => r.needs_explanation).length;
  if (fl) add("flat", `${fl} ratio${fl === 1 ? "" : "s"} moved by more than 25%${state.settings?.entity_type === "company" ? "; Schedule III asks for the reason in the notes" : "; worth explaining the reason"}.`);
  return out;
}
function bento(a, KPIS, delta) {
  const st = state.settings, cy = a.key.cy, py = a.key.py;
  let i = 0;
  const tile = (cls, body) => `<section class="tile ${cls}" style="--i:${i++}">${body}</section>`;
  const fig = (k, label, cls) => tile(`kpi-t ${cls || ""}`, `<p class="tk">${label}</p><p class="tv" data-count="${cy[k] || 0}">${esc(compactRs(cy[k]))}</p>${vsLy(cy[k], py ? py[k] : null)}${pair(cy[k], py ? py[k] : null)}`);
  // Profit, the headline.
  const margin = cy.revenue ? ((cy.profit / cy.revenue) * 100).toFixed(1) + "% of revenue" : "";
  const spent = (cy.revenue || 0) + (cy.other_income || 0) - (cy.profit || 0);
  const spentPy = py ? (py.revenue || 0) + (py.other_income || 0) - (py.profit || 0) : 0;
  const amt = v => (v < 0 ? "(" + esc(compactRs(-v)) + ")" : esc(compactRs(v || 0)));
  const flow = `<li class="hd"><span></span><span>This year</span>${py ? "<span>Last year</span>" : ""}</li>` + [["Revenue", cy.revenue, py && py.revenue], ["Other income", cy.other_income, py && py.other_income], ["Expenses and tax", -spent, -spentPy]]
    .map(([l, v, w]) => `<li><span>${l}</span><span>${amt(v)}</span>${py ? `<span>${amt(w)}</span>` : ""}</li>`).join("");
  let h = tile("hero", `<p class="tk">Profit / (loss) for the year</p><p class="tv big" data-count="${cy.profit || 0}">${esc(compactRs(cy.profit))}</p>${vsLy(cy.profit, py ? py.profit : null)}<p class="muted small">${margin}</p><ul class="flow">${flow}<li class="tot"><span>Profit / (loss)</span><span>${amt(cy.profit)}</span>${py ? `<span>${amt(py.profit)}</span>` : ""}</li></ul>${pair(cy.profit, py ? py.profit : null)}`);
  // Ready for a final copy?
  const pend = a.mapping.filter(m => !m.head || ["suggested", "review"].includes(m.status)).length;
  const items = [
    ["Must-fix problems", a.summary.must_fix, "check"],
    ["Placements to confirm", pend, "map"],
    ["Disclosures to answer", st ? openDisclosures(st) : 0, "disclose"],
    ["Legal items to verify", a.legal && a.legal.statements ? a.legal.statements.applicable - a.legal.statements.verified : 0, "legal"],
  ];
  const done = items.filter(x => !x[1]).length, C = 213.6;
  h += tile("ready", `<p class="tk">Final copy</p><div class="ring-row"><svg class="ring" viewBox="0 0 80 80" aria-hidden="true"><circle cx="40" cy="40" r="34" class="ring-bg"/><circle cx="40" cy="40" r="34" class="ring-fg" style="--off:${(C * (1 - done / items.length)).toFixed(1)}"/></svg><div><p class="tv">${done === items.length ? "Ready to sign" : `${items.length - done} of ${items.length} open`}</p><p class="muted small">${done === items.length ? "Nothing blocks a final copy." : "A draft can be printed now."}</p></div></div><ul class="ready-list">${items.map(([l, n, v]) => `<li class="${n ? "open" : "ok"}"><span>${n ? "●" : "✓"}</span><span>${l}</span><b>${n || ""}</b>${n ? `<button type="button" class="small" data-go="${v}">Open</button>` : ""}</li>`).join("")}</ul>`);
  h += fig("revenue", "Revenue", "sm");
  h += fig("total_assets", "Total assets", "sm");
  for (const [k, l] of [["cash_bank", "Cash and bank"], ["receivables", "Debtors"], ["payables", "Creditors"], ["borrowings", "Borrowings"]]) h += fig(k, l, "q");
  // Decisions: what moved, and the biggest movements line by line.
  const pts = pointsToLook(a);
  h += tile("points", `<p class="tk">Points to look at, against last year</p>` + (pts == null
    ? `<p class="muted">Import last year's trial balance to compare the two years.</p><button type="button" class="small" data-go="import">Import last year</button>`
    : pts.length ? `<ul class="pt-list">${pts.map(x => `<li><span class="pd ${x.dir}">${x.dir === "up" ? "▲" : x.dir === "down" ? "▼" : "●"}</span><span>${esc(x.text)}</span></li>`).join("")}</ul>` : `<p class="tv">No large movements</p><p class="muted small">Nothing moved by 25% or more against last year.</p>`));
  if (py) {
    const br = v => (v < 0 ? `(${compactRs(-v)})` : compactRs(v));
    // Capital-account movement lines (Add: profit, Less: drawings) repeat other figures: left out.
    const mv = (a.compare || []).map(r => ({ ...r, d: r.cy - (r.py || 0) })).filter(r => r.d && !/^(add|less)\b|profit\s*&\s*loss a\/c/i.test(r.label)).sort((x, y) => Math.abs(y.d) - Math.abs(x.d)).slice(0, 8);
    h += tile("moves", `<p class="tk">Biggest movements from last year <span class="muted">(note lines)</span></p><table class="mv"><thead><tr><th>Line</th><th class="num">Last year</th><th class="num">This year</th><th class="num">Change</th></tr></thead><tbody>${mv.map(r => `<tr><td>${esc(r.label)}<span class="muted small"> · Note ${r.note}</span></td><td class="num">${r.py == null ? "-" : esc(br(r.py))}</td><td class="num">${esc(br(r.cy))}</td><td class="num">${r.d > 0 ? "▲" : "▼"} ${esc(compactRs(Math.abs(r.d)))}${r.py ? ` <span class="muted">${Math.abs((r.d / Math.abs(r.py)) * 100).toFixed(0)}%</span>` : ' <span class="muted">new</span>'}</td></tr>`).join("")}</tbody></table><p class="muted small">Full comparison in Detailed tables below.</p>`);
  }
  const ch = a.charts || [];
  const chart = (c, cls) => tile(cls, `<figure class="chart">${c.svg}<figcaption class="sr">${esc(c.title)}</figcaption></figure>`);
  const monthly = ch.filter(c => /month/i.test(c.title)), ageing = ch.filter(c => /^age/i.test(c.title)), other = ch.filter(c => !monthly.includes(c) && !ageing.includes(c));
  for (const c of monthly) h += chart(c, "c-wide");
  // Tax audit items (cases and amounts, from the findings).
  const rows = TAX_ITEMS.map(([code, label, oldSec, newSec]) => {
    const fs = a.findings.filter(f => f.code === code);
    return { code, label, sec: newAct() ? newSec : oldSec, n: fs.length, amt: fs.reduce((s, f) => s + Math.abs(f.amount || 0), 0) };
  });
  const hit = rows.filter(r => r.n);
  h += tile("tax", `<p class="tk">Items for tax audit <span class="muted">(${newAct() ? "Income-tax Act, 2025 · Form 26" : "Income-tax Act, 1961 · Form 3CD"})</span></p>` +
    (hit.length ? `<ul class="tax-list">${hit.map(r => `<li><span>${r.label}${r.sec ? ` <span class="muted">${r.sec}</span>` : ""}</span><b>${r.n}</b><span class="num">₹ ${rupees(r.amt)}</span><button type="button" class="small" data-code="${r.code}" data-label="${esc(r.label)}">See list</button></li>`).join("")}</ul><p class="muted small">Loans count only principal; interest and TDS are kept apart.</p>` : `<p class="tv">None found</p><p class="muted small">No cash or loan items above the limits.</p>`));
  for (const c of other) h += chart(c, "c-wide");
  for (const c of ageing) h += chart(c, "c-half");
  if (a.ratios.length) {
    const f = (v, u) => (v == null ? "-" : v.toFixed(2) + (u === "%" ? "%" : ""));
    const flagged = a.ratios.filter(r => r.needs_explanation).length;
    h += tile("ratios", `<p class="tk">Ratios, this year and last year <span class="muted">(${flagged ? `${flagged} changed by more than 25%: marked explain` : "none changed by more than 25%"})</span></p><ul class="ratio-list">${a.ratios.map(r => `<li class="${r.needs_explanation ? "flag" : ""}" title="${esc([r.numerator + " / " + r.denominator, ...(r.notes || [])].join(" · "))}"><span>${esc(r.name)}${r.review ? ' <span class="muted small">· check figures</span>' : ""}</span><b>${f(r.cy, r.unit)}</b><span class="muted">${f(r.py, r.unit)}</span></li>`).join("")}</ul>`);
  }
  return `<div class="bento">${h}</div>`;
}
// Entrance: tiles rise in turn, bars grow, lines draw, figures count up.
// Played once per set of results, never on keyboard navigation repeats.
function playBento(box, a) {
  const g = box.querySelector(".bento");
  if (!g || state.bentoPlayed === a || reduceMotion()) { state.bentoPlayed = a; return; }
  state.bentoPlayed = a;
  g.classList.add("play");
  for (const el of $$("[data-count]", g)) {
    const to = Number(el.dataset.count), t0 = performance.now(), dur = 600;
    const tick = t => {
      const k = Math.min(1, (t - t0) / dur), e = 1 - Math.pow(1 - k, 3);
      el.textContent = compactRs(Math.round(to * e));
      if (k < 1) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }
  setTimeout(() => g.classList.remove("play"), 1600);
}
function renderAnalysis() {
  const a = state.analysis, box = $("#anBody");
  if (!a) { box.innerHTML = '<p class="empty">Run the checks first.</p>'; return; }
  const cy = a.key.cy, py = a.key.py;
  const KEY = [["revenue", "Revenue from operations"], ["other_income", "Other income"], ["profit", "Profit / (loss) for the year"], ["employee", "Employee costs"], ["finance_costs", "Interest and finance costs"], ["depreciation", "Depreciation"],
    ["total_assets", "Total assets"], ["ppe", "Fixed assets (net)"], ["inventories", "Stock"], ["receivables", "Debtors"], ["cash_bank", "Cash and bank"], ["payables", "Creditors"], ["borrowings", "Borrowings"]];
  const money = p => (p < 0 ? `(${rupees(-p)})` : rupees(p));
  const KPIS = [["revenue", "Revenue"], ["profit", "Profit / (loss)"], ["total_assets", "Total assets"], ["cash_bank", "Cash and bank"], ["receivables", "Debtors"], ["payables", "Creditors"]];
  const delta = (c, p) => {
    if (!py || !p) return '<span class="kd muted">no last year</span>';
    const ch = ((c - p) / Math.abs(p)) * 100;
    return `<span class="kd ${ch >= 0 ? "up" : "down"}">${ch >= 0 ? "▲" : "▼"} ${Math.abs(ch).toFixed(1)}% on last year</span>`;
  };
  let h = bento(a, KPIS, delta);
  h += `<details class="more"><summary>Detailed tables: key figures, every note line against last year, loans, ratios and ageing</summary><div class="two"><div class="box"><h2>Key figures (₹)</h2><table class="grid dense"><thead><tr><th>Item</th><th class="num">This year</th>${py ? '<th class="num">Last year</th><th class="num">Change</th>' : ""}</tr></thead><tbody>` +
    KEY.map(([k, l]) => `<tr><td>${l}</td><td class="num">${money(cy[k])}</td>${py ? `<td class="num">${money(py[k])}</td><td class="num">${pct(cy[k], py[k])}</td>` : ""}</tr>`).join("") + "</tbody></table></div></div>";
  // Loans
  const loans = a.loans || [];
  h += `<div class="box"><h2>Loans and deposits taken (₹)</h2>` + (loans.length ? `<div class="scroll"><table class="grid dense"><thead><tr><th>Lender</th><th class="num">Opening</th><th class="num">Taken by bank</th><th class="num">Taken in cash</th><th class="num">By journal</th><th class="num">Interest</th><th class="num">Repaid by bank</th><th class="num">Repaid in cash</th><th class="num">Closing</th><th class="num">Highest balance</th></tr></thead><tbody>` +
    loans.map(l => `<tr><td>${esc(l.ledger)}${l.exempt ? ' <span class="badge">bank / exempt</span>' : ""}</td>${[l.opening, l.accepted_bank, l.accepted_cash, l.accepted_journal, l.interest_credited, l.repaid_bank, l.repaid_cash, l.closing, l.max_outstanding].map(v => `<td class="num ${v && (v === l.accepted_cash || v === l.repaid_cash) && !l.exempt ? "hl" : ""}">${v ? rupees(v) : "-"}</td>`).join("")}</tr>`).join("") + "</tbody></table></div>" : '<p class="muted">No loan ledgers found (a day book is needed for movements).</p>') + "</div>";
  // Ratios
  if (a.ratios.length) {
    const f = v => (v == null ? "-" : v.toFixed(2));
    h += `<div class="box"><h2>Ratios</h2><table class="grid dense"><thead><tr><th>Ratio</th><th>Formula</th><th class="num">This year</th><th class="num">Last year</th><th class="num">Change</th></tr></thead><tbody>` +
      a.ratios.map(r => `<tr class="${r.needs_explanation ? "flag" : ""}"><td>${esc(r.name)}${(r.notes || []).length ? `<div class="muted small">${r.notes.map(esc).join("<br>")}</div>` : ""}</td><td class="muted small">${esc(r.numerator)} / ${esc(r.denominator)}${r.num_cy != null ? `<div>This year: ₹${rupees(Math.round(r.num_cy * 100))} / ₹${rupees(Math.round((r.den_cy || 0) * 100))}</div>` : ""}${r.num_py != null ? `<div>Last year: ₹${rupees(Math.round(r.num_py * 100))} / ₹${rupees(Math.round((r.den_py || 0) * 100))}</div>` : ""}</td><td class="num">${f(r.cy)}${r.unit === "%" ? "%" : ""}</td><td class="num">${f(r.py)}${r.unit === "%" && r.py != null ? "%" : ""}</td><td class="num">${r.variance_pct == null ? "-" : r.variance_pct.toFixed(1) + "%"}${r.needs_explanation ? ' <span class="badge warning">over 25%</span>' : ""}</td></tr>`).join("") + "</tbody></table></div>";
  }
  // Ageing
  h += `<p class="muted small">Ageing is from the bill (transaction) date, payments applied to the oldest bills first. Due dates are not in the imported books, so this is not ageing from the due date.</p>`;
  for (const [k, title] of [["receivables", "Ageing of debtors (₹)"], ["payables", "Ageing of creditors (₹)"]]) {
    const ag = a.ageing[k];
    if (!ag || !ag.rows.length) continue;
    h += `<div class="box"><h2>${title}</h2><div class="scroll"><table class="grid dense"><thead><tr><th>Category</th>${ag.bucket_labels.map(b => `<th class="num">${esc(b)}</th>`).join("")}</tr></thead><tbody>` +
      ag.rows.map(r => `<tr><td>${esc(r.category)}</td>${r.buckets.map(v => `<td class="num">${v ? rupees(Math.abs(v)) : "-"}</td>`).join("")}</tr>`).join("") + "</tbody></table></div></div>";
  }
  if (py && (a.compare || []).length) {
    let last = null;
    h += `<div class="box"><h2>This year against last year, note by note (₹)</h2><div class="scroll"><table class="grid dense"><thead><tr><th>Line</th><th class="num">This year</th><th class="num">Last year</th><th class="num">Change</th><th class="num">%</th></tr></thead><tbody>` +
      a.compare.map(r => { const d = r.cy - (r.py || 0); const head = r.note !== last ? `<tr class="sub"><td colspan="5"><b>Note ${r.note}. ${esc(r.title)}</b></td></tr>` : ""; last = r.note;
        return head + `<tr><td>${esc(r.label)}</td><td class="num">${money(r.cy)}</td><td class="num">${r.py == null ? "-" : money(r.py)}</td><td class="num">${d ? money(d) : "-"}</td><td class="num">${r.py ? pct(r.cy, r.py) : r.cy ? "new" : ""}</td></tr>`; }).join("") + "</tbody></table></div></div>";
  }
  h += "</details>";
  box.innerHTML = h;
  playBento(box, a);
  $$("[data-go]", box).forEach(b => b.addEventListener("click", () => show(b.dataset.go)));
  $$("[data-code]", box).forEach(b => b.addEventListener("click", () => {
    state.codes = [b.dataset.code]; state.codesLabel = b.dataset.label; show("check"); renderCheck();
  }));
}
$("#anRerun").addEventListener("click", runChecks);
$("#anWorkbook").addEventListener("click", async () => {
  const b = $("#anWorkbook"); b.disabled = true; b.textContent = "Saving...";
  try {
    const r = await api("POST", `/api/projects/${pid()}/export`, { mode: "draft", folder: "", files: { pdf: false, xlsx: false, docx: false, html: false, auditor_workbook: true, tax_audit: true, json: false } });
    state.lastExport = r.dir; $("#openExport").disabled = false;
    toast("Auditor workbooks saved.");
    if (await confirmBox("Auditor workbook and tax audit helper saved", r.dir, "Open folder")) openFolder(r.dir);
  } catch (e) { toast(e.message, true); }
  b.disabled = false; b.textContent = "Save auditor workbook";
});

// ---- notes and disclosures --------------------------------------------------------
const MONEY = v => (v ? (v / 100).toFixed(2) : "");
const FIELDS = {
  holders_5pct: [["name", "Name of shareholder", "text"], ["shares", "Shares this year", "int"], ["py_shares", "Shares last year", "int"]],
  promoters: [["name", "Promoter name", "text"], ["shares", "Shares this year", "int"], ["py_shares", "Shares last year", "int"]],
  contingent_liabilities: [["nature", "Nature", "text"], ["cy", "This year (₹)", "money"], ["py", "Last year (₹)", "money"]],
  commitments: [["nature", "Nature", "text"], ["cy", "This year (₹)", "money"], ["py", "Last year (₹)", "money"]],
  related_parties: [["name", "Name", "text"], ["relationship", "Relationship", "text", "rpRel"]],
  related_transactions: [["party", "Related party", "text", "rpParties"], ["nature", "Nature", "text", "rpNature"], ["cy", "This year (₹)", "money"], ["py", "Last year (₹)", "money"]],
  extra_policies: [["title", "Title", "text"], ["text", "Wording", "area"]],
  notes: [["title", "Title of the note", "text"], ["text", "Text", "area"]],
  share_classes: [["name", "Class (e.g. Equity shares of ₹10 each)", "text"], ["face_value", "Face value per share (₹)", "money"], ["paid_per_share", "Paid up per share (₹, if partly paid)", "money"],
    ["authorised", "Authorised: this year", "int"], ["py_authorised", "Authorised: last year", "int"], ["issued", "Issued: this year", "int"], ["py_issued", "Issued: last year", "int"],
    ["subscribed", "Subscribed and paid up: this year", "int"], ["py_subscribed", "Subscribed and paid up: last year", "int"], ["added", "Shares issued during the year", "int"], ["reduced", "Shares bought back / reduced", "int"], ["rights", "Rights, preferences and restrictions", "area"]],
};
const MSME_ROWS = [["interest_due_unpaid", "Interest due and remaining unpaid"], ["interest_paid_s16", "Interest paid under section 16"], ["paid_beyond_appointed_day", "Payments made beyond the appointed day"],
  ["interest_due_for_delay", "Interest due for the period of delay (paid late)"], ["interest_accrued_unpaid", "Interest accrued and remaining unpaid at year end"], ["further_interest", "Further interest due in succeeding years"]];
function fieldHtml([k, label, type, list], v) {
  const val = type === "money" ? MONEY(v) : v ?? "";
  const input = type === "area" ? `<textarea data-k="${k}" rows="3">${esc(val)}</textarea>`
    : `<input data-k="${k}" data-t="${type}" value="${esc(val)}" ${type === "text" ? "" : 'inputmode="decimal" class="num"'} ${list ? `list="${list}"` : ""}>`;
  return `<div class="field ${type === "area" ? "wide" : ""}"><label>${esc(label)}</label>${input}</div>`;
}
function addItem(key, item = {}) {
  const box = key === "share_classes" ? $("#dShareClasses") : $(`[data-list="${key}"]`);
  const d = document.createElement("div");
  d.className = "item-row" + (key === "share_classes" || FIELDS[key].some(f => f[2] === "area") ? " card" : "");
  d.dataset.key = key;
  d.innerHTML = `<div class="item-fields">${FIELDS[key].map(f => fieldHtml(f, item[f[0]])).join("")}</div><button type="button" class="small danger-ghost" aria-label="Remove">Remove</button>`;
  d.querySelector("button").addEventListener("click", () => { d.remove(); emptyHints(); });
  box.appendChild(d); emptyHints();
}
function emptyHints() {
  for (const box of [...$$("[data-list]"), $("#dShareClasses")]) {
    let hint = box.querySelector(".list-empty");
    const has = box.querySelector(".item-row");
    if (!has && !hint) { hint = document.createElement("p"); hint.className = "list-empty muted small"; hint.textContent = "Nothing entered."; box.appendChild(hint); }
    if (has && hint) hint.remove();
  }
}
function readItems(key) {
  return $$(`.item-row[data-key="${key}"]`).map(row => {
    const o = {};
    for (const [k, , type] of FIELDS[key]) {
      const el = row.querySelector(`[data-k="${k}"]`); const v = el.value.trim();
      o[k] = type === "money" ? paise(v) : type === "int" ? (parseInt(v.replace(/,/g, ""), 10) || 0) : v;
      if (type === "money" && isNaN(o[k])) throw new Error(`"${v}" is not an amount.`);
    }
    return o;
  }).filter(o => Object.values(o).some(v => v !== "" && v !== 0));
}
// Every required section has three states (TRUTH-MODEL §2): never assume nil.
const ANSWER_TEXT = {
  share_capital: ["Not answered yet", null, "Particulars entered below"],
  contingent: ["Not answered yet", "None: there are no contingent liabilities or commitments", "Details entered below"],
  related_parties: ["Not answered yet", "None: no related parties with transactions or control", "Details entered below"],
  msme: ["Not answered yet", "None: no amounts due to micro or small enterprises", "Checked: MSME suppliers are tagged on Map ledgers; interest below"],
};
function renderAnswers(d) {
  for (const bar of $$("[data-answer]")) {
    const sec = bar.dataset.answer, t = ANSWER_TEXT[sec];
    const cur = (d.answers || {})[sec] || "";
    const carried = (d.pending_review || []).includes(sec);
    bar.innerHTML = `<p class="ab-q">${carried ? "Carried from last year: check the particulars and choose your answer again" : "Your answer for this section"}</p>` + [["", t[0]], ["nil", t[1]], ["provided", t[2]]].filter(x => x[1])
      .map(([v, l]) => `<label class="radio"><input type="radio" name="ans-${sec}" value="${v}" ${cur === v ? "checked" : ""}> ${esc(l)}</label>`).join("");
    bar.classList.toggle("unanswered", !cur);
    $$("input", bar).forEach(r => r.addEventListener("change", () => bar.classList.toggle("unanswered", !r.value)));
  }
}
function readAnswers() {
  const out = {};
  for (const bar of $$("[data-answer]")) { const v = ($$("input", bar).find(r => r.checked) || {}).value; if (v) out[bar.dataset.answer] = v; }
  return out;
}
let policyStd = [];
async function loadDisclosures() {
  const st = await reloadSettings();
  const d = st.options.disclosures || {};
  $$(".co-only").forEach(b => (b.hidden = st.entity_type !== "company"));
  const first = st.entity_type === "company" ? "capital" : "contingent";
  showTab(state.dTab && !(state.dTab === "capital" && st.entity_type !== "company") ? state.dTab : first);
  $("#dShareClasses").innerHTML = ""; $$("[data-list]").forEach(b => (b.innerHTML = ""));
  for (const key of Object.keys(FIELDS)) for (const it of d[key] || []) addItem(key, it);
  if (st.entity_type === "company" && !(d.share_classes || []).length) addItem("share_classes", { name: "Equity shares of ₹10 each", face_value: 1000 });
  emptyHints();
  renderAnswers(d);
  const m = d.msme || {};
  $("#dMsme tbody").innerHTML = MSME_ROWS.map(([k, l]) => `<tr><td>${l}</td><td class="num"><input class="num" data-m="${k}" data-i="0" value="${MONEY((m[k] || [0, 0])[0])}" inputmode="decimal" aria-label="${l}, this year"></td><td class="num"><input class="num" data-m="${k}" data-i="1" value="${MONEY((m[k] || [0, 0])[1])}" inputmode="decimal" aria-label="${l}, last year"></td></tr>`).join("");
  try { policyStd = await api("GET", `/api/projects/${pid()}/policies`); } catch (e) { policyStd = []; }
  const own = d.policy_text || {};
  $("#dPolicies").innerHTML = policyStd.map((p, i) => `<div class="policy"><div class="title-row"><h2>${i + 1}. ${esc(p.title)}</h2><span>${own[p.title] ? '<span class="badge okb">Your wording</span> ' : ""}<button type="button" class="small" data-reset="${i}">Use standard wording</button></span></div><textarea rows="4" data-policy="${i}">${esc(own[p.title] || p.text)}</textarea></div>`).join("");
  $$("[data-reset]").forEach(b => b.addEventListener("click", () => { $(`[data-policy="${b.dataset.reset}"]`).value = policyStd[+b.dataset.reset].text; }));
  $("#rpParties").innerHTML = (d.related_parties || []).map(p => `<option value="${esc(p.name)}">`).join("");
  setStatus($("#dStatus"), "");
}
function showTab(t) {
  state.dTab = t;
  $$(".tabs [data-tab]").forEach(b => { b.classList.toggle("on", b.dataset.tab === t); b.setAttribute("aria-selected", String(b.dataset.tab === t)); });
  $$("[data-tabpanel]").forEach(p => (p.hidden = p.dataset.tabpanel !== t));
}
$$(".tabs [data-tab]").forEach(b => b.addEventListener("click", () => showTab(b.dataset.tab)));
$$("[data-add]").forEach(b => b.addEventListener("click", () => addItem(b.dataset.add)));
$("#dSave").addEventListener("click", async () => {
  try {
    const d = {};
    for (const key of Object.keys(FIELDS)) d[key] = readItems(key);
    d.msme = {};
    for (const [k] of MSME_ROWS) {
      const v = [0, 1].map(i => paise($(`[data-m="${k}"][data-i="${i}"]`).value));
      if (v.some(isNaN)) throw new Error("MSME interest: use numbers only.");
      d.msme[k] = v;
    }
    d.answers = readAnswers();
    // Sections carried from last year stay open until you choose an answer.
    d.pending_review = ((state.settings.options.disclosures || {}).pending_review || []).filter(sec => !d.answers[sec]);
    // Particulars entered count as an answer; "details below" with nothing entered does not.
    const LIST_OF = { share_capital: ["share_classes"], contingent: ["contingent_liabilities", "commitments"], related_parties: ["related_parties"] };
    for (const [sec, keys] of Object.entries(LIST_OF)) {
      const has = keys.some(k => d[k].length);
      if (has && !d.pending_review.includes(sec)) d.answers[sec] = "provided";
      else if (d.answers[sec] === "provided") throw new Error(`${$(`[data-answer="${sec}"]`).closest("[data-tabpanel]").dataset.tabpanel.replace(/^./, c => c.toUpperCase())}: you chose "details below" but nothing is entered. Enter the details or choose "None".`);
    }
    d.policy_text = {};
    policyStd.forEach((p, i) => { const t = $(`[data-policy="${i}"]`).value.trim(); if (t && t !== p.text.trim()) d.policy_text[p.title] = t; });
    const options = { ...state.settings.options, disclosures: d };
    await api("POST", `/api/projects/${pid()}/settings`, { options });
    state.settings.options = options; state.analysis = null;
    setStatus($("#dStatus"), "Saved."); toast("Disclosures saved.");
    $("#rpParties").innerHTML = d.related_parties.map(p => `<option value="${esc(p.name)}">`).join("");
    renderNext();
  } catch (e) { setStatus($("#dStatus"), e.message, true); toast(e.message, true); }
});

// ---- 6. presentation ----------------------------------------------------------
const isCo = () => state.settings?.entity_type === "company";
const toggleOn = v => v === "on" || (v === "auto" && isCo());
async function loadPresent() {
  const st = await reloadSettings();
  fillOpts(st.options);
  fillPeriod(st.period);
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
// Period: full financial year or chosen dates.
const iso = d => d.toISOString().slice(0, 10);
function fyStartYear() { return Number(state.settings.fy.slice(0, 4)); }
function fillPeriod(p) {
  $$('input[name=per]').forEach(r => (r.checked = r.value === (p ? "other" : "fy")));
  $("#perBox").hidden = !p;
  $("#perStart").value = p ? p.start : ""; $("#perEnd").value = p ? p.end : ""; $("#perCmp").value = p && p.comparative ? p.comparative : "";
}
function readPeriod() {
  if (($$('input[name=per]').find(r => r.checked) || {}).value !== "other") return null;
  const start = $("#perStart").value, end = $("#perEnd").value, cmp = $("#perCmp").value;
  if (!start || !end) throw new Error("Choose the start and end dates of the period.");
  return { start, end, comparative: cmp || null };
}
$$('input[name=per]').forEach(r => r.addEventListener("change", () => { $("#perBox").hidden = r.value !== "other" || !r.checked; }));
$$("[data-per]").forEach(b => b.addEventListener("click", () => {
  const y = fyStartYear(), U = (yy, m, d) => iso(new Date(Date.UTC(yy, m, d)));
  const map = { q1: [U(y, 3, 1), U(y, 5, 30)], q2: [U(y, 6, 1), U(y, 8, 30)], q3: [U(y, 9, 1), U(y, 11, 31)], q4: [U(y + 1, 0, 1), U(y + 1, 2, 31)], h1: [U(y, 3, 1), U(y, 8, 30)], cal: [U(y, 0, 1), U(y, 11, 31)] };
  const [a, z] = map[b.dataset.per];
  $("#perStart").value = a; $("#perEnd").value = z;
  $("#perCmp").value = b.dataset.per === "cal" ? U(y - 1, 11, 31) : U(y, 2, 31);
}));
function fillOpts(o) {
  $("#oUnit").value = o.unit; $("#oDec").value = String(o.decimals);
  $$('input[name=layout]').forEach(r => (r.checked = r.value === o.layout));
  $("#oPy").checked = o.show_previous_year; $("#oNil").checked = o.hide_nil_lines; $("#oRel").checked = o.reletter;
  $("#oCover").checked = o.cover_page; $("#oPol").checked = o.accounting_policies; $("#oAge").checked = o.ageing; $("#oParty").checked = o.party_wise_details;
  $("#oPpe").checked = o.ppe_schedule; $("#oItDep").checked = o.tax_depreciation_annexure; $("#oCharts").checked = !!o.charts_annexure;
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
    ppe_schedule: $("#oPpe").checked, tax_depreciation_annexure: $("#oItDep").checked, charts_annexure: $("#oCharts").checked,
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
    const period = readPeriod();
    await api("POST", `/api/projects/${pid()}/settings`, { options, profit_sharing, period });
    state.settings.period = period;
    state.settings.options = options; state.settings.profit_sharing = profit_sharing; state.analysis = null;
    setStatus($("#oStatus"), "Saved."); toast("Saved."); refreshPreview(); renderNext();
  } catch (err) { setStatus($("#oStatus"), err.message, true); }
}
$("#opts").addEventListener("submit", e => { e.preventDefault(); saveOpts(readOpts()); });
$("#resetOpts").addEventListener("click", async () => {
  if (!(await confirmBox("Reset to the standard choices?", "Units, table style and what to print go back to the usual settings. Address lines and ratio reasons are kept.", "Reset"))) return;
  const o = state.settings.options;
  const std = { ...o, unit: "rupees", decimals: 2, layout: "boxed", show_previous_year: true, hide_nil_lines: true, reletter: true, cover_page: true,
    accounting_policies: true, cash_flow: "auto", ratios: "auto", ageing: true, ppe_schedule: true, tax_depreciation_annexure: true, charts_annexure: false, party_wise_details: false, hidden_sections: [] };
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
  const a = state.analysis;
  let legal = null;
  try { legal = await api("GET", `/api/projects/${pid()}/legal?tax_audit=${$('[data-file="tax_audit"]').checked ? 1 : 0}`); } catch {}
  const reasons = a ? [...(a.summary.must_fix ? [`${a.summary.must_fix} "Must fix" item(s) on the Check screen`] : []), ...(a.blockers || [])] : [];
  if (legal && !legal.ready) reasons.push(`Legal content: ${legal.verified} of ${legal.applicable} items verified${legal.stale ? ` (${legal.stale} changed since verification)` : ""}. Record verifications in the Legal verification register.`);
  const blocked = reasons.length > 0;
  $("#modeFinal").disabled = blocked;
  if (blocked) $$('input[name=mode]').forEach(r => (r.checked = r.value === "draft"));
  $("#finalWhy").hidden = !blocked;
  $("#finalWhy").innerHTML = blocked ? `<p class="k">Final copy is locked until these are done. A draft can be exported now.</p><ul>${reasons.map(r => `<li>${esc(r)}</li>`).join("")}</ul>${legal && !legal.ready ? '<button type="button" class="small" id="goLegal">Open Legal verification register</button>' : ""}` : "";
  $("#goLegal")?.addEventListener("click", () => show("legal"));
  setStatus($("#eStatus"), "");
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

// ---- legal verification register --------------------------------------------
state.lgFilter = "open";
async function loadLegal() {
  if (!pid()) return;
  const r = await api("GET", `/api/projects/${pid()}/legal?tax_audit=${$("#lgTax").checked ? 1 : 0}`);
  state.legal = r;
  $("#lgTiles").innerHTML = `
    <div class="tile ${r.ready ? "ok" : "warning"}"><div class="k">Final copy</div><div class="v sm">${r.ready ? "Legal content verified" : "Locked: legal content not verified"}</div></div>
    <div class="tile"><div class="k">Items for this client year</div><div class="v">${r.applicable}</div></div>
    <div class="tile"><div class="k">Verified</div><div class="v">${r.verified}</div></div>
    <div class="tile ${r.stale ? "warning" : ""}"><div class="k">Changed since verified</div><div class="v">${r.stale}</div></div>
    <div class="tile ${r.pending ? "warning" : ""}"><div class="k">Not verified</div><div class="v">${r.pending}</div></div>`;
  renderLegal();
}
function renderLegal() {
  const r = state.legal; if (!r) return;
  const f = state.lgFilter;
  const order = [...new Set(r.items.map(x => x.group))];
  const shown = r.items.filter(x => f === "all" || (f === "open" ? x.status === "pending" : x.status === f))
    .map((x, i) => [x, i]).sort((p, q) => order.indexOf(p[0].group) - order.indexOf(q[0].group) || p[1] - q[1]).map(p => p[0]);
  const tb = $("#lgTable tbody"); tb.innerHTML = "";
  let group = null;
  for (const x of shown) {
    if (x.group !== group) { group = x.group; const g = document.createElement("tr"); g.className = "sub"; g.innerHTML = `<td colspan="5"><b>${esc(group)}</b></td>`; tb.appendChild(g); }
    const tr = document.createElement("tr");
    const v = x.verification;
    tr.innerHTML = `<td><input type="checkbox" data-id="${esc(x.id)}" aria-label="Select"></td>
      <td><button type="button" class="linkish">${esc(x.title)}</button><div class="muted small">${esc(x.id)}</div><pre class="lg-content" hidden>${esc(x.content || "(no content)")}</pre></td>
      <td class="muted small">${esc(x.shipped_status || "")}</td>
      <td><span class="badge ${x.status === "verified" ? "okb" : x.status === "stale" ? "blocker" : "warning"}">${x.status === "verified" ? "Verified" : x.status === "stale" ? "Changed since verified" : "Not verified"}</span></td>
      <td class="small">${v ? `${esc(v.verified_by)}, ${esc(v.verified_on)}<div class="muted">${esc(v.document_title)} · ${esc(v.provision)} · ${esc(v.official_source)}</div>` : '<span class="muted">-</span>'}</td>`;
    tr.querySelector(".linkish").addEventListener("click", () => { const c = tr.querySelector(".lg-content"); c.hidden = !c.hidden; });
    tb.appendChild(tr);
  }
  if (!shown.length) tb.innerHTML = `<tr><td colspan="5" class="empty">Nothing in this filter.</td></tr>`;
  $$("#lgTable input[data-id]").forEach(c => c.addEventListener("change", lgCount));
  $("#lgAll").checked = false; lgCount();
}
function lgSelected() { return $$("#lgTable input[data-id]:checked").map(c => c.dataset.id); }
function lgCount() { $("#lgSel").textContent = lgSelected().length; }
$("#lgAll").addEventListener("change", () => { $$("#lgTable input[data-id]").forEach(c => (c.checked = $("#lgAll").checked)); lgCount(); });
$$("[data-lf]").forEach(b => b.addEventListener("click", () => { state.lgFilter = b.dataset.lf; $$("[data-lf]").forEach(x => x.classList.toggle("on", x === b)); renderLegal(); }));
$("#lgTax").addEventListener("change", loadLegal);
$("#lgBack").addEventListener("click", () => show("export"));
$("#openLegal").addEventListener("click", () => { $("#helpMenu").hidePopover?.(); if (pid()) show("legal"); else toast("Open a client year first.", true); });
$("#lgForm").addEventListener("submit", async e => {
  e.preventDefault();
  const ids = lgSelected();
  if (!ids.length) return setStatus($("#lgStatus"), "Select at least one item.", true);
  const body = { item_ids: ids, authority: $("#lgAuth").value.trim(), document_title: $("#lgDoc").value.trim(), provision: $("#lgProv").value.trim(), official_source: $("#lgSrc").value.trim(),
    effective_from: $("#lgFrom").value, effective_until: $("#lgUntil").value, verified_on: $("#lgOn").value, verified_by: $("#lgBy").value.trim(), note: $("#lgNote").value.trim() };
  setStatus($("#lgStatus"), "Saving...");
  try { state.legal = await api("POST", `/api/projects/${pid()}/legal/verify`, body); state.analysis = null; setStatus($("#lgStatus"), `Recorded for ${ids.length} item(s).`); loadLegal(); }
  catch (err) { setStatus($("#lgStatus"), err.message, true); }
});
$("#lgWithdraw").addEventListener("click", async () => {
  const ids = lgSelected();
  if (!ids.length) return setStatus($("#lgStatus"), "Select at least one item.", true);
  if (!$("#lgBy").value.trim()) return setStatus($("#lgStatus"), "Fill 'Verified by' with who is withdrawing.", true);
  try { await api("POST", `/api/projects/${pid()}/legal/withdraw`, { item_ids: ids, verified_by: $("#lgBy").value.trim(), note: $("#lgNote").value.trim() }); state.analysis = null; setStatus($("#lgStatus"), `Withdrawn for ${ids.length} item(s).`); loadLegal(); }
  catch (err) { setStatus($("#lgStatus"), err.message, true); }
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
    case "checks_run": return `Must fix ${d.must_fix}${d.review != null ? `, Review ${d.review}` : ""}, Check ${d.check}, Notes ${d.notes} (rules ${d.rules_version})`;
    case "mapping_changed": {
      const lbl = id => (state.heads.find(h => h.id === id) || {}).label || id;
      return `${d.ledger}: ${d.from ? lbl(d.from) : "automatic"} to ${d.to ? lbl(d.to) : "automatic"}${d.ai_suggested ? " (AI suggestion accepted)" : ""}`;
    }
    case "mapping_confirmed": return `${d.count} placement(s) confirmed: ${(d.ledgers || []).slice(0, 6).map(x => x.ledger).join(", ")}${(d.ledgers || []).length > 6 ? ", …" : ""}`;
    case "rules_pinned": return `Pinned to rule pack ${d.rules_version} and format ${d.format_pack} (${d.format_status})`;
    case "rules_migrated": return `Moved from rule pack ${d.rules_from} to ${d.rules_to}; ${(d.rule_changes || []).length} rule(s) changed${d.format_changed ? ", format changed" : ""}`;
    case "rolled_forward": return `Started from FY ${d.from_fy}: ${d.previous_year_ledgers} ledgers as last year's figures (${d.adjustments_included} adjustment(s) included), ${d.fixed_assets} fixed assets carried forward`;
    case "next_year_started": return `Next year FY ${d.fy} started from this year`;
    case "legal_verification_recorded": return `Legal verification recorded by ${d.verified_by} on ${d.verified_on} for ${(d.items || []).length} item(s) (source: ${d.official_source})`;
    case "legal_verification_withdrawn": return `Legal verification withdrawn by ${d.verified_by} for ${(d.items || []).length} item(s)`;
    case "branch_added": return `Branch ${d.branch} added`;
    case "branch_removed": return `Branch ${d.branch} removed`;
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
  c.textContent = r.status.intact ? `Audit trail intact: ${r.status.events} events, each sealed with the fingerprint of the one before. Any change or removal in between would show here.` : `Audit trail damaged: ${r.status.problem}.`;
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
