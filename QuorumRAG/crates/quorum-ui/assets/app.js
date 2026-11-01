// QuorumRAG Web UI Application Controller

(function() {
  // State
  let currentModel = 'AutoFusion';
  let currentSearchMode = 'Hybrid';
  let isGenerating = false;
  let reindexPollInterval = null;

  // DOM Elements
  const themeSelector = document.getElementById('theme-selector');
  const modelPills = document.querySelectorAll('.model-pill');
  const searchPills = document.querySelectorAll('.search-pill');
  const promptInput = document.getElementById('prompt-input');
  const chatForm = document.getElementById('chat-form');
  const btnSend = document.getElementById('btn-send');
  const chatStream = document.getElementById('chat-stream');
  const welcomeBanner = document.getElementById('welcome-banner');
  const btnToggleSidebar = document.getElementById('btn-toggle-sidebar');
  const btnMobileSidebar = document.getElementById('btn-mobile-sidebar');
  const sidebar = document.getElementById('sidebar');
  const btnNewChat = document.getElementById('btn-new-chat');
  const telemetryBar = document.getElementById('telemetry-bar');
  const telemetryModelInfo = document.getElementById('telemetry-model-info');
  const hwStatus = document.getElementById('hardware-status');

  // Modal Elements
  const btnOpenReindex = document.getElementById('btn-open-reindex');
  const btnCloseReindex = document.getElementById('btn-close-reindex');
  const modalReindex = document.getElementById('modal-reindex');
  const btnStartReindex = document.getElementById('btn-start-reindex');
  const btnCancelReindex = document.getElementById('btn-cancel-reindex');
  const reindexProgressBar = document.getElementById('reindex-progress-bar');
  const reindexProgressText = document.getElementById('reindex-progress-text');
  const reindexProgressBox = document.getElementById('reindex-progress-box');
  const reindexStatusPill = document.getElementById('reindex-status-pill');

  const btnOpenDrift = document.getElementById('btn-open-drift');
  const btnCloseDrift = document.getElementById('btn-close-drift');
  const modalDrift = document.getElementById('modal-drift');
  const btnRunDrift = document.getElementById('btn-run-drift');
  const driftDryRun = document.getElementById('drift-dry-run');
  const driftResultsBox = document.getElementById('drift-results-box');
  const driftReportBody = document.getElementById('drift-report-body');

  // Cache DOM Elements
  const btnQuickRecache = document.getElementById('btn-quick-recache');
  const cacheStatsBadge = document.getElementById('cache-stats-badge');
  const cacheBadgeText = document.getElementById('cache-badge-text');
  const btnOpenRecache = document.getElementById('btn-open-recache');
  const btnCloseRecache = document.getElementById('btn-close-recache');
  const modalRecache = document.getElementById('modal-recache');
  const btnExecuteRecache = document.getElementById('btn-execute-recache');
  const recacheClear = document.getElementById('recache-clear');
  const recacheWarm = document.getElementById('recache-warm');
  const cacheStatusPill = document.getElementById('cache-status-pill');
  const recacheResultBox = document.getElementById('recache-result-box');
  const recacheResultText = document.getElementById('recache-result-text');

  // 1. THEME SWITCHER
  themeSelector.addEventListener('change', (e) => {
    document.body.className = e.target.value;
    localStorage.setItem('quorumrag_theme', e.target.value);
  });

  const savedTheme = localStorage.getItem('quorumrag_theme');
  if (savedTheme) {
    document.body.className = savedTheme;
    themeSelector.value = savedTheme;
  }

  // 2. MODEL SELECTOR
  modelPills.forEach(pill => {
    pill.addEventListener('click', () => {
      modelPills.forEach(p => p.classList.remove('active'));
      pill.classList.add('active');
      currentModel = pill.dataset.model;
    });
  });

  // 3. SEARCH MODE TOGGLE
  searchPills.forEach(pill => {
    pill.addEventListener('click', () => {
      searchPills.forEach(p => p.classList.remove('active'));
      pill.classList.add('active');
      currentSearchMode = pill.dataset.mode;
    });
  });

  // 4. SIDEBAR TOGGLE
  if (btnToggleSidebar) {
    btnToggleSidebar.addEventListener('click', () => {
      sidebar.classList.toggle('collapsed');
    });
  }
  if (btnMobileSidebar) {
    btnMobileSidebar.addEventListener('click', () => {
      sidebar.classList.toggle('collapsed');
    });
  }

  // 5. NEW CHAT
  btnNewChat.addEventListener('click', () => {
    chatStream.innerHTML = '';
    welcomeBanner.classList.remove('hidden');
    telemetryBar.classList.add('hidden');
    resetTelemetryPills();
  });

  // 6. QUICK PROMPTS
  document.querySelectorAll('.prompt-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      promptInput.value = chip.dataset.prompt;
      submitPrompt();
    });
  });

  // 7. INPUT AUTO-RESIZE & KEYBOARD HANDLING
  promptInput.addEventListener('input', () => {
    promptInput.style.height = 'auto';
    promptInput.style.height = Math.min(promptInput.scrollHeight, 160) + 'px';
  });

  promptInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submitPrompt();
    }
  });

  chatForm.addEventListener('submit', (e) => {
    e.preventDefault();
    submitPrompt();
  });

  // PROBE HARDWARE ON LOAD
  fetch('/api/health')
    .then(r => r.json())
    .then(data => {
      if (data.hardware && hwStatus) {
        hwStatus.textContent = `RAM: ${data.hardware.free_ram_gb.toFixed(1)}GB free | Model: ${data.active_model_family}`;
      }
    })
    .catch(() => {});

  // 8. SEND PROMPT & SSE CONSUMER
  async function submitPrompt() {
    const text = promptInput.value.trim();
    if (!text || isGenerating) return;

    // Check for @extract slash command
    if (text.startsWith('@extract ')) {
      const codeToExtract = text.substring(9).trim();
      handleExtractCommand(codeToExtract);
      promptInput.value = '';
      promptInput.style.height = 'auto';
      return;
    }

    isGenerating = true;
    btnSend.disabled = true;
    welcomeBanner.classList.add('hidden');
    telemetryBar.classList.remove('hidden');
    resetTelemetryPills();

    // Append User Message
    appendMessage('user', text);
    promptInput.value = '';
    promptInput.style.height = 'auto';

    // Create Assistant Message with typing cursor
    const { msgEl, bodyEl, citationsEl, headerEl } = createAssistantMessage();

    try {
      const resp = await fetch('/api/chat', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'text/event-stream',
        },
        body: JSON.stringify({
          prompt: text,
          search_mode: currentSearchMode,
          model_family: currentModel,
          top_k: 8,
          access_tier: 0
        })
      });

      if (!resp.ok) {
        throw new Error(`Server returned status ${resp.status}`);
      }

      const reader = resp.body.getReader();
      const decoder = new TextDecoder('utf-8');
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop(); // keep partial trailing line

        for (const line of lines) {
          const trimmed = line.trim();
          if (trimmed.startsWith('data:')) {
            const jsonStr = trimmed.substring(5).trim();
            if (jsonStr) {
              try {
                const event = JSON.parse(jsonStr);
                handleTelemetryEvent(event, bodyEl, citationsEl, headerEl);
              } catch (err) {
                console.error('Failed to parse SSE event:', jsonStr, err);
              }
            }
          }
        }
      }

    } catch (err) {
      bodyEl.innerHTML += `<div style="color:var(--danger);">Error: ${err.message}</div>`;
    } finally {
      // Remove typing cursor
      const cursor = bodyEl.querySelector('.typing-cursor');
      if (cursor) cursor.remove();
      isGenerating = false;
      btnSend.disabled = false;
    }
  }

  function handleTelemetryEvent(event, bodyEl, citationsEl, headerEl) {
    const type = event.type || event.event;

    if (type === 'cache_telemetry' || type === 'CacheTelemetry') {
      const data = event.data || event;
      if (data.cache_hit) {
        if (headerEl && !headerEl.querySelector('.cache-hit-pill')) {
          const cachePill = document.createElement('span');
          cachePill.className = 'cache-hit-pill';
          cachePill.textContent = `⚡ Cache Hit (Epoch ${data.cache_epoch})`;
          headerEl.appendChild(cachePill);
        }
      }
      refreshCacheStats();
    } else if (type === 'StateTransition') {
      const fromState = event.from_state;
      const toState = event.to_state;
      updateTelemetryStep(toState);
    } else if (type === 'HardwareProbed') {
      if (telemetryModelInfo) {
        telemetryModelInfo.textContent = `HW: ${event.data.free_ram_gb}GB free RAM | ${event.data.selected_tier}`;
      }
    } else if (type === 'IntentClassified') {
      if (telemetryModelInfo) {
        telemetryModelInfo.textContent = `Model: ${event.model_name || event.data.model_name} | Mode: ${event.search_mode || event.data.search_mode}`;
      }
    } else if (type === 'TokenDelta' || type === 'token') {
      const delta = event.delta || event.content || '';
      appendToken(bodyEl, delta);
    } else if (type === 'CitationEmitted' || type === 'citation') {
      const cit = event.citation || event.data.citation;
      if (cit) {
        appendCitation(citationsEl, cit);
      }
    } else if (type === 'ExecutionFinished' || type === 'done') {
      markTelemetryComplete();
    } else if (type === 'ExecutionError') {
      bodyEl.innerHTML += `<div style="color:var(--danger); margin-top:8px;">[Error: ${event.error_message || 'Execution error'}]</div>`;
    }
  }

  function appendToken(bodyEl, token) {
    let cursor = bodyEl.querySelector('.typing-cursor');
    if (!cursor) {
      cursor = document.createElement('span');
      cursor.className = 'typing-cursor';
      cursor.textContent = '▍';
      bodyEl.appendChild(cursor);
    }
    const textNode = document.createTextNode(token);
    bodyEl.insertBefore(textNode, cursor);
    scrollToBottom();
  }

  function appendCitation(citationsEl, cit) {
    const badge = document.createElement('a');
    badge.className = 'citation-badge';
    badge.href = '#';
    badge.title = `Score: ${cit.score.toFixed(3)}\n${cit.snippet || ''}`;
    badge.innerHTML = `📄 [${cit.peer_id}:${cit.file_path}:${cit.line_start}-${cit.line_end}]`;
    badge.addEventListener('click', (e) => {
      e.preventDefault();
      alert(`Source Citation:\nFile: ${cit.file_path}\nLines: ${cit.line_start}-${cit.line_end}\nPeer: ${cit.peer_id}\nSnippet:\n${cit.snippet || '(no snippet)'}`);
    });
    citationsEl.appendChild(badge);
  }

  function updateTelemetryStep(state) {
    const s = String(state).toLowerCase();
    if (s.includes('classify')) {
      setActiveStep('step-classify');
    } else if (s.includes('retrieve')) {
      setCompletedStep('step-classify');
      setActiveStep('step-retrieve');
    } else if (s.includes('grade')) {
      setCompletedStep('step-retrieve');
      setActiveStep('step-grade');
    } else if (s.includes('synthesize')) {
      setCompletedStep('step-grade');
      setActiveStep('step-synthesize');
    }
  }

  function setActiveStep(id) {
    const el = document.getElementById(id);
    if (el) {
      el.classList.remove('completed');
      el.classList.add('active');
    }
  }

  function setCompletedStep(id) {
    const el = document.getElementById(id);
    if (el) {
      el.classList.remove('active');
      el.classList.add('completed');
    }
  }

  function resetTelemetryPills() {
    ['step-classify', 'step-retrieve', 'step-grade', 'step-synthesize'].forEach(id => {
      const el = document.getElementById(id);
      if (el) el.classList.remove('active', 'completed');
    });
  }

  function markTelemetryComplete() {
    ['step-classify', 'step-retrieve', 'step-grade', 'step-synthesize'].forEach(id => {
      const el = document.getElementById(id);
      if (el) {
        el.classList.remove('active');
        el.classList.add('completed');
      }
    });
  }

  function appendMessage(role, text) {
    const msgDiv = document.createElement('div');
    msgDiv.className = `message message-${role}`;
    msgDiv.textContent = text;
    chatStream.appendChild(msgDiv);
    scrollToBottom();
  }

  function createAssistantMessage() {
    const msgDiv = document.createElement('div');
    msgDiv.className = 'message message-assistant';

    const header = document.createElement('div');
    header.className = 'message-header';
    header.innerHTML = `<span>🤖 QuorumRAG</span><span style="opacity:0.6;">(Agentic State Graph)</span>`;

    const body = document.createElement('div');
    body.className = 'message-body';

    const citations = document.createElement('div');
    citations.className = 'citation-container';

    msgDiv.appendChild(header);
    msgDiv.appendChild(body);
    msgDiv.appendChild(citations);
    chatStream.appendChild(msgDiv);
    scrollToBottom();

    return { msgEl: msgDiv, bodyEl: body, citationsEl: citations, headerEl: header };
  }

  function scrollToBottom() {
    const container = document.getElementById('messages-container');
    container.scrollTop = container.scrollHeight;
  }

  // 9. @extract COMMAND HANDLER
  async function handleExtractCommand(text) {
    appendMessage('user', `@extract ${text}`);
    const { bodyEl } = createAssistantMessage();
    bodyEl.innerHTML = `<em>Running Google LangExtract 4-Arm Consensus...</em>`;

    try {
      const res = await fetch('/api/langextract', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ text })
      });
      const data = await res.json();
      bodyEl.innerHTML = `
        <div style="background:var(--bg-surface); border:1px solid var(--border-color); border-radius:8px; padding:12px;">
          <h4>🔬 Google LangExtract Results (Consensus: ${data.consensus_passed ? 'PASSED' : 'REJECTED'})</h4>
          <p>Confidence: ${(data.fused_confidence_bp / 100).toFixed(1)}% | Verified Spans: ${data.verified_extractions.length}</p>
          <pre style="margin-top:8px;">${JSON.stringify(data.verified_extractions, null, 2)}</pre>
          <div style="margin-top:10px;">
            <a href="/assets/langextract.html" target="_blank" class="btn-primary" style="display:inline-block; text-decoration:none; padding:4px 8px; font-size:12px;">Open Interactive Visualizer</a>
          </div>
        </div>
      `;
    } catch (e) {
      bodyEl.innerHTML = `<span style="color:var(--danger);">Extraction failed: ${e.message}</span>`;
    }
  }

  // 10. REINDEX MODAL LOGIC
  btnOpenReindex.addEventListener('click', () => {
    modalReindex.classList.remove('hidden');
    checkReindexStatus();
  });
  btnCloseReindex.addEventListener('click', () => modalReindex.classList.add('hidden'));

  btnStartReindex.addEventListener('click', async () => {
    const targetPath = document.getElementById('reindex-target-path').value.trim();
    const full = document.getElementById('reindex-full').checked;
    const forceEmbed = document.getElementById('reindex-force-embed').checked;

    reindexProgressBox.classList.remove('hidden');
    btnCancelReindex.classList.remove('hidden');
    btnStartReindex.disabled = true;

    const payload = {
      full_rebuild: full,
      force_reembed: forceEmbed
    };
    if (targetPath) {
      payload.target_paths = [targetPath];
    }

    try {
      const resp = await fetch('/api/reindex', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
      const data = await resp.json();
      reindexProgressText.textContent = `Task started: ${data.task_id || 'active'}`;
      reindexStatusPill.className = 'status-pill active';
      reindexStatusPill.textContent = 'Active';

      pollReindexProgress();
    } catch (e) {
      reindexProgressText.textContent = `Error: ${e.message}`;
    }
  });

  btnCancelReindex.addEventListener('click', async () => {
    await fetch('/api/reindex/cancel', { method: 'POST' });
    reindexProgressText.textContent = 'Cancellation requested...';
  });

  function pollReindexProgress() {
    if (reindexPollInterval) clearInterval(reindexPollInterval);
    reindexPollInterval = setInterval(checkReindexStatus, 500);
  }

  async function checkReindexStatus() {
    try {
      const r = await fetch('/api/reindex/status');
      const data = await r.json();
      if (data.phase) {
        reindexProgressBox.classList.remove('hidden');
        reindexProgressText.textContent = `Phase: ${data.phase} | Scanned: ${data.files_scanned || 0} | Indexed: ${data.files_indexed || 0} | Chunks: ${data.chunks_newly_embedded || 0}`;
        if (data.phase === 'completed' || data.phase === 'failed' || data.phase === 'cancelled') {
          if (reindexPollInterval) clearInterval(reindexPollInterval);
          btnStartReindex.disabled = false;
          btnCancelReindex.classList.add('hidden');
          reindexStatusPill.className = 'status-pill idle';
          reindexStatusPill.textContent = 'Idle';
          reindexProgressBar.style.width = '100%';
        } else {
          reindexProgressBar.style.width = '60%';
        }
      }
    } catch (_) {}
  }

  // 11. DRIFT MODAL LOGIC
  btnOpenDrift.addEventListener('click', () => modalDrift.classList.remove('hidden'));
  btnCloseDrift.addEventListener('click', () => modalDrift.classList.add('hidden'));

  btnRunDrift.addEventListener('click', async () => {
    btnRunDrift.disabled = true;
    btnRunDrift.textContent = 'Remediating...';
    driftResultsBox.classList.remove('hidden');
    driftReportBody.innerHTML = '<tr><td colspan="2">Scanning 5 drift dimensions...</td></tr>';

    try {
      const resp = await fetch('/api/drift/fix', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ dry_run: driftDryRun.checked })
      });
      const rep = await resp.json();
      driftReportBody.innerHTML = `
        <tr><td><strong>Merkle State:</strong></td><td>Orphaned purged: ${rep.orphaned_db_files_purged}, Missing indexed: ${rep.missing_fs_files_indexed}</td></tr>
        <tr><td><strong>Hybrid Desync:</strong></td><td>Missing in Tantivy: ${rep.lance_chunks_missing_in_tantivy}, Missing in Lance: ${rep.tantivy_chunks_missing_in_lance} (Parity restored: ${rep.hybrid_parity_restored})</td></tr>
        <tr><td><strong>Vector Contract:</strong></td><td>Re-embedded: ${rep.contract_drifted_chunks_reembedded}</td></tr>
        <tr><td><strong>ACL Policy:</strong></td><td>Updated: ${rep.acl_drifted_chunks_updated}</td></tr>
        <tr><td><strong>Storage Compaction:</strong></td><td>Fragments consolidated: ${rep.fragments_consolidated}, Tantivy merged: ${rep.tantivy_segments_merged}</td></tr>
        <tr style="font-weight:bold; border-top:1px solid var(--border-color);"><td>Total Anomalies Healed:</td><td>${rep.total_anomalies_healed}</td></tr>
      `;
    } catch (e) {
      driftReportBody.innerHTML = `<tr><td colspan="2" style="color:var(--danger);">Error: ${e.message}</td></tr>`;
    } finally {
      btnRunDrift.disabled = false;
      btnRunDrift.textContent = 'Run Drift Remediation';
    }
  });

  // 12. CACHE & RE-CACHE LOGIC
  async function refreshCacheStats() {
    try {
      const resp = await fetch('/api/cache/stats');
      if (!resp.ok) return;
      const data = await resp.json();

      const pct = (data.hit_rate * 100).toFixed(1);
      if (cacheBadgeText) {
        cacheBadgeText.textContent = `Cache: ${pct}% (${data.hits} hits)`;
      }
      if (cacheStatusPill) {
        cacheStatusPill.textContent = `${data.hits} hits`;
        cacheStatusPill.className = data.hits > 0 ? 'status-pill healthy' : 'status-pill idle';
      }

      const epochEl = document.getElementById('recache-epoch-val');
      const hitsEl = document.getElementById('recache-hits-val');
      const missesEl = document.getElementById('recache-misses-val');
      const rateEl = document.getElementById('recache-hitrate-val');
      const qEl = document.getElementById('recache-queries-val');
      const eEl = document.getElementById('recache-embeddings-val');

      if (epochEl) epochEl.textContent = data.cache_epoch;
      if (hitsEl) hitsEl.textContent = data.hits;
      if (missesEl) missesEl.textContent = data.misses;
      if (rateEl) rateEl.textContent = `${pct}%`;
      if (qEl) qEl.textContent = data.cached_queries;
      if (eEl) eEl.textContent = data.cached_embeddings;
    } catch (_) {}
  }

  // Poll cache stats on load and every 10 seconds
  refreshCacheStats();
  setInterval(refreshCacheStats, 10000);

  // Quick Re-Cache from Top Header
  if (btnQuickRecache) {
    btnQuickRecache.addEventListener('click', async () => {
      btnQuickRecache.classList.add('loading');
      btnQuickRecache.innerHTML = `<span class="icon">⏳</span> <span>Re-Caching...</span>`;
      try {
        await fetch('/api/cache/recache', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ clear: false, warm: true })
        });
        await refreshCacheStats();
        btnQuickRecache.innerHTML = `<span class="icon">✓</span> <span>Re-Cached!</span>`;
        setTimeout(() => {
          btnQuickRecache.innerHTML = `<span class="icon">⚡</span> <span>Re-Cache</span>`;
          btnQuickRecache.classList.remove('loading');
        }, 1500);
      } catch (err) {
        btnQuickRecache.innerHTML = `<span class="icon">⚠️</span> <span>Error</span>`;
        setTimeout(() => {
          btnQuickRecache.innerHTML = `<span class="icon">⚡</span> <span>Re-Cache</span>`;
          btnQuickRecache.classList.remove('loading');
        }, 2000);
      }
    });
  }

  // Modal Open/Close
  if (btnOpenRecache) {
    btnOpenRecache.addEventListener('click', () => {
      modalRecache.classList.remove('hidden');
      refreshCacheStats();
    });
  }
  if (btnCloseRecache) {
    btnCloseRecache.addEventListener('click', () => modalRecache.classList.add('hidden'));
  }

  // Modal Execute Re-Cache
  if (btnExecuteRecache) {
    btnExecuteRecache.addEventListener('click', async () => {
      btnExecuteRecache.disabled = true;
      btnExecuteRecache.textContent = 'Executing...';
      recacheResultBox.classList.remove('hidden');
      recacheResultText.textContent = 'Purging stale entries and pre-warming...';

      try {
        const resp = await fetch('/api/cache/recache', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            clear: recacheClear.checked,
            warm: recacheWarm.checked
          })
        });
        const result = await resp.json();
        recacheResultText.innerHTML = `<strong>Success:</strong> Cache Epoch bumped to <strong>${result.cache_epoch}</strong>. Cleared ${result.cleared_entries} entries, pre-warmed ${result.warmed_entries} entries in ${result.duration_ms}ms.`;
        await refreshCacheStats();
      } catch (e) {
        recacheResultText.innerHTML = `<span style="color:var(--danger)">Error: ${e.message}</span>`;
      } finally {
        btnExecuteRecache.disabled = false;
        btnExecuteRecache.textContent = 'Execute Re-Cache';
      }
    });
  }

})();
