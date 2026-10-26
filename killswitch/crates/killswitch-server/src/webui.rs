//! # Embedded Interactive WebUI Dashboard (`GET /` and `GET /ui`)
//!
//! Serves a self-contained, zero-external-dependency HTML5/CSS/JS Security Operations Dashboard
//! for the Standalone Cross-Platform Rust KillSwitch Agent.
//!
//! Exposes interactive controls for:
//! - Autonomous & Manual Reversible Host Containment (`GET /status`, `POST /engage`, `POST /disarm`, `POST /health-probe`, `POST /mode`)
//! - 10-Node Cyclic LangGraph Stateful Agent Orchestrator (`GET /graph/state`, `POST /graph/step`)
//! - 100% OpenTelemetry & OpenLineage Endpoint DLP Data Lineage Engine (`GET /lineage/status`, `POST /lineage/seed`, `POST /lineage/evaluate`)
//! - Octopus 18-Arm Containment Rings & EDR Telemetry Ingest (`GET /octopus/arms`, `POST /edr/ingest`)
//! - Cross-Platform Native OS Service Management (`GET /service/status`, `POST /service/control`)
//! - BLAKE3 Hash-Chained Audit Ledger (`GET /history`)

use axum::response::Html;

pub async fn get_webui_dashboard() -> Html<&'static str> {
    Html(WEBUI_HTML)
}

pub const WEBUI_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light dark">
  <title>KillSwitch — Autonomous &amp; Reversible Endpoint Security Agent</title>
  <style>
    :root {
      --bg-light: #f4f6fb;
      --bg-dark: #0b0f19;
      --surface-light: #ffffff;
      --surface-dark: #131b2e;
      --surface-alt-light: #eef2f9;
      --surface-alt-dark: #1a243d;
      --text-light: #0f172a;
      --text-dark: #e2e8f0;
      --muted-light: #475569;
      --muted-dark: #94a3b8;
      --border-light: #cbd5e1;
      --border-dark: #26334d;
      --brand-light: #0284c7;
      --brand-dark: #38bdf8;
      --danger-light: #dc2626;
      --danger-dark: #f87171;
      --ok-light: #16a34a;
      --ok-dark: #4ade80;
      --warn-light: #d97706;
      --warn-dark: #fbbf24;
      --scroll-track-light: #e2e8f0;
      --scroll-track-dark: #111827;
      --scroll-thumb-light: #94a3b8;
      --scroll-thumb-dark: #475569;

      --bg: var(--bg-light);
      --surface: var(--surface-light);
      --surface-alt: var(--surface-alt-light);
      --text: var(--text-light);
      --muted: var(--muted-light);
      --border: var(--border-light);
      --brand: var(--brand-light);
      --danger: var(--danger-light);
      --ok: var(--ok-light);
      --warn: var(--warn-light);
      --scrollbar-track: var(--scroll-track-light);
      --scrollbar-thumb: var(--scroll-thumb-light);

      color-scheme: light dark;
      accent-color: var(--brand);
      scrollbar-width: thin;
      scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
    }

    @media (prefers-color-scheme: dark) {
      :root {
        --bg: var(--bg-dark);
        --surface: var(--surface-dark);
        --surface-alt: var(--surface-alt-dark);
        --text: var(--text-dark);
        --muted: var(--muted-dark);
        --border: var(--border-dark);
        --brand: var(--brand-dark);
        --danger: var(--danger-dark);
        --ok: var(--ok-dark);
        --warn: var(--warn-dark);
        --scrollbar-track: var(--scroll-track-dark);
        --scrollbar-thumb: var(--scroll-thumb-dark);
      }
    }

    @supports (color: light-dark(white, black)) {
      :root {
        --bg: light-dark(var(--bg-light), var(--bg-dark));
        --surface: light-dark(var(--surface-light), var(--surface-dark));
        --surface-alt: light-dark(var(--surface-alt-light), var(--surface-alt-dark));
        --text: light-dark(var(--text-light), var(--text-dark));
        --muted: light-dark(var(--muted-light), var(--muted-dark));
        --border: light-dark(var(--border-light), var(--border-dark));
        --brand: light-dark(var(--brand-light), var(--brand-dark));
        --danger: light-dark(var(--danger-light), var(--danger-dark));
        --ok: light-dark(var(--ok-light), var(--ok-dark));
        --warn: light-dark(var(--warn-light), var(--warn-dark));
        --scrollbar-track: light-dark(var(--scroll-track-light), var(--scroll-track-dark));
        --scrollbar-thumb: light-dark(var(--scroll-thumb-light), var(--scroll-thumb-dark));
      }
    }

    @supports not (scrollbar-color: auto) {
      .table-wrapper::-webkit-scrollbar {
        width: 10px;
        height: 10px;
      }
      .table-wrapper::-webkit-scrollbar-thumb {
        background: var(--scrollbar-thumb);
      }
      .table-wrapper::-webkit-scrollbar-track {
        background: var(--scrollbar-track);
      }
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: var(--bg);
      color: var(--text);
      line-height: 1.5;
    }

    header {
      background: var(--surface);
      border-block-end: 1px solid var(--border);
      padding: 1rem 1.5rem;
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: 1rem;
      position: sticky;
      inset-block-start: 0;
      z-index: 10;
    }

    .brand-title {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .brand-title h1 {
      font-size: 1.25rem;
      margin: 0;
      font-weight: 700;
    }

    .badge {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      padding: 0.25rem 0.75rem;
      border-radius: 999px;
      font-size: 0.8rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      border: 1px solid var(--border);
      background: var(--surface-alt);
    }

    .badge.ok {
      color: var(--ok);
      border-color: var(--ok);
    }

    .badge.danger {
      color: var(--danger);
      border-color: var(--danger);
    }

    .badge.warn {
      color: var(--warn);
      border-color: var(--warn);
    }

    main {
      max-inline-size: 1440px;
      margin: 0 auto;
      padding: 1.5rem;
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
      gap: 1.25rem;
    }

    .card {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: 0.75rem;
      padding: 1.25rem;
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }

    .card.full-span {
      grid-column: 1 / -1;
    }

    .card h2 {
      margin: 0;
      font-size: 1.05rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 0.5rem;
      border-block-end: 1px solid var(--border);
      padding-block-end: 0.65rem;
    }

    .kv-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
      gap: 0.75rem;
    }

    .kv-box {
      background: var(--surface-alt);
      border: 1px solid var(--border);
      border-radius: 0.5rem;
      padding: 0.65rem 0.85rem;
    }

    .kv-label {
      font-size: 0.75rem;
      color: var(--muted);
      text-transform: uppercase;
      letter-spacing: 0.04em;
    }

    .kv-val {
      font-size: 0.95rem;
      font-weight: 600;
      margin-block-start: 0.2rem;
      word-break: break-all;
    }

    .form-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 0.75rem;
    }

    label {
      display: flex;
      flex-direction: column;
      gap: 0.3rem;
      font-size: 0.82rem;
      color: var(--muted);
      font-weight: 600;
    }

    input, select, button {
      font: inherit;
      padding: 0.5rem 0.75rem;
      border-radius: 0.45rem;
      border: 1px solid var(--border);
      background: var(--surface-alt);
      color: var(--text);
    }

    button {
      cursor: pointer;
      font-weight: 600;
      transition: opacity 0.15s ease;
    }

    button:hover {
      opacity: 0.9;
    }

    button.primary {
      background: var(--brand);
      color: #0b0f19;
      border-color: var(--brand);
    }

    button.danger {
      background: var(--danger);
      color: #ffffff;
      border-color: var(--danger);
    }

    button.ok {
      background: var(--ok);
      color: #052e16;
      border-color: var(--ok);
    }

    .btn-row {
      display: flex;
      flex-wrap: wrap;
      gap: 0.6rem;
      align-items: center;
    }

    .toast {
      padding: 0.65rem 0.9rem;
      border-radius: 0.5rem;
      border: 1px solid var(--border);
      background: var(--surface-alt);
      font-size: 0.85rem;
      font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      white-space: pre-wrap;
      word-break: break-word;
    }

    /* Responsive table pattern per modern-web-guidance */
    .table-wrapper {
      overflow: auto;
      max-inline-size: 100%;
      max-block-size: min(360px, 65vh);
      container-type: inline-size;
      scrollbar-gutter: stable;
      border: 1px solid var(--border);
      border-radius: 0.5rem;
    }

    table {
      width: 100%;
      border-collapse: separate;
      border-spacing: 0;
      font-size: 0.85rem;
    }

    caption {
      text-align: start;
      padding: 0.5rem 0.75rem;
      font-weight: 600;
      color: var(--muted);
      background: var(--surface-alt);
      border-block-end: 1px solid var(--border);
    }

    th, td {
      padding: 0.55rem 0.75rem;
      text-align: start;
      border-block-end: 1px solid var(--border);
    }

    thead {
      position: sticky;
      inset-block-start: 0;
      z-index: 1;
      background: var(--surface-alt);
    }

    tbody th {
      position: sticky;
      inset-inline-start: 0;
      background: var(--surface);
      font-weight: 600;
    }

    @container (width < 600px) {
      table, thead, tbody, tr, th, td {
        display: block;
      }

      thead {
        display: none;
      }

      tr {
        margin-block-end: 1rem;
        border: 1px solid var(--border);
      }

      tbody th {
        position: sticky;
        inset-block-start: 0;
        z-index: 2;
        background: var(--surface-alt);
      }

      td:nth-child(2)::before {
        content: var(--label-2) ": ";
        font-weight: 600;
        color: var(--muted);
      }
      td:nth-child(3)::before {
        content: var(--label-3) ": ";
        font-weight: 600;
        color: var(--muted);
      }
      td:nth-child(4)::before {
        content: var(--label-4) ": ";
        font-weight: 600;
        color: var(--muted);
      }
      td:nth-child(5)::before {
        content: var(--label-5) ": ";
        font-weight: 600;
        color: var(--muted);
      }
    }
  </style>
</head>
<body>
  <header>
    <div class="brand-title">
      <h1>🛡️ KillSwitch Security Operations &amp; LangGraph Agent</h1>
      <span id="isolation-badge" class="badge ok">OPERATIONAL</span>
      <span id="mode-badge" class="badge">MODE: AUTONOMOUS</span>
    </div>
    <div class="btn-row">
      <button type="button" class="primary" onclick="refreshAll()">🔄 Refresh Telemetry</button>
      <button type="button" onclick="toggleMode('autonomous')">🤖 Autonomous Mode</button>
      <button type="button" onclick="toggleMode('manual')">👤 Manual HITL Mode</button>
    </div>
  </header>

  <main>
    <!-- 1. Host Containment & Reversible Isolation -->
    <section class="card">
      <h2>
        <span>1. Host Containment &amp; Reversible KillSwitch</span>
        <span id="uptime-pill" class="badge">Uptime: 0s</span>
      </h2>
      <div class="kv-grid">
        <div class="kv-box">
          <div class="kv-label">Containment Level</div>
          <div id="st-level" class="kv-val">none</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Network Quarantine</div>
          <div id="st-net" class="kv-val">false</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Frozen / Tarpitted PIDs</div>
          <div id="st-pids" class="kv-val">0 / 0</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Allowlisted Ports</div>
          <div id="st-ports" class="kv-val">3030</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">BLAKE3 State Hash</div>
          <div id="st-hash" class="kv-val">-</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Last Reason / Operator</div>
          <div id="st-reason" class="kv-val">-</div>
        </div>
      </div>

      <div class="form-grid">
        <label>Operator ID
          <input id="ctl-operator" type="text" value="secops_admin">
        </label>
        <label>Containment Level
          <select id="ctl-level">
            <option value="hermetic_isolation">Hermetic Isolation (WFP / Netlink / PF + Freeze)</option>
            <option value="network_quarantine">Network Quarantine Only</option>
            <option value="process_freeze">Non-Destructive Process Freeze Only</option>
          </select>
        </label>
        <label>Reason
          <input id="ctl-reason" type="text" value="Operator console verification">
        </label>
        <label>Confirmation Token
          <input id="ctl-token" type="text" value="CONFIRM RESTORE">
        </label>
      </div>

      <div class="btn-row">
        <button type="button" class="danger" onclick="engageKillSwitch()">🚨 Engage KillSwitch Isolation</button>
        <button type="button" class="ok" onclick="disarmKillSwitch()">🟢 Disarm &amp; Restore Host</button>
        <button type="button" onclick="runHealthProbe(false)">🩺 Run Process-Tree Health Probe</button>
      </div>
    </section>

    <!-- 2. 10-Node Stateful LangGraph Orchestrator -->
    <section class="card">
      <h2>
        <span>2. Stateful LangGraph Agent Orchestrator</span>
        <span id="graph-valid-badge" class="badge ok">Trace Chain Valid</span>
      </h2>
      <div class="kv-grid">
        <div class="kv-box">
          <div class="kv-label">Current Graph Node</div>
          <div id="gr-node" class="kv-val">Monitoring</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Aggregate Threat Score</div>
          <div id="gr-score" class="kv-val">0.00</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Active Threats / Trace Len</div>
          <div id="gr-threats" class="kv-val">0 / 0</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Byzantine Fraud Proofs</div>
          <div id="gr-fraud" class="kv-val">0</div>
        </div>
      </div>

      <div class="form-grid">
        <label>Force Target Graph Node (Optional)
          <select id="gr-force-node">
            <option value="">Auto Transition (Next Cyclic Edge)</option>
            <option value="Monitoring">Monitoring</option>
            <option value="IngestingSignals">IngestingSignals</option>
            <option value="CorrelatingEdrAndOctopus">CorrelatingEdrAndOctopus</option>
            <option value="AdjudicatingZeroFalsePositive">AdjudicatingZeroFalsePositive</option>
            <option value="OptionalOllamaReasoning">OptionalOllamaReasoning</option>
            <option value="EngagingContainment">EngagingContainment</option>
            <option value="IsolatedHealthProbing">IsolatedHealthProbing</option>
            <option value="DisarmingAndRestoring">DisarmingAndRestoring</option>
          </select>
        </label>
      </div>

      <div class="btn-row">
        <button type="button" class="primary" onclick="stepLangGraph()">⚡ Step LangGraph Cycle</button>
        <button type="button" onclick="refreshGraph()">🔄 Refresh Graph State</button>
      </div>
      <div id="action-toast" class="toast">Ready. Zero external OS CLI subprocesses active.</div>
    </section>

    <!-- 3. 100% OpenTelemetry & OpenLineage Endpoint DLP Data Lineage Engine -->
    <section class="card full-span">
      <h2>
        <span>3. Hybrid OpenTelemetry &amp; OpenLineage Endpoint DLP Data Lineage Engine (100% Autonomous Voter)</span>
        <span id="dlp-counts-badge" class="badge">0 Files / 0 Processes</span>
      </h2>
      <div class="form-grid">
        <label>Sensitive File Path
          <input id="dlp-path" type="text" value="C:\Users\Alice\Documents\secret_roadmap.docx">
        </label>
        <label>Content SHA-256 Digest
          <input id="dlp-sha" type="text" value="8f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a90">
        </label>
        <label>DLP Classification
          <select id="dlp-class">
            <option value="CrownJewel">CrownJewel (Severity 3)</option>
            <option value="Confidential">Confidential (Severity 2)</option>
            <option value="Internal">Internal (Severity 1)</option>
            <option value="Public">Public (Severity 0)</option>
          </select>
        </label>
        <label>W3C Sticky Policy
          <select id="dlp-policy">
            <option value="Quarantine">Quarantine (Auto-KillSwitch Block)</option>
            <option value="Audit">Audit</option>
            <option value="Allow">Allow</option>
          </select>
        </label>
        <label>Egress Process PID
          <input id="dlp-pid" type="number" value="5001">
        </label>
        <label>Destination Egress URL
          <input id="dlp-url" type="text" value="https://external-drop.example.com/upload">
        </label>
      </div>

      <div class="btn-row">
        <button type="button" class="primary" onclick="seedSensitiveFile()">🌱 Seed Sensitive Asset (OTel Span)</button>
        <button type="button" class="danger" onclick="evaluateLineageEgress()">🛡️ Evaluate Inline Egress Verdict</button>
        <button type="button" onclick="seedDemoAssets()">📦 Seed CrownJewel + Confidential Demo Set</button>
      </div>

      <div tabindex="0" role="region" aria-labelledby="dlp-files-caption" class="table-wrapper">
        <table style="--label-2: 'SHA-256'; --label-3: 'Classification'; --label-4: 'Policy';">
          <caption id="dlp-files-caption">Tracked Tainted Files &amp; OpenLineage Datasets</caption>
          <thead>
            <tr>
              <th>File Path</th>
              <th>SHA-256 Digest</th>
              <th>Classification</th>
              <th>Sticky Policy</th>
            </tr>
          </thead>
          <tbody id="dlp-files-tbody">
            <tr><th colspan="4">No tainted files seeded yet.</th></tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- 4. Octopus 18-Arm Containment Mesh & EDR Telemetry -->
    <section class="card">
      <h2>
        <span>4. Octopus 18-Arm Containment Mesh &amp; EDR Telemetry</span>
        <span id="octopus-count-badge" class="badge ok">18 Arms Active</span>
      </h2>
      <div class="form-grid">
        <label>EDR Event PID
          <input id="edr-pid" type="number" value="8842">
        </label>
        <label>Process Name
          <input id="edr-name" type="text" value="suspicious_agent.exe">
        </label>
        <label>Command Line
          <input id="edr-cmd" type="text" value="suspicious_agent.exe --exfil">
        </label>
      </div>
      <div class="btn-row">
        <button type="button" class="primary" onclick="ingestEdrEvent()">📡 Ingest EDR Telemetry Event</button>
      </div>
      <div tabindex="0" role="region" aria-labelledby="octopus-caption" class="table-wrapper">
        <table style="--label-2: 'Arm Name'; --label-3: 'Role'; --label-4: 'Epoch';">
          <caption id="octopus-caption">Octopus Multi-Ring Containment Arms (Constant-Time HMAC-SHA256)</caption>
          <thead>
            <tr>
              <th>Arm ID</th>
              <th>Arm Name</th>
              <th>Role</th>
              <th>Generation Epoch</th>
            </tr>
          </thead>
          <tbody id="octopus-tbody">
            <tr><th colspan="4">Loading arms...</th></tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- 5. Cross-Platform Native OS Service & BLAKE3 History Ledger -->
    <section class="card">
      <h2>
        <span>5. Native OS Service &amp; BLAKE3 Audit Ledger</span>
        <span id="svc-state-badge" class="badge">Service: Querying</span>
      </h2>
      <div class="kv-grid">
        <div class="kv-box">
          <div class="kv-label">Service Name</div>
          <div id="svc-name" class="kv-val">killswitch</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Installed / State</div>
          <div id="svc-installed" class="kv-val">-</div>
        </div>
      </div>

      <div tabindex="0" role="region" aria-labelledby="history-caption" class="table-wrapper">
        <table style="--label-2: 'Timestamp'; --label-3: 'Operator'; --label-4: 'Reason'; --label-5: 'BLAKE3 Hash';">
          <caption id="history-caption">BLAKE3 Hash-Chained KillSwitch History Ledger</caption>
          <thead>
            <tr>
              <th>Action</th>
              <th>Timestamp</th>
              <th>Operator</th>
              <th>Reason</th>
              <th>BLAKE3 Event Hash</th>
            </tr>
          </thead>
          <tbody id="history-tbody">
            <tr><th colspan="5">Loading ledger...</th></tr>
          </tbody>
        </table>
      </div>
    </section>
  </main>

  <script>
    function setToast(msg) {
      const el = document.getElementById('action-toast');
      if (el) el.textContent = typeof msg === 'string' ? msg : JSON.stringify(msg, null, 2);
    }

    async function api(path, opts) {
      const res = await fetch(path, Object.assign({
        headers: { 'Content-Type': 'application/json' }
      }, opts || {}));
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(data.message || data.error || ('HTTP ' + res.status));
      }
      return data;
    }

    async function refreshStatus() {
      const s = await api('/status');
      const badge = document.getElementById('isolation-badge');
      badge.textContent = s.isolated ? '🚨 ISOLATED (' + s.containment_level + ')' : '🟢 OPERATIONAL';
      badge.className = 'badge ' + (s.isolated ? 'danger' : 'ok');

      document.getElementById('mode-badge').textContent = 'MODE: ' + (s.active_mode || 'autonomous').toUpperCase();
      document.getElementById('uptime-pill').textContent = 'Uptime: ' + s.uptime_secs + 's';
      document.getElementById('st-level').textContent = s.containment_level;
      document.getElementById('st-net').textContent = String(s.network_isolated);
      document.getElementById('st-pids').textContent = s.processes_frozen_count + ' frozen / ' + s.processes_tarpitted_count + ' tarpit';
      document.getElementById('st-ports').textContent = (s.allowlisted_ports || []).join(', ');
      document.getElementById('st-hash').textContent = (s.state_hash_blake3 || '').slice(0, 24) + '…';
      document.getElementById('st-reason').textContent = (s.reason || 'Normal') + (s.isolated_by ? ' (' + s.isolated_by + ')' : '');
    }

    async function refreshGraph() {
      const g = await api('/graph/state');
      document.getElementById('gr-node').textContent = g.current_node;
      document.getElementById('gr-score').textContent = Number(g.aggregate_threat_score || 0).toFixed(2);
      document.getElementById('gr-threats').textContent = g.active_threats_count + ' threats / ' + g.execution_trace_len + ' hops';
      document.getElementById('gr-fraud').textContent = String(g.vetoed_fraud_proofs_count);
      const vb = document.getElementById('graph-valid-badge');
      vb.textContent = g.trace_chain_valid ? 'Trace Chain Valid' : 'Trace Chain Tampered';
      vb.className = 'badge ' + (g.trace_chain_valid ? 'ok' : 'danger');
    }

    async function refreshLineage() {
      const l = await api('/lineage/status');
      document.getElementById('dlp-counts-badge').textContent =
        l.tainted_files_count + ' Tainted Files / ' + l.tainted_processes_count + ' Tainted Procs';
      const tbody = document.getElementById('dlp-files-tbody');
      tbody.replaceChildren();
      const files = l.tainted_files || [];
      if (files.length === 0) {
        const tr = document.createElement('tr');
        const th = document.createElement('th');
        th.colSpan = 4;
        th.textContent = 'No tainted files seeded yet.';
        tr.appendChild(th);
        tbody.appendChild(tr);
        return;
      }
      for (const f of files) {
        const tr = document.createElement('tr');
        const th = document.createElement('th');
        th.textContent = f.path;
        const tdSha = document.createElement('td');
        tdSha.textContent = (f.sha256 || '').slice(0, 20) + '…';
        const tdClass = document.createElement('td');
        tdClass.textContent = f.classification;
        const tdPol = document.createElement('td');
        tdPol.textContent = f.policy;
        tr.append(th, tdSha, tdClass, tdPol);
        tbody.appendChild(tr);
      }
    }

    async function refreshOctopus() {
      const o = await api('/octopus/arms');
      document.getElementById('octopus-count-badge').textContent = o.total_arms + ' Arms Active';
      const tbody = document.getElementById('octopus-tbody');
      tbody.replaceChildren();
      for (const arm of (o.arms || [])) {
        const tr = document.createElement('tr');
        const th = document.createElement('th');
        th.textContent = 'Arm #' + arm.arm_id;
        const tdName = document.createElement('td');
        tdName.textContent = arm.agent_name;
        const tdRole = document.createElement('td');
        tdRole.textContent = arm.role;
        const tdGen = document.createElement('td');
        tdGen.textContent = 'Gen ' + arm.generation;
        tr.append(th, tdName, tdRole, tdGen);
        tbody.appendChild(tr);
      }
    }

    async function refreshServiceAndHistory() {
      try {
        const sv = await api('/service/status');
        document.getElementById('svc-name').textContent = sv.name + ' (' + sv.display_name + ')';
        document.getElementById('svc-installed').textContent =
          (sv.is_installed ? 'Installed' : 'Not Installed') + ' / ' + sv.state;
        document.getElementById('svc-state-badge').textContent = 'Service: ' + sv.state;
      } catch (_) {}

      const h = await api('/history?limit=15');
      const tbody = document.getElementById('history-tbody');
      tbody.replaceChildren();
      const entries = h.entries || [];
      if (entries.length === 0) {
        const tr = document.createElement('tr');
        const th = document.createElement('th');
        th.colSpan = 5;
        th.textContent = 'No containment events recorded yet.';
        tr.appendChild(th);
        tbody.appendChild(tr);
        return;
      }
      for (const e of entries) {
        const tr = document.createElement('tr');
        const th = document.createElement('th');
        th.textContent = e.action;
        const tdTs = document.createElement('td');
        tdTs.textContent = e.timestamp;
        const tdOp = document.createElement('td');
        tdOp.textContent = e.operator;
        const tdRe = document.createElement('td');
        tdRe.textContent = e.reason;
        const tdHa = document.createElement('td');
        tdHa.textContent = (e.entry_hash || '').slice(0, 16) + '…';
        tr.append(th, tdTs, tdOp, tdRe, tdHa);
        tbody.appendChild(tr);
      }
    }

    async function refreshAll() {
      try {
        await Promise.all([
          refreshStatus(),
          refreshGraph(),
          refreshLineage(),
          refreshOctopus(),
          refreshServiceAndHistory()
        ]);
      } catch (e) {
        setToast('Refresh warning: ' + e.message);
      }
    }

    async function toggleMode(mode) {
      try {
        const res = await api('/mode', {
          method: 'POST',
          body: JSON.stringify({ mode })
        });
        setToast(res);
        await refreshAll();
      } catch (e) {
        setToast('Mode switch error: ' + e.message);
      }
    }

    async function engageKillSwitch() {
      try {
        const res = await api('/engage', {
          method: 'POST',
          body: JSON.stringify({
            operator: document.getElementById('ctl-operator').value,
            reason: document.getElementById('ctl-reason').value,
            level: document.getElementById('ctl-level').value,
            token: document.getElementById('ctl-token').value
          })
        });
        setToast(res);
        await refreshAll();
      } catch (e) {
        setToast('Engage error: ' + e.message);
      }
    }

    async function disarmKillSwitch() {
      try {
        const res = await api('/disarm', {
          method: 'POST',
          body: JSON.stringify({
            operator: document.getElementById('ctl-operator').value,
            reason: document.getElementById('ctl-reason').value,
            token: document.getElementById('ctl-token').value
          })
        });
        setToast(res);
        await refreshAll();
      } catch (e) {
        setToast('Disarm error: ' + e.message);
      }
    }

    async function runHealthProbe(forceRollback) {
      try {
        const res = await api('/health-probe', {
          method: 'POST',
          body: JSON.stringify({ force_rollback: forceRollback })
        });
        setToast(res);
        await refreshAll();
      } catch (e) {
        setToast('Health probe error: ' + e.message);
      }
    }

    async function stepLangGraph() {
      try {
        const forceNode = document.getElementById('gr-force-node').value || null;
        const res = await api('/graph/step', {
          method: 'POST',
          body: JSON.stringify({ force_node: forceNode })
        });
        setToast(res);
        await refreshAll();
      } catch (e) {
        setToast('Graph step error: ' + e.message);
      }
    }

    async function seedSensitiveFile() {
      try {
        const res = await api('/lineage/seed', {
          method: 'POST',
          body: JSON.stringify({
            path: document.getElementById('dlp-path').value,
            sha256: document.getElementById('dlp-sha').value,
            classification: document.getElementById('dlp-class').value,
            policy: document.getElementById('dlp-policy').value
          })
        });
        setToast(res);
        await refreshLineage();
      } catch (e) {
        setToast('Lineage seed error: ' + e.message);
      }
    }

    async function seedDemoAssets() {
      try {
        await api('/lineage/seed', {
          method: 'POST',
          body: JSON.stringify({
            path: 'C:\\Users\\Alice\\Documents\\secret_roadmap.docx',
            sha256: '8f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a90',
            classification: 'CrownJewel',
            policy: 'Quarantine'
          })
        });
        await api('/lineage/seed', {
          method: 'POST',
          body: JSON.stringify({
            path: 'C:\\Users\\Alice\\Documents\\q4_financials.xlsx',
            sha256: '4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c',
            classification: 'Confidential',
            policy: 'Audit'
          })
        });
        setToast('Seeded CrownJewel (secret_roadmap.docx) and Confidential (q4_financials.xlsx) into EndpointLineageEngine.');
        await refreshLineage();
      } catch (e) {
        setToast('Demo seed error: ' + e.message);
      }
    }

    async function evaluateLineageEgress() {
      try {
        const res = await api('/lineage/evaluate', {
          method: 'POST',
          body: JSON.stringify({
            pid: Number(document.getElementById('dlp-pid').value || 5001),
            destination_url: document.getElementById('dlp-url').value
          })
        });
        setToast(res);
        await refreshAll();
      } catch (e) {
        setToast('Lineage evaluate error: ' + e.message);
      }
    }

    async function ingestEdrEvent() {
      try {
        const res = await api('/edr/ingest', {
          method: 'POST',
          body: JSON.stringify({
            event_id: 'evt-' + Date.now(),
            pid: Number(document.getElementById('edr-pid').value || 8842),
            process_name: document.getElementById('edr-name').value,
            image_path: 'C:\\Temp\\' + document.getElementById('edr-name').value,
            command_line: document.getElementById('edr-cmd').value
          })
        });
        setToast(res);
        await refreshAll();
      } catch (e) {
        setToast('EDR ingest error: ' + e.message);
      }
    }

    refreshAll();
    setInterval(refreshAll, 5000);
  </script>
</body>
</html>
"#;
