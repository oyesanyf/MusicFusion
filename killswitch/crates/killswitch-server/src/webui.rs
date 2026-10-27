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

    :where(a:any-link, button, input, select):focus-visible {
      outline: 3px solid var(--brand);
      outline-offset: 3px;
    }

    button, input, select {
      min-block-size: 24px;
      min-inline-size: 24px;
    }

    button[aria-pressed="true"] {
      outline: 2px solid var(--brand);
      background: var(--brand);
      color: #0b0f19;
    }

    .status-banner {
      background: var(--surface-alt);
      border-block-end: 1px solid var(--border);
      padding: 0.65rem 1.5rem;
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: 0.75rem;
      font-size: 0.88rem;
      font-weight: 600;
    }

    .status-banner.isolated {
      background: color-mix(in srgb, var(--danger) 15%, var(--surface));
      border-color: var(--danger);
    }

    .toast.ok {
      border-color: var(--ok);
    }

    .toast.danger {
      border-color: var(--danger);
    }

    .toast.warn {
      border-color: var(--warn);
    }
  </style>
</head>
<body>
  <header>
    <div class="brand-title">
      <h1 id="dashboard-heading" tabindex="-1">🛡️ KillSwitch Standalone Security Agent</h1>
      <span id="isolation-badge" class="badge ok">🟢 OPERATIONAL</span>
      <span id="mode-badge" class="badge">MODE: AUTONOMOUS</span>
      <span id="standalone-badge" class="badge ok">STANDALONE SENSOR (NO EXTERNAL EDR REQUIRED)</span>
    </div>
    <div class="btn-row">
      <button type="button" class="primary" onclick="refreshAll(true, this)">🔄 Refresh All Telemetry</button>
      <button id="btn-mode-auto" type="button" aria-pressed="true" onclick="toggleMode('autonomous', this)">🤖 Autonomous Mode</button>
      <button id="btn-mode-manual" type="button" aria-pressed="false" onclick="toggleMode('manual', this)">👤 Manual HITL Mode</button>
    </div>
  </header>

  <div id="global-status-bar" class="status-banner" role="status" aria-live="polite" aria-atomic="true">
    <span id="global-status-text">🟢 System Ready — Standalone Native OS Sensor Active (Zero External EDR Required). WebUI Port Preserved.</span>
    <span id="global-last-action">Last Action: Initialized</span>
  </div>
  <div id="global-alert-bar" role="alert" aria-live="assertive" style="display:none;" class="status-banner isolated"></div>

  <main>
    <!-- 1. Host Containment & Reversible Isolation -->
    <section class="card">
      <h2>
        <span>1. Host Containment &amp; Reversible KillSwitch</span>
        <span id="uptime-pill" class="badge">Uptime: 0s</span>
      </h2>
      <div class="kv-grid">
        <div class="kv-box">
          <div class="kv-label">Containment Status</div>
          <div id="st-level" class="kv-val">disarmed (normal)</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Network Quarantine</div>
          <div id="st-net" class="kv-val">false (Internet Enabled)</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Frozen / Tarpitted PIDs</div>
          <div id="st-pids" class="kv-val">0 / 0</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Preserved WebUI Ports</div>
          <div id="st-ports" class="kv-val">3030, 4040</div>
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
        <label for="ctl-operator">Operator ID
          <input id="ctl-operator" type="text" value="secops_admin" autocomplete="username">
        </label>
        <label for="ctl-level">Containment Level
          <select id="ctl-level">
            <option value="hermetic_isolation">Hermetic Isolation (WFP / Netlink / PF + Freeze)</option>
            <option value="network_quarantine">Network Quarantine Only</option>
            <option value="process_freeze">Non-Destructive Process Freeze Only</option>
          </select>
        </label>
        <label for="ctl-reason">Reason
          <input id="ctl-reason" type="text" value="Operator console verification">
        </label>
        <label for="ctl-token">Confirmation Token
          <input id="ctl-token" type="text" value="CONFIRM RESTORE">
        </label>
      </div>

      <div class="btn-row">
        <button id="btn-engage" type="button" class="danger" onclick="engageKillSwitch(this)">🚨 Engage KillSwitch Isolation</button>
        <button id="btn-disarm" type="button" class="ok" onclick="disarmKillSwitch(this)">🟢 Disarm &amp; Restore Host</button>
        <button id="btn-probe" type="button" onclick="runHealthProbe(false, this)">🩺 Run Process-Tree Health Probe</button>
        <button id="btn-rollback" type="button" onclick="runHealthProbe(true, this)">♻️ Verify &amp; Early Rollback</button>
      </div>
      <div id="card1-status" class="toast">Containment Status: Ready. Click "Engage KillSwitch Isolation" to quarantine external traffic or "Disarm &amp; Restore Host" to restore internet.</div>
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
        <label for="gr-force-node">Force Target Graph Node (Optional)
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
        <button type="button" class="primary" onclick="stepLangGraph(this)">⚡ Step LangGraph Cycle</button>
        <button type="button" onclick="refreshGraph(true, this)">🔄 Refresh Graph State</button>
        <button type="button" onclick="resetLangGraph(this)">🔁 Reset to Monitoring</button>
      </div>
      <div id="action-toast" class="toast">LangGraph Orchestrator Ready. Click "Step LangGraph Cycle" to advance the 10-node state machine.</div>
    </section>

    <!-- 3. 100% OpenTelemetry & OpenLineage Endpoint DLP Data Lineage Engine -->
    <section class="card full-span">
      <h2>
        <span>3. Hybrid OpenTelemetry &amp; OpenLineage Endpoint DLP Data Lineage Engine (100% Autonomous Voter)</span>
        <span id="dlp-counts-badge" class="badge">0 Files / 0 Processes</span>
      </h2>
      <div class="form-grid">
        <label for="dlp-path">Sensitive File Path
          <input id="dlp-path" type="text" value="C:\Users\Alice\Documents\secret_roadmap.docx">
        </label>
        <label for="dlp-sha">Content SHA-256 Digest
          <input id="dlp-sha" type="text" value="8f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a90">
        </label>
        <label for="dlp-class">DLP Classification
          <select id="dlp-class">
            <option value="CrownJewel">CrownJewel (Severity 3)</option>
            <option value="Confidential">Confidential (Severity 2)</option>
            <option value="Internal">Internal (Severity 1)</option>
            <option value="Public">Public (Severity 0)</option>
          </select>
        </label>
        <label for="dlp-policy">W3C Sticky Policy
          <select id="dlp-policy">
            <option value="Quarantine">Quarantine (Auto-KillSwitch Block)</option>
            <option value="Audit">Audit</option>
            <option value="Allow">Allow</option>
          </select>
        </label>
        <label for="dlp-pid">Egress Process PID
          <input id="dlp-pid" type="number" value="5001">
        </label>
        <label for="dlp-url">Destination Egress URL
          <input id="dlp-url" type="text" value="https://external-drop.example.com/upload">
        </label>
      </div>

      <div class="btn-row">
        <button type="button" class="primary" onclick="seedSensitiveFile(this)">🌱 Seed Sensitive Asset (OTel Span)</button>
        <button type="button" onclick="seedDemoAssets(this)">📦 Seed CrownJewel + Confidential Demo Set</button>
        <button type="button" class="danger" onclick="evaluateLineageEgress(this)">🛡️ Evaluate Inline Egress Verdict</button>
        <button type="button" onclick="refreshLineage(true, this)">🔄 Refresh Lineage Table</button>
      </div>
      <div id="card3-status" class="toast">DLP Lineage Status: Ready. Seed a sensitive file and click "Evaluate Inline Egress Verdict" to verify W3C Sticky Taint blocking.</div>

      <div tabindex="0" role="region" aria-labelledby="dlp-files-caption" class="table-wrapper">
        <table style="--label-2: 'SHA-256'; --label-3: 'Classification'; --label-4: 'Policy';">
          <caption id="dlp-files-caption">Tracked Tainted Files &amp; OpenLineage Datasets</caption>
          <thead>
            <tr>
              <th scope="col">File Path</th>
              <th scope="col">SHA-256 Digest</th>
              <th scope="col">Classification</th>
              <th scope="col">Sticky Policy</th>
            </tr>
          </thead>
          <tbody id="dlp-files-tbody">
            <tr><th scope="row" colspan="4">No tainted files seeded yet.</th></tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- 4. Standalone Built-In Host Sensor & Octopus 18-Arm Containment Mesh -->
    <section class="card">
      <h2>
        <span>4. Standalone Host Sensor &amp; Octopus 18-Arm Mesh (No External EDR Needed)</span>
        <span id="octopus-count-badge" class="badge ok">18 Arms Active</span>
      </h2>
      <div class="form-grid">
        <label for="edr-pid">Target Process PID
          <input id="edr-pid" type="number" value="8842">
        </label>
        <label for="edr-name">Process Name
          <input id="edr-name" type="text" value="suspicious_agent.exe">
        </label>
        <label for="edr-cmd">Command Line
          <input id="edr-cmd" type="text" value="suspicious_agent.exe --exfil">
        </label>
      </div>
      <div class="btn-row">
        <button type="button" class="ok" onclick="scanLiveHostSensor(this)">🔍 Scan Live Host Processes (Standalone Sensor)</button>
        <button type="button" class="primary" onclick="ingestEdrEvent(this)">📡 Correlate Process Telemetry Event</button>
        <button type="button" onclick="refreshOctopus(true, this)">🔄 Refresh 18-Arm Mesh</button>
      </div>
      <div id="card4-status" class="toast">Standalone Sensor Ready: Runs 100% independently using built-in OS process, WinTrust &amp; socket inspection without external EDR.</div>
      <div tabindex="0" role="region" aria-labelledby="octopus-caption" class="table-wrapper">
        <table style="--label-2: 'Arm Name'; --label-3: 'Role'; --label-4: 'Epoch';">
          <caption id="octopus-caption">Octopus Multi-Ring Containment Arms (Constant-Time HMAC-SHA256)</caption>
          <thead>
            <tr>
              <th scope="col">Arm ID</th>
              <th scope="col">Arm Name</th>
              <th scope="col">Role</th>
              <th scope="col">Generation Epoch</th>
            </tr>
          </thead>
          <tbody id="octopus-tbody">
            <tr><th scope="row" colspan="4">Loading arms...</th></tr>
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
          <div class="kv-label">Service / Agent Name</div>
          <div id="svc-name" class="kv-val">killswitch</div>
        </div>
        <div class="kv-box">
          <div class="kv-label">Mode / State</div>
          <div id="svc-installed" class="kv-val">-</div>
        </div>
      </div>

      <div class="btn-row">
        <button type="button" class="primary" onclick="controlService('status', this)">🔍 Check Service Status</button>
        <button type="button" class="ok" onclick="controlService('stop', this)">🛑 Disarm &amp; Stop Service</button>
        <button type="button" onclick="refreshServiceAndHistory(true, this)">🔄 Refresh BLAKE3 Ledger</button>
      </div>
      <div id="card5-status" class="toast">BLAKE3 Audit Ledger Ready. Every containment and disarm transition is cryptographically hash-chained.</div>

      <div tabindex="0" role="region" aria-labelledby="history-caption" class="table-wrapper">
        <table style="--label-2: 'Timestamp'; --label-3: 'Operator'; --label-4: 'Reason'; --label-5: 'BLAKE3 Hash';">
          <caption id="history-caption">BLAKE3 Hash-Chained KillSwitch History Ledger</caption>
          <thead>
            <tr>
              <th scope="col">Action</th>
              <th scope="col">Timestamp</th>
              <th scope="col">Operator</th>
              <th scope="col">Reason</th>
              <th scope="col">BLAKE3 Event Hash</th>
            </tr>
          </thead>
          <tbody id="history-tbody">
            <tr><th scope="row" colspan="5">Loading ledger...</th></tr>
          </tbody>
        </table>
      </div>
    </section>
  </main>

  <script>
    function nowStamp() {
      return new Date().toLocaleTimeString();
    }

    function setCardStatus(cardId, msg, tone) {
      const text = '[' + nowStamp() + '] ' + (typeof msg === 'string' ? msg : JSON.stringify(msg, null, 2));
      if (cardId) {
        const cardEl = document.getElementById(cardId);
        if (cardEl) {
          cardEl.textContent = text;
          cardEl.className = 'toast' + (tone ? ' ' + tone : '');
        }
      }
      const actionToast = document.getElementById('action-toast');
      if (actionToast && (!cardId || cardId === 'action-toast')) {
        actionToast.textContent = text;
        actionToast.className = 'toast' + (tone ? ' ' + tone : '');
      }
      const lastActionEl = document.getElementById('global-last-action');
      if (lastActionEl) {
        const oneLine = typeof msg === 'string' ? msg.split('\n')[0] : 'Updated';
        lastActionEl.textContent = '[' + nowStamp() + '] ' + oneLine;
      }
      const alertBar = document.getElementById('global-alert-bar');
      if (alertBar) {
        if (tone === 'danger') {
          alertBar.style.display = 'flex';
          alertBar.textContent = text;
        } else if (tone === 'ok') {
          alertBar.style.display = 'none';
        }
      }
    }

    function setToast(msg, tone) {
      setCardStatus('action-toast', msg, tone);
    }

    async function withButtonFeedback(btn, busyLabel, fn) {
      const origText = btn ? btn.textContent : '';
      if (btn) {
        btn.disabled = true;
        btn.textContent = busyLabel || '⏳ Processing...';
      }
      try {
        return await fn();
      } finally {
        if (btn) {
          btn.disabled = false;
          btn.textContent = origText;
        }
      }
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
      badge.textContent = s.isolated ? '🚨 ISOLATED (' + s.containment_level.toUpperCase() + ')' : '🟢 OPERATIONAL';
      badge.className = 'badge ' + (s.isolated ? 'danger' : 'ok');

      const activeMode = (s.active_mode || 'autonomous').toLowerCase();
      document.getElementById('mode-badge').textContent = 'MODE: ' + activeMode.toUpperCase();
      const btnAuto = document.getElementById('btn-mode-auto');
      const btnManual = document.getElementById('btn-mode-manual');
      if (btnAuto && btnManual) {
        btnAuto.setAttribute('aria-pressed', activeMode === 'autonomous' ? 'true' : 'false');
        btnManual.setAttribute('aria-pressed', activeMode === 'manual' ? 'true' : 'false');
      }

      document.getElementById('uptime-pill').textContent = 'Uptime: ' + s.uptime_secs + 's';
      document.getElementById('st-level').textContent = s.isolated
        ? 'ISOLATED (' + s.containment_level + ')'
        : 'DISARMED (' + s.containment_level + ')';
      document.getElementById('st-net').textContent = s.network_isolated
        ? 'true (External Internet Blocked)'
        : 'false (Internet Connected)';
      document.getElementById('st-pids').textContent = s.processes_frozen_count + ' frozen / ' + s.processes_tarpitted_count + ' tarpit';
      const portsStr = (s.allowlisted_ports || []).join(', ');
      document.getElementById('st-ports').textContent = portsStr;
      document.getElementById('st-hash').textContent = (s.state_hash_blake3 || '').slice(0, 24) + '…';
      document.getElementById('st-reason').textContent = (s.reason || 'Normal') + (s.isolated_by ? ' (' + s.isolated_by + ')' : '');

      const globalBar = document.getElementById('global-status-bar');
      const globalText = document.getElementById('global-status-text');
      if (globalBar && globalText) {
        if (s.isolated) {
          globalBar.className = 'status-banner isolated';
          globalText.textContent =
            '🚨 HOST ISOLATED (' + s.containment_level + ') — External Internet Blocked | Preserved WebUI Ports: [' + portsStr + '] | Click "Disarm & Restore Host" to restore internet.';
        } else {
          globalBar.className = 'status-banner';
          globalText.textContent =
            '🟢 HOST OPERATIONAL (Internet Enabled) — Mode: ' + activeMode.toUpperCase() + ' | Preserved WebUI Ports: [' + portsStr + '] | Standalone OS Sensor Active.';
        }
      }
      return s;
    }

    async function refreshGraph(showFeedback, btn) {
      return withButtonFeedback(btn, '⏳ Refreshing...', async () => {
        const g = await api('/graph/state');
        document.getElementById('gr-node').textContent = g.current_node;
        document.getElementById('gr-score').textContent = Number(g.aggregate_threat_score || 0).toFixed(2);
        document.getElementById('gr-threats').textContent = g.active_threats_count + ' threats / ' + g.execution_trace_len + ' hops';
        document.getElementById('gr-fraud').textContent = String(g.vetoed_fraud_proofs_count);
        const vb = document.getElementById('graph-valid-badge');
        vb.textContent = g.trace_chain_valid ? 'Trace Chain Valid' : 'Trace Chain Tampered';
        vb.className = 'badge ' + (g.trace_chain_valid ? 'ok' : 'danger');
        if (showFeedback) {
          setCardStatus(
            'action-toast',
            '✅ Graph State Refreshed — Node: ' + g.current_node + ' | Score: ' + Number(g.aggregate_threat_score || 0).toFixed(2) + ' | Trace Hops: ' + g.execution_trace_len + ' |BLAKE3 Valid: ' + g.trace_chain_valid,
            'ok'
          );
        }
        return g;
      });
    }

    async function refreshLineage(showFeedback, btn) {
      return withButtonFeedback(btn, '⏳ Refreshing...', async () => {
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
          th.scope = 'row';
          th.textContent = 'No tainted files seeded yet.';
          tr.appendChild(th);
          tbody.appendChild(tr);
        } else {
          for (const f of files) {
            const tr = document.createElement('tr');
            const th = document.createElement('th');
            th.scope = 'row';
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
        if (showFeedback) {
          setCardStatus(
            'card3-status',
            '✅ DLP Lineage Refreshed — ' + l.tainted_files_count + ' tainted files & ' + l.tainted_processes_count + ' tainted processes tracked.',
            'ok'
          );
        }
        return l;
      });
    }

    async function refreshOctopus(showFeedback, btn) {
      return withButtonFeedback(btn, '⏳ Refreshing...', async () => {
        const o = await api('/octopus/arms');
        document.getElementById('octopus-count-badge').textContent = o.total_arms + ' Arms Active';
        const tbody = document.getElementById('octopus-tbody');
        tbody.replaceChildren();
        for (const arm of (o.arms || [])) {
          const tr = document.createElement('tr');
          const th = document.createElement('th');
          th.scope = 'row';
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
        if (showFeedback) {
          setCardStatus(
            'card4-status',
            '✅ Octopus Containment Mesh Refreshed — ' + o.total_arms + ' HMAC-SHA256 arms verified.',
            'ok'
          );
        }
        return o;
      });
    }

    async function refreshServiceAndHistory(showFeedback, btn) {
      return withButtonFeedback(btn, '⏳ Refreshing...', async () => {
        let svcText = 'standalone_running';
        try {
          const sv = await api('/service/status');
          document.getElementById('svc-name').textContent = sv.name + ' (' + sv.display_name + ')';
          document.getElementById('svc-installed').textContent =
            (sv.is_installed ? 'OS Service Installed' : 'Standalone Agent Mode') + ' / ' + sv.state + (sv.pid ? ' (PID ' + sv.pid + ')' : '');
          document.getElementById('svc-state-badge').textContent = 'Agent: ' + sv.state;
          svcText = sv.state;
        } catch (_) {}

        const h = await api('/history?limit=15');
        const tbody = document.getElementById('history-tbody');
        tbody.replaceChildren();
        const entries = h.entries || [];
        if (entries.length === 0) {
          const tr = document.createElement('tr');
          const th = document.createElement('th');
          th.colSpan = 5;
          th.scope = 'row';
          th.textContent = 'No containment events recorded yet.';
          tr.appendChild(th);
          tbody.appendChild(tr);
        } else {
          for (const e of entries) {
            const tr = document.createElement('tr');
            const th = document.createElement('th');
            th.scope = 'row';
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
        if (showFeedback) {
          setCardStatus(
            'card5-status',
            '✅ BLAKE3 Ledger & Service Refreshed — Agent State: ' + svcText + ' | Total Ledger Entries: ' + h.total_entries + ' | Chain Valid: ' + h.chain_valid,
            'ok'
          );
        }
        return h;
      });
    }

    async function refreshAll(showFeedback, btn) {
      return withButtonFeedback(btn, '⏳ Refreshing...', async () => {
        try {
          const [s, g, l, o, h] = await Promise.all([
            refreshStatus(),
            refreshGraph(false),
            refreshLineage(false),
            refreshOctopus(false),
            refreshServiceAndHistory(false)
          ]);
          if (showFeedback) {
            setCardStatus(
              'card1-status',
              '✅ All 6 Subsystems Refreshed — Status: ' + (s.isolated ? 'ISOLATED' : 'OPERATIONAL') + ' | Graph Node: ' + g.current_node + ' | DLP Files: ' + l.tainted_files_count + ' | Arms: ' + o.total_arms + ' | Ledger Entries: ' + h.total_entries,
              'ok'
            );
          }
        } catch (e) {
          setCardStatus('card1-status', '⚠️ Refresh warning: ' + e.message, 'warn');
        }
      });
    }

    async function toggleMode(mode, btn) {
      return withButtonFeedback(btn, '⏳ Switching...', async () => {
        try {
          const res = await api('/mode', {
            method: 'POST',
            body: JSON.stringify({ mode })
          });
          await refreshAll(false);
          setCardStatus(
            'action-toast',
            '✅ Orchestrator Mode switched from ' + res.previous_mode.toUpperCase() + ' to ' + res.current_mode.toUpperCase(),
            'ok'
          );
        } catch (e) {
          setCardStatus('action-toast', '❌ Mode switch error: ' + e.message, 'danger');
        }
      });
    }

    async function engageKillSwitch(btn) {
      return withButtonFeedback(btn, '🚨 Engaging Isolation...', async () => {
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
          await refreshAll(false);
          setCardStatus(
            'card1-status',
            '🚨 ISOLATION ENGAGED: ' + res.message + '\nBLAKE3 State Hash: ' + (res.status.state_hash_blake3 || ''),
            'danger'
          );
        } catch (e) {
          setCardStatus('card1-status', '❌ Engage error: ' + e.message, 'danger');
        }
      });
    }

    async function disarmKillSwitch(btn) {
      return withButtonFeedback(btn, '🟢 Disarming & Restoring...', async () => {
        try {
          const res = await api('/disarm', {
            method: 'POST',
            body: JSON.stringify({
              operator: document.getElementById('ctl-operator').value,
              reason: document.getElementById('ctl-reason').value,
              token: document.getElementById('ctl-token').value
            })
          });
          await refreshAll(false);
          setCardStatus(
            'card1-status',
            '🟢 HOST RESTORED: ' + res.message,
            'ok'
          );
        } catch (e) {
          setCardStatus('card1-status', '❌ Disarm error: ' + e.message, 'danger');
        }
      });
    }

    async function runHealthProbe(forceRollback, btn) {
      return withButtonFeedback(btn, '🩺 Probing Host...', async () => {
        try {
          const res = await api('/health-probe', {
            method: 'POST',
            body: JSON.stringify({ force_rollback: forceRollback })
          });
          await refreshAll(false);
          const r = res.probe_report || {};
          setCardStatus(
            'card1-status',
            '🩺 Health Probe Complete — Scanned ' + (r.total_processes_scanned || 0) + ' live processes | Verified ' + (r.critical_os_anchors_verified || 0) + ' critical OS anchors | Anomalies: ' + ((r.anomalies || []).length) + ' | Early Rollback Executed: ' + res.early_rollback_executed,
            'ok'
          );
        } catch (e) {
          setCardStatus('card1-status', '❌ Health probe error: ' + e.message, 'danger');
        }
      });
    }

    async function stepLangGraph(btn) {
      return withButtonFeedback(btn, '⚡ Stepping Graph...', async () => {
        try {
          const forceNode = document.getElementById('gr-force-node').value || null;
          const res = await api('/graph/step', {
            method: 'POST',
            body: JSON.stringify({ force_node: forceNode })
          });
          await refreshAll(false);
          setCardStatus(
            'action-toast',
            '⚡ LangGraph Step Executed: ' + res.summary + ' | Current Node: ' + res.executed_node + ' | Threat Score: ' + Number(res.threat_score || 0).toFixed(2),
            'ok'
          );
        } catch (e) {
          setCardStatus('action-toast', '❌ Graph step error: ' + e.message, 'danger');
        }
      });
    }

    async function resetLangGraph(btn) {
      return withButtonFeedback(btn, '🔁 Resetting...', async () => {
        try {
          const res = await api('/graph/step', {
            method: 'POST',
            body: JSON.stringify({ force_node: 'Monitoring' })
          });
          document.getElementById('gr-force-node').value = '';
          await refreshAll(false);
          setCardStatus(
            'action-toast',
            '🔁 LangGraph Reset to Monitoring — ' + res.summary,
            'ok'
          );
        } catch (e) {
          setCardStatus('action-toast', '❌ Graph reset error: ' + e.message, 'danger');
        }
      });
    }

    async function seedSensitiveFile(btn) {
      return withButtonFeedback(btn, '🌱 Seeding Asset...', async () => {
        try {
          const targetPid = Number(document.getElementById('dlp-pid').value || 5001);
          const res = await api('/lineage/seed', {
            method: 'POST',
            body: JSON.stringify({
              path: document.getElementById('dlp-path').value,
              sha256: document.getElementById('dlp-sha').value,
              classification: document.getElementById('dlp-class').value,
              policy: document.getElementById('dlp-policy').value,
              pid: targetPid
            })
          });
          await refreshLineage(false);
          setCardStatus(
            'card3-status',
            '🌱 Seeded Sensitive Asset: ' + res.path + ' [' + res.classification + ' / ' + res.policy + '] and attached W3C Sticky Taint Baggage to Egress PID ' + targetPid,
            'ok'
          );
        } catch (e) {
          setCardStatus('card3-status', '❌ Lineage seed error: ' + e.message, 'danger');
        }
      });
    }

    async function seedDemoAssets(btn) {
      return withButtonFeedback(btn, '📦 Seeding Demo Set...', async () => {
        try {
          const targetPid = Number(document.getElementById('dlp-pid').value || 5001);
          await api('/lineage/seed', {
            method: 'POST',
            body: JSON.stringify({
              path: 'C:\\Users\\Alice\\Documents\\secret_roadmap.docx',
              sha256: '8f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a90',
              classification: 'CrownJewel',
              policy: 'Quarantine',
              pid: targetPid
            })
          });
          await api('/lineage/seed', {
            method: 'POST',
            body: JSON.stringify({
              path: 'C:\\Users\\Alice\\Documents\\q4_financials.xlsx',
              sha256: '4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c4d2e7a908f3a9b1c',
              classification: 'Confidential',
              policy: 'Audit',
              pid: targetPid
            })
          });
          await refreshLineage(false);
          setCardStatus(
            'card3-status',
            '📦 Seeded CrownJewel (secret_roadmap.docx) + Confidential (q4_financials.xlsx) into EndpointLineageEngine and tainted Egress PID ' + targetPid + '. Now click "Evaluate Inline Egress Verdict"!',
            'ok'
          );
        } catch (e) {
          setCardStatus('card3-status', '❌ Demo seed error: ' + e.message, 'danger');
        }
      });
    }

    async function evaluateLineageEgress(btn) {
      return withButtonFeedback(btn, '🛡️ Evaluating Egress...', async () => {
        try {
          const pid = Number(document.getElementById('dlp-pid').value || 5001);
          const url = document.getElementById('dlp-url').value;
          const res = await api('/lineage/evaluate', {
            method: 'POST',
            body: JSON.stringify({
              pid: pid,
              destination_url: url
            })
          });
          await refreshAll(false);
          if (res.is_block) {
            setCardStatus(
              'card3-status',
              '🛑 EGRESS BLOCKED (100% DLP Voter) — PID ' + pid + ' -> ' + url + '\nClassification: ' + res.classification + ' | Policy: ' + res.policy + ' | Reason: ' + res.reason + '\nSource Hashes: ' + (res.source_hashes || []).join(', '),
              'danger'
            );
          } else {
            setCardStatus(
              'card3-status',
              '🟢 EGRESS ALLOWED — PID ' + pid + ' -> ' + url + ' (' + (res.reason || 'No sensitive taint baggage') + ')',
              'ok'
            );
          }
        } catch (e) {
          setCardStatus('card3-status', '❌ Lineage evaluate error: ' + e.message, 'danger');
        }
      });
    }

    async function scanLiveHostSensor(btn) {
      return withButtonFeedback(btn, '🔍 Scanning Host...', async () => {
        try {
          const res = await api('/edr/ingest', {
            method: 'POST',
            body: JSON.stringify({
              event_id: 'scan-' + Date.now(),
              event_kind: 'ProcessCreate',
              pid: Number(document.getElementById('edr-pid').value || 8842),
              process_name: 'standalone_os_sensor',
              scan_live_host: true
            })
          });
          await refreshAll(false);
          setCardStatus(
            'card4-status',
            '🔍 ' + (res.summary || ('Standalone OS Sensor scanned ' + res.live_processes_scanned + ' processes.')),
            'ok'
          );
        } catch (e) {
          setCardStatus('card4-status', '❌ Standalone sensor scan error: ' + e.message, 'danger');
        }
      });
    }

    async function ingestEdrEvent(btn) {
      return withButtonFeedback(btn, '📡 Correlating...', async () => {
        try {
          const pid = Number(document.getElementById('edr-pid').value || 8842);
          const name = document.getElementById('edr-name').value || 'suspicious_agent.exe';
          const cmd = document.getElementById('edr-cmd').value || name;
          const res = await api('/edr/ingest', {
            method: 'POST',
            body: JSON.stringify({
              event_id: 'evt-' + Date.now(),
              event_kind: 'ProcessCreate',
              pid: pid,
              process_name: name,
              image_path: 'C:\\Temp\\' + name,
              command_line: cmd,
              scan_live_host: false
            })
          });
          await refreshAll(false);
          setCardStatus(
            'card4-status',
            '📡 ' + (res.summary || ('Correlated PID ' + pid + ' (' + name + ') — Score: ' + res.correlator_score)),
            res.threat_signal_emitted ? 'warn' : 'ok'
          );
        } catch (e) {
          setCardStatus('card4-status', '❌ Telemetry ingest error: ' + e.message, 'danger');
        }
      });
    }

    async function controlService(action, btn) {
      return withButtonFeedback(btn, '⏳ Updating Service...', async () => {
        try {
          if (action === 'status') {
            const sv = await api('/service/status');
            await refreshServiceAndHistory(false);
            setCardStatus(
              'card5-status',
              '🔍 Service Status: ' + sv.name + ' — State: ' + sv.state + ' | Mode: ' + (sv.is_installed ? 'Installed OS Service' : 'Standalone Self-Contained Agent (No External EDR Required)'),
              'ok'
            );
            return;
          }
          const res = await api('/service/control', {
            method: 'POST',
            body: JSON.stringify({ action: action, service_name: 'killswitch' })
          });
          await refreshAll(false);
          setCardStatus(
            'card5-status',
            '✅ Service Action "' + action + '" completed — Resulting State: ' + res.resulting_status.state + ' (Host containment disarmed safely)',
            'ok'
          );
        } catch (e) {
          setCardStatus('card5-status', '❌ Service control error: ' + e.message, 'danger');
        }
      });
    }

    refreshAll(false);
    setInterval(() => refreshAll(false), 5000);
  </script>
</body>
</html>
"#;
