// LedgerCraft app (no framework, works offline).
"use strict";

const TOKEN = document.querySelector('meta[name="lc-token"]').content;
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
const state = { id: null, settings: null, analysis: null, heads: [], filter: "all", status: null };

async function api(method, path, body, raw) {
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

// ---- navigation ------------------------------------------------------------
function show(view) {
  $$(".steps button").forEach(b => b.classList.toggle("active", b.dataset.view === view));
  $$("section[data-panel]").forEach(s => (s.hidden = s.dataset.panel !== view));
  if (view === "present") loadPresent();
  if (view === "export") loadSign();
  if (view === "audit") loadAudit();
  if (view === "map" && !state.analysis) runChecks();
  window.scrollTo(0, 0);
}
$$(".steps button").forEach(b => b.addEventListener("click", () => show(b.dataset.view)));
$$("[data-go]").forEach(b => b.addEventListener("click", () => show(b.dataset.go)));

// ---- status / AI pill ----------------------------------------------------
async function refreshStatus() {
  try {
    const s = await api("GET", "/api/status");
    state.status = s;
    $("#dataDir").textContent = "Data folder: " + s.data_dir;
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
  if (!state.id) return toast("Open a project first.", true);
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

// ---- 1. projects ----------------------------------------------------------
function currentFy() {
  const d = new Date(); const y = d.getMonth() >= 3 ? d.getFullYear() : d.getFullYear() - 1;
  return `${y}-${String((y + 1) % 100).padStart(2, "0")}`;
}
$("#npFy").value = currentFy();
const TYPE_LABEL = { firm: "Partnership firm", llp: "LLP", company: "Company", proprietor: "Proprietorship", huf: "HUF", aop: "AOP", boi: "BOI" };

async function loadProjects() {
  const list = await api("GET", "/api/projects");
  const tb = $("#projectList tbody");
  tb.innerHTML = list.length ? "" : `<tr><td colspan="5" class="muted">No clients yet. Create the first one above.</td></tr>`;
  for (const p of list) {
    const tr = document.createElement("tr");
    tr.innerHTML = `<td>${esc(p.entity_name)}</td><td>${esc(TYPE_LABEL[p.entity_type] || p.entity_type)}</td><td>${esc(p.fy)}</td><td class="muted">${esc((p.modified || "").slice(0, 16).replace("T", " "))}</td><td><button class="small">Open</button></td>`;
    tr.querySelector("button").addEventListener("click", () => openProject(p.id));
    tb.appendChild(tr);
  }
}
$("#newProject").addEventListener("submit", async e => {
  e.preventDefault();
  setStatus($("#npError"), "");
  try {
    const r = await api("POST", "/api/projects", { name: $("#npName").value, entity_type: $("#npType").value, fy: $("#npFy").value });
    $("#npName").value = "";
    await loadProjects();
    openProject(r.id);
  } catch (err) { setStatus($("#npError"), err.message, true); }
});

async function openProject(id) {
  const p = await api("GET", `/api/projects/${encodeURIComponent(id)}`);
  state.id = id; state.settings = p.settings; state.analysis = null;
  $("#projectName").textContent = `${p.settings.entity_name}  |  ${TYPE_LABEL[p.settings.entity_type] || ""}  |  FY ${p.settings.fy}`;
  $$(".steps button").forEach(b => (b.disabled = false));
  renderFiles();
  $("#depBasis").value = p.settings.depreciation_basis;
  show("import");
}

// ---- 2. import ------------------------------------------------------------
const FILES = [
  ["tb", "Trial balance (this year)", "Required. Ledger, Group, Opening, Closing (or Debit and Credit)."],
  ["py_tb", "Trial balance (last year)", "For comparatives, opening-balance checks and cash flow."],
  ["vouchers", "Day book (vouchers)", "For cash limits, loans, ageing and partner capital movement."],
  ["far", "Fixed asset register", "Sheet 'Fixed Assets'; optional sheet 'IT Opening'."],
  ["accounts_master", "Account master (BUSY)", "Only if the trial balance has no Group column."],
];
function renderFiles() {
  const box = $("#fileRows"); box.innerHTML = "";
  const inp = state.settings.inputs;
  for (const [kind, title, help] of FILES) {
    const have = inp[kind] || (kind === "tb" && inp.tally_company);
    const row = document.createElement("div"); row.className = "file-row";
    row.innerHTML = `<div><div class="name">${title}</div><div class="state ${have ? "ok" : ""}">${have ? (inp.tally_company && kind !== "far" && kind !== "accounts_master" && String(inp[kind] || "").endsWith(".json") ? "Imported from Tally: " + esc(inp.tally_company) : "Loaded") : esc(help)}</div></div>
      <label class="small"><input type="file" accept=".xlsx,.xls,.csv,.xlsm,.ods" hidden><span class="pseudo-btn">${have ? "Replace" : "Choose file"}</span></label>`;
    const input = row.querySelector("input"), span = row.querySelector(".pseudo-btn");
    span.setAttribute("role", "button"); span.tabIndex = 0;
    span.className = "pseudo-btn"; span.style.cssText = "display:inline-block;padding:6px 12px;border:1px solid var(--line);border-radius:6px;cursor:pointer;background:var(--paper)";
    span.addEventListener("keydown", e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); input.click(); } });
    input.addEventListener("change", async () => {
      const f = input.files[0]; if (!f) return;
      setStatus($("#fStatus"), `Reading ${f.name}...`);
      try {
        const r = await api("POST", `/api/projects/${pid()}/upload?kind=${kind}&name=${encodeURIComponent(f.name)}`, undefined, await f.arrayBuffer());
        setStatus($("#fStatus"), `${f.name}: ${r.contents} loaded.`);
        const p = await api("GET", `/api/projects/${pid()}`); state.settings = p.settings; state.analysis = null; renderFiles();
      } catch (e) { setStatus($("#fStatus"), e.message, true); }
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
    setStatus($("#tStatus"), c.length ? `${c.length} compan${c.length === 1 ? "y" : "ies"} found.` : "Tally answered but no company is open.");
  } catch (e) { setStatus($("#tStatus"), e.message, true); }
});
$("#tImport").addEventListener("click", async () => {
  const company = $("#tCompany").value; if (!company) return setStatus($("#tStatus"), "Choose a company first.", true);
  setStatus($("#tStatus"), "Importing from Tally. Large books take a minute...");
  $("#tImport").disabled = true;
  try {
    const r = await api("POST", `/api/projects/${pid()}/tally`, { host: $("#tHost").value, port: Number($("#tPort").value), company, vouchers: $("#tVouchers").checked });
    setStatus($("#tStatus"), `Imported ${r.ledgers} ledgers, ${r.vouchers} vouchers${r.previous_year ? ", and last year's balances" : ""}.`);
    const p = await api("GET", `/api/projects/${pid()}`); state.settings = p.settings; state.analysis = null; renderFiles();
  } catch (e) { setStatus($("#tStatus"), e.message, true); }
  $("#tImport").disabled = false;
});

// ---- 3. check -------------------------------------------------------------
const SEV = { blocker: "Must fix", warning: "Check", info: "Note" };
async function runChecks() {
  const b = $("#runChecks"); b.disabled = true; b.textContent = "Checking...";
  try {
    state.analysis = await api("POST", `/api/projects/${pid()}/analyse`);
    renderCheck(); renderMap();
  } catch (e) { toast(e.message, true); }
  b.disabled = false; b.textContent = "Run checks again";
}
$("#runChecks").addEventListener("click", runChecks);
$$(".filters [data-f]").forEach(b => b.addEventListener("click", () => { state.filter = b.dataset.f; $$(".filters [data-f]").forEach(x => x.classList.toggle("on", x === b)); renderCheck(); }));
$("#expert").addEventListener("change", () => { if (state.analysis) renderCheck(); });

function renderCheck() {
  const a = state.analysis; if (!a) return;
  const s = a.summary;
  $("#tiles").innerHTML = `
    <div class="tile ${s.must_fix ? "blocker" : ""}"><div class="k">Must fix</div><div class="v">${s.must_fix}</div></div>
    <div class="tile ${s.check ? "warning" : ""}"><div class="k">Check</div><div class="v">${s.check}</div></div>
    <div class="tile"><div class="k">Notes</div><div class="v">${s.notes}</div></div>
    <div class="tile"><div class="k">Profit / (loss)</div><div class="v money">${esc(s.profit)}</div></div>
    <div class="tile"><div class="k">Balance sheet total</div><div class="v money">${esc(s.total_assets)}</div></div>
    <div class="tile"><div class="k">Data</div><div class="v" style="font-size:14px;font-weight:500">${s.ledgers} ledgers, ${s.vouchers} vouchers${s.has_previous_year ? ", last year" : ""}${s.has_far ? ", asset register" : ""}</div></div>`;
  const ul = $("#findings"); ul.innerHTML = "";
  const list = a.findings.filter(f => state.filter === "all" || f.severity === state.filter);
  if (!list.length) ul.innerHTML = `<li class="muted">${a.findings.length ? "Nothing in this filter." : "No issues found. The books pass every check."}</li>`;
  for (const f of list) {
    const li = document.createElement("li"); li.className = "finding";
    li.innerHTML = `<span class="badge ${f.severity}">${SEV[f.severity]}</span><span class="t">${esc(f.title)}</span>
      <span><button class="small ai-explain" type="button">Explain</button></span>
      <div class="m">${esc(f.message)}</div>
      ${f.suggestion ? `<div class="s"><b>What to do:</b> ${esc(f.suggestion)}</div>` : ""}
      ${expert() && f.legal_ref ? `<div class="ref">Reference: ${esc(f.legal_ref)}</div>` : ""}`;
    li.querySelector(".ai-explain").addEventListener("click", async ev => {
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
async function renderMap() {
  await loadHeads();
  const a = state.analysis; if (!a) return;
  const q = $("#mapSearch").value.toLowerCase(), only = $("#mapAttention").checked;
  const tb = $("#mapTable tbody"); tb.innerHTML = "";
  const rows = a.mapping.filter(m => (!only || needsAttention(m)) && (!q || m.name.toLowerCase().includes(q) || m.group.toLowerCase().includes(q)));
  if (!rows.length) tb.innerHTML = `<tr><td colspan="6" class="muted">${only ? "Nothing needs attention. Untick the box to see every ledger." : "No ledger matches."}</td></tr>`;
  for (const m of rows) {
    const tr = document.createElement("tr");
    const opts = state.heads.map(h => `<option value="${h.id}" ${h.id === m.head ? "selected" : ""}>${esc(h.label)}</option>`).join("");
    const why = reasons(m);
    tr.innerHTML = `<td>${needsAttention(m) ? '<span class="needs" aria-hidden="true"></span>' : ""}${esc(m.name)}${why.length ? `<div class="src">${esc(why.join("; "))}</div>` : ""}</td>
      <td>${esc(m.group)}${m.standard_group ? "" : ' <span class="badge blocker">group not recognised</span>'}</td>
      <td class="num">${esc(m.amount)}</td>
      <td><select class="head" aria-label="Shown under for ${esc(m.name)}"><option value="">Not mapped</option>${opts}</select>
        <div class="src">${m.source === "memory" ? "Your choice (remembered)" : m.source === "name_rule" ? "By ledger name" : m.source ? "By group" : ""}${m.reclassified ? ", moved by balance side" : ""}</div></td>
      <td><div class="tags">${TAGS.map(t => `<button type="button" class="tag ${m.tags.includes(t) ? "on" : ""}" data-t="${t}" title="${TAG_TIP[t]}">${t}</button>`).join("")}</div></td>
      <td><button class="small" type="button">Suggest</button></td>`;
    tr.querySelector("select").addEventListener("change", async e => {
      try { await api("POST", `/api/projects/${pid()}/mapping`, { ledger: m.name, head: e.target.value }); toast("Saved. Remembered for this client."); await runChecks(); }
      catch (err) { toast(err.message, true); }
    });
    tr.querySelectorAll(".tag").forEach(b => b.addEventListener("click", async () => {
      b.classList.toggle("on");
      const tags = [...tr.querySelectorAll(".tag.on")].map(x => x.dataset.t);
      try { await api("POST", `/api/projects/${pid()}/tags`, { ledger: m.name, tags }); m.tags = tags; toast("Tag saved."); }
      catch (err) { toast(err.message, true); }
    }));
    tr.querySelector("td:last-child button").addEventListener("click", async e => {
      const btn = e.currentTarget; btn.disabled = true; btn.textContent = "Thinking...";
      try {
        const r = await api("POST", `/api/projects/${pid()}/ai/map`, { ledger: m.name });
        if (confirm(`${r.label} (${r.model}):\n\n${r.head_label}\n${r.reason}\n\nUse this?`)) {
          await api("POST", `/api/projects/${pid()}/mapping`, { ledger: m.name, head: r.head, ai: true }); await runChecks();
        }
      } catch (err) { toast(err.message, true); }
      btn.disabled = false; btn.textContent = "Suggest";
    });
    tb.appendChild(tr);
  }
}
$("#mapSearch").addEventListener("input", renderMap);
$("#mapAttention").addEventListener("change", renderMap);

// ---- 5. presentation ----------------------------------------------------------
async function loadPresent() {
  const p = await api("GET", `/api/projects/${pid()}`); state.settings = p.settings;
  const o = p.settings.options;
  $("#oUnit").value = o.unit; $("#oDec").value = String(o.decimals);
  $$('input[name=layout]').forEach(r => (r.checked = r.value === o.layout));
  $("#oPy").checked = o.show_previous_year; $("#oNil").checked = o.hide_nil_lines; $("#oRel").checked = o.reletter;
  $("#oCover").checked = o.cover_page; $("#oPol").checked = o.accounting_policies; $("#oAge").checked = o.ageing; $("#oParty").checked = o.party_wise_details;
  $("#oCf").value = o.cash_flow; $("#oRatios").value = o.ratios; $("#oDetails").value = (o.entity_details || []).join("\n");
  if (!state.analysis) { try { state.analysis = await api("POST", `/api/projects/${pid()}/analyse`); } catch (e) { toast(e.message, true); } }
  // Profit sharing (non-company).
  const isCo = p.settings.entity_type === "company";
  $("#psBox").hidden = isCo;
  const ps = $("#psRows"); ps.innerHTML = "";
  if (!isCo && state.analysis) {
    const owners = state.analysis.mapping.filter(m => m.standard_group === "Capital Account" && !/drawing/i.test(m.name));
    for (const m of owners) {
      const cur = (p.settings.profit_sharing.find(x => x[0] === m.name) || [m.name, 1])[1];
      ps.insertAdjacentHTML("beforeend", `<div class="ps-row"><div class="field"><label>${esc(m.name)}</label></div><div class="field"><input type="number" min="0" step="1" value="${cur}" data-owner="${esc(m.name)}" aria-label="Share of ${esc(m.name)}"></div></div>`);
    }
    if (!owners.length) ps.innerHTML = '<p class="muted small">No capital accounts found.</p>';
  }
  // Ratio explanations.
  const rr = $("#ratioRows"); rr.innerHTML = "";
  const ratiosOn = o.ratios === "on" || (o.ratios === "auto" && isCo);
  const need = ratiosOn ? (state.analysis?.ratios || []).filter(r => r.needs_explanation) : [];
  $("#ratioBox").hidden = !need.length;
  for (const r of need) {
    const div = document.createElement("div"); div.className = "ratio-row";
    div.innerHTML = `<div class="rh"><b>${esc(r.name)}</b><span class="muted">${r.variance_pct > 0 ? "+" : ""}${r.variance_pct.toFixed(1)}%</span></div><textarea rows="2" data-ratio="${esc(r.name)}">${esc(o.ratio_explanations[r.name] || "")}</textarea><div><button class="small" type="button">Draft with AI</button></div>`;
    div.querySelector("button").addEventListener("click", async e => {
      const b = e.currentTarget; b.disabled = true;
      try { const x = await api("POST", `/api/projects/${pid()}/ai/ratio`, { name: r.name }); div.querySelector("textarea").value = x.text; toast("Draft added. Review before saving."); }
      catch (err) { toast(err.message, true); }
      b.disabled = false;
    });
    rr.appendChild(div);
  }
  refreshPreview();
}
function refreshPreview() { $("#preview").src = `/api/projects/${pid()}/preview?t=${TOKEN}&ts=${Date.now()}`; }
$("#refreshPreview").addEventListener("click", refreshPreview);
$("#opts").addEventListener("submit", async e => {
  e.preventDefault();
  const o = state.settings.options;
  const options = {
    ...o,
    unit: $("#oUnit").value, decimals: Number($("#oDec").value),
    layout: ($$('input[name=layout]').find(r => r.checked) || {}).value || "boxed",
    show_previous_year: $("#oPy").checked, hide_nil_lines: $("#oNil").checked, reletter: $("#oRel").checked,
    cover_page: $("#oCover").checked, accounting_policies: $("#oPol").checked, ageing: $("#oAge").checked, party_wise_details: $("#oParty").checked,
    cash_flow: $("#oCf").value, ratios: $("#oRatios").value,
    entity_details: $("#oDetails").value.split("\n").map(x => x.trim()).filter(Boolean),
    ratio_explanations: Object.fromEntries($$("#ratioRows textarea").map(t => [t.dataset.ratio, t.value.trim()]).filter(x => x[1])),
  };
  const profit_sharing = $$("#psRows input").map(i => [i.dataset.owner, Math.max(0, Math.round(Number(i.value) || 0))]);
  try {
    await api("POST", `/api/projects/${pid()}/settings`, { options, profit_sharing });
    state.settings.options = options; state.settings.profit_sharing = profit_sharing; state.analysis = null;
    setStatus($("#oStatus"), "Saved."); refreshPreview();
  } catch (err) { setStatus($("#oStatus"), err.message, true); }
});

// ---- 6. sign and export -----------------------------------------------------
function sigRow(sg = {}) {
  const d = document.createElement("div"); d.className = "sig-row";
  d.innerHTML = `<div class="field"><label>Name</label><input data-k="name" value="${esc(sg.name || "")}"></div>
    <div class="field"><label>Designation</label><input data-k="designation" value="${esc(sg.designation || "")}"></div>
    <div class="field"><label>ID type</label><input data-k="id_label" value="${esc(sg.id_label || "")}" placeholder="DIN"></div>
    <div class="field"><label>ID number</label><input data-k="id" value="${esc(sg.id || "")}"></div>
    <button type="button" class="small" aria-label="Remove signatory">Remove</button>`;
  d.querySelector("button").addEventListener("click", () => d.remove());
  $("#sigRows").appendChild(d);
}
async function loadSign() {
  const form = $("#signForm"); form.inert = true; form.setAttribute("aria-busy", "true");
  const p = await api("GET", `/api/projects/${pid()}`); state.settings = p.settings;
  const s = p.settings.signoff;
  $("#sFirm").value = s.auditor_firm; $("#sFrn").value = s.frn; $("#sPartner").value = s.auditor_partner; $("#sMno").value = s.membership_no;
  $("#sUdin").value = s.udin; $("#sPlace").value = s.place; $("#sDate").value = s.date;
  $("#sigRows").innerHTML = ""; (s.signatories.length ? s.signatories : [{}]).forEach(sigRow);
  if (!state.analysis) { try { state.analysis = await api("POST", `/api/projects/${pid()}/analyse`); } catch {} }
  const blocked = state.analysis && state.analysis.summary.must_fix > 0;
  $("#modeFinal").disabled = !!blocked;
  setStatus($("#eStatus"), blocked ? `Final copy is locked until ${state.analysis.summary.must_fix} "Must fix" item(s) are cleared. Draft export is available.` : "");
  form.inert = false; form.removeAttribute("aria-busy"); form.dataset.ready = "1";
}
$("#addSig").addEventListener("click", () => sigRow());
$("#signForm").addEventListener("submit", async e => {
  e.preventDefault();
  const signoff = {
    auditor_firm: $("#sFirm").value.trim(), frn: $("#sFrn").value.trim(), auditor_partner: $("#sPartner").value.trim(), membership_no: $("#sMno").value.trim(),
    udin: $("#sUdin").value.trim(), place: $("#sPlace").value.trim(), date: $("#sDate").value.trim(),
    signatories: $$("#sigRows .sig-row").map(r => Object.fromEntries($$("input", r).map(i => [i.dataset.k, i.value.trim()]))).filter(x => x.name || x.designation),
  };
  setStatus($("#eStatus"), "Exporting...");
  try {
    await api("POST", `/api/projects/${pid()}/settings`, { signoff });
    const mode = ($$('input[name=mode]').find(r => r.checked) || {}).value;
    const r = await api("POST", `/api/projects/${pid()}/export`, { mode, folder: $("#eFolder").value });
    setStatus($("#eStatus"), "Done.");
    const box = $("#eResult"); box.hidden = false;
    box.innerHTML = `<h2>Saved</h2><p>Folder: <b>${esc(r.dir)}</b></p><ul>${(r.files || []).map(f => `<li>${esc(f[0])}</li>`).join("")}</ul>` +
      (r.warnings?.length ? `<h2>Before signing</h2><ul>${r.warnings.map(w => `<li>${esc(w)}</li>`).join("")}</ul>` : "");
  } catch (err) { setStatus($("#eStatus"), err.message, true); }
});

// ---- 7. audit ---------------------------------------------------------------
const short = h => (h ? String(h).slice(0, 10) + "..." : "");
function describe(e) {
  const d = e.details || {};
  switch (e.action) {
    case "project_created": return `Started ${d.entity}, FY ${d.fy}`;
    case "file_imported": return `${d.original_name}: ${d.contents} (fingerprint ${short(d.sha256)})`;
    case "tally_import": return `From Tally company "${d.company}": ${d.ledgers} ledgers, ${d.vouchers} vouchers${d.previous_year ? ", last year's balances" : ""}`;
    case "checks_run": return `Must fix ${d.must_fix}, Check ${d.check}, Notes ${d.notes} (rules ${d.rules_version})`;
    case "mapping_changed": {
      const lbl = id => (state.heads.find(h => h.id === id) || {}).label || id;
      return `${d.ledger}: ${d.from ? lbl(d.from) : "automatic"} to ${d.to ? lbl(d.to) : "automatic"}${d.ai_suggested ? " (AI suggestion accepted)" : ""}`;
    }
    case "tags_changed": return `${d.ledger}: tags ${(d.to || []).join(", ") || "removed"}`;
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
loadProjects().catch(e => toast(e.message, true));
refreshStatus();
