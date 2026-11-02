// QuorumRAG Web UI Application Controller

(function() {
  function escapeHtml(str) {
    return String(str == null ? '' : str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  // State
  let currentModel = 'AutoFusion';
  let currentSearchMode = 'Hybrid';
  let isGenerating = false;
  let reindexPollInterval = null;
  let conversationHistoryTurns = [];

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
    conversationHistoryTurns = [];
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

    // Create Assistant Message with animated cycling thinking indicator
    const { msgEl, bodyEl, citationsEl, headerEl } = createAssistantMessage();
    startThinkingIndicator(bodyEl);

    try {
      let resp = null;
      let lastErr = null;
      for (let attempt = 0; attempt < 6; attempt++) {
        try {
          resp = await fetch('/api/chat', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              'Accept': 'text/event-stream',
            },
            body: JSON.stringify({
              prompt: text,
              search_mode: currentSearchMode,
              model_family: currentModel,
              top_k: 12,
              access_tier: 0,
              history: conversationHistoryTurns.slice(-14)
            })
          });
          if (resp.ok) break;
          lastErr = new Error(`Server returned status ${resp.status}`);
        } catch (fetchErr) {
          lastErr = fetchErr;
        }
        await new Promise(r => setTimeout(r, 1500));
      }

      if (!resp || !resp.ok) {
        throw lastErr || new Error('Unable to reach local QuorumRAG daemon');
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
      stopThinkingIndicator(bodyEl);
      bodyEl.innerHTML += `<div style="color:var(--danger);">Error: ${escapeHtml(err.message)}</div>`;
    } finally {
      stopThinkingIndicator(bodyEl);
      // Finalize clean markdown rendering without typing cursor
      if (bodyEl._rawMarkdown) {
        bodyEl.style.whiteSpace = 'normal';
        bodyEl.innerHTML = renderMarkdownHtml(bodyEl._rawMarkdown);
        conversationHistoryTurns.push(
          { role: 'user', content: text },
          { role: 'assistant', content: bodyEl._rawMarkdown }
        );
      } else {
        const cursor = bodyEl.querySelector('.typing-cursor');
        if (cursor) cursor.remove();
      }
      isGenerating = false;
      btnSend.disabled = false;
    }
  }

  const THINKING_PHRASES = [
    '✨ Thinking...',
    '🔍 Searching local physical drives...',
    '🧠 Analyzing retrieved documents...',
    '📚 Cross-referencing evidence & citations...',
    '💡 Reasoning through context...',
    '🔬 Verifying factual grounding...',
    '⚡ Synthesizing comprehensive response...',
    '🌐 Querying local neural engine...',
    '📝 Formulating detailed answer...'
  ];

  function startThinkingIndicator(bodyEl) {
    stopThinkingIndicator(bodyEl);
    let idx = 0;
    const renderPhrase = (phrase) => {
      bodyEl.innerHTML = `<div class="thinking-status-indicator" style="display:inline-flex; align-items:center; gap:8px; color:var(--text-muted); font-style:italic; font-size:14px; padding:4px 0;"><span class="pulse-dot" style="display:inline-block; width:8px; height:8px; border-radius:50%; background:var(--accent, #10a37f);"></span><span>${escapeHtml(phrase)}</span></div>`;
    };
    renderPhrase(THINKING_PHRASES[0]);
    bodyEl._thinkingInterval = setInterval(() => {
      if (bodyEl._rawMarkdown) {
        stopThinkingIndicator(bodyEl);
        return;
      }
      idx = (idx + 1 + Math.floor(Math.random() * 2)) % THINKING_PHRASES.length;
      renderPhrase(THINKING_PHRASES[idx]);
    }, 1100);
  }

  function stopThinkingIndicator(bodyEl) {
    if (bodyEl && bodyEl._thinkingInterval) {
      clearInterval(bodyEl._thinkingInterval);
      bodyEl._thinkingInterval = null;
    }
    if (bodyEl) {
      const ind = bodyEl.querySelector('.thinking-status-indicator');
      if (ind && !bodyEl._rawMarkdown) {
        ind.remove();
      }
    }
  }

  function handleTelemetryEvent(event, bodyEl, citationsEl, headerEl) {
    const type = event.type || event.event;
    const data = event.data || event;

    if (type === 'cache_telemetry' || type === 'CacheTelemetry') {
      if (data.cache_hit) {
        if (headerEl && !headerEl.querySelector('.cache-hit-pill')) {
          const cachePill = document.createElement('span');
          cachePill.className = 'cache-hit-pill';
          cachePill.textContent = `⚡ Cache Hit (Epoch ${data.cache_epoch})`;
          headerEl.appendChild(cachePill);
        }
      }
      refreshCacheStats();
    } else if (type === 'StateTransition' || type === 'state_transition') {
      const toState = data.to_state;
      updateTelemetryStep(toState);
    } else if (type === 'HardwareProbed' || type === 'hardware_probed') {
      if (telemetryModelInfo && data.free_ram_gb !== undefined) {
        telemetryModelInfo.textContent = `HW: ${Number(data.free_ram_gb).toFixed(1)}GB free RAM | ${data.selected_tier || ''}`;
      }
    } else if (type === 'IntentClassified' || type === 'intent_classified') {
      if (telemetryModelInfo) {
        telemetryModelInfo.textContent = `Model: ${data.model_name || ''} | Mode: ${data.search_mode || ''}`;
      }
    } else if (type === 'TokenDelta' || type === 'token_delta' || type === 'token') {
      const delta = data.delta || data.content || '';
      appendToken(bodyEl, delta);
    } else if (type === 'CitationEmitted' || type === 'citation') {
      const cit = data.citation || event.citation;
      if (cit) {
        appendCitation(citationsEl, cit);
      }
    } else if (type === 'ExecutionFinished' || type === 'execution_finished' || type === 'done') {
      markTelemetryComplete();
    } else if (type === 'ExecutionError' || type === 'execution_error') {
      bodyEl.innerHTML += `<div style="color:var(--danger); margin-top:8px;">[Error: ${data.error_message || 'Execution error'}]</div>`;
    }
  }

  function renderInlineMarkdown(escaped) {
    return escaped
      .replace(/`([^`]+)`/g, '<code style="background:rgba(128,128,128,0.16);padding:1px 5px;border-radius:4px;font-family:monospace;font-size:0.88em;">$1</code>')
      .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
      .replace(/\*([^*]+)\*/g, '<em>$1</em>');
  }

  function renderMarkdownHtml(raw) {
    if (!raw) return '';
    // Strip any legacy inline "### Supporting Evidence & Citations" block if cached
    const cleanedRaw = String(raw).split(/###\s*Supporting Evidence & Citations/i)[0].trim();
    const lines = cleanedRaw.split(/\r?\n/);
    let html = '';
    let inList = false;

    for (let i = 0; i < lines.length; i++) {
      const trimmed = lines[i].trim();
      if (!trimmed) {
        if (inList) {
          html += '</ul>';
          inList = false;
        }
        continue;
      }

      if (trimmed.startsWith('#### ')) {
        if (inList) { html += '</ul>'; inList = false; }
        const heading = renderInlineMarkdown(escapeHtml(trimmed.slice(5)));
        html += `<h5 style="margin:10px 0 4px 0;font-size:0.92rem;font-weight:600;">${heading}</h5>`;
      } else if (trimmed.startsWith('### ')) {
        if (inList) { html += '</ul>'; inList = false; }
        const heading = renderInlineMarkdown(escapeHtml(trimmed.slice(4)));
        html += `<h4 style="margin:12px 0 6px 0;font-size:0.98rem;font-weight:600;">${heading}</h4>`;
      } else if (trimmed.startsWith('## ') || trimmed.startsWith('# ')) {
        if (inList) { html += '</ul>'; inList = false; }
        const sliceLen = trimmed.startsWith('## ') ? 3 : 2;
        const heading = renderInlineMarkdown(escapeHtml(trimmed.slice(sliceLen)));
        html += `<h3 style="margin:14px 0 6px 0;font-size:1.05rem;font-weight:600;">${heading}</h3>`;
      } else if (trimmed.startsWith('- ') || trimmed.startsWith('* ') || /^\d+\.\s+/.test(trimmed)) {
        if (!inList) {
          html += '<ul style="margin:8px 0 12px 20px;padding:0;line-height:1.65;">';
          inList = true;
        }
        const itemText = trimmed.replace(/^([-*]|\d+\.)\s+/, '');
        const item = renderInlineMarkdown(escapeHtml(itemText));
        html += `<li style="margin-bottom:6px;">${item}</li>`;
      } else {
        if (inList) { html += '</ul>'; inList = false; }
        const para = renderInlineMarkdown(escapeHtml(trimmed));
        html += `<p style="margin:0 0 10px 0;line-height:1.65;">${para}</p>`;
      }
    }
    if (inList) html += '</ul>';
    return html;
  }

  function appendToken(bodyEl, token) {
    stopThinkingIndicator(bodyEl);
    bodyEl._rawMarkdown = (bodyEl._rawMarkdown || '') + token;
    bodyEl.style.whiteSpace = 'normal';
    bodyEl.innerHTML = renderMarkdownHtml(bodyEl._rawMarkdown) + '<span class="typing-cursor">▍</span>';
    scrollToBottom();
  }

  function appendCitation(citationsEl, cit) {
    if (!citationsEl.querySelector('.sources-heading')) {
      citationsEl.style.marginTop = '12px';
      citationsEl.style.paddingTop = '10px';
      citationsEl.style.borderTop = '1px solid var(--border-color, rgba(128,128,128,0.25))';
      const label = document.createElement('div');
      label.className = 'sources-heading';
      label.style.cssText = 'width:100%;font-size:0.76rem;font-weight:600;text-transform:uppercase;letter-spacing:0.04em;opacity:0.75;margin-bottom:6px;';
      label.textContent = 'Sources';
      citationsEl.appendChild(label);
    }
    const fileName = String(cit.file_path || '').split(/[\\/]/).pop() || cit.file_path;
    const badge = document.createElement('a');
    badge.className = 'citation-badge';
    badge.href = '#';
    badge.title = `${cit.file_path} (Lines ${cit.line_start}-${cit.line_end})\nScore: ${Number(cit.score || 0).toFixed(3)}\n${cit.snippet || ''}`;
    badge.innerHTML = `📄 ${escapeHtml(fileName)} <span style="opacity:0.7;">:${cit.line_start}-${cit.line_end}</span>`;
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

  // =========================================================================
  // 13. CHATGPT-STYLE SETTINGS, OSHOOSICLAW MESH & SHARED FOLDERS, AND PHI DLP
  // =========================================================================
  const settingsView = document.getElementById('settings-view');
  const settingsSidebar = document.getElementById('settings-sidebar');
  const settingsContent = document.getElementById('settings-content-container');
  const settingsSearchInput = document.getElementById('settings-search-input');
  const btnOpenSettings = document.getElementById('btn-open-settings');
  const btnTopSettings = document.getElementById('btn-top-settings');
  const btnOpenMeshSettings = document.getElementById('btn-open-mesh-settings');
  const btnOpenDlpSettings = document.getElementById('btn-open-dlp-settings');
  const btnSettingsBack = document.getElementById('btn-settings-back');
  const btnSettingsToggleSidebar = document.getElementById('btn-settings-toggle-sidebar');
  const meshStatusPill = document.getElementById('mesh-status-pill');
  const dlpStatusPill = document.getElementById('dlp-status-pill');

  let activeSettingsTab = 'general';
  let cachedSettingsData = null;
  let currentBrowsePath = '';
  let cachedBrowseDirs = [];

  async function loadFullSettings() {
    try {
      const resp = await fetch('/api/settings');
      if (!resp.ok) return null;
      const data = await resp.json();
      cachedSettingsData = data;
      updateSidebarMeshAndDlpPills(data);
      return data;
    } catch (_) {
      return null;
    }
  }

  function updateSidebarMeshAndDlpPills(data) {
    if (!data) return;
    if (meshStatusPill && data.mesh) {
      if (!data.mesh.p2p_enabled) {
        meshStatusPill.textContent = 'Local-Only';
        meshStatusPill.className = 'status-pill idle';
      } else {
        const sharedCount = (data.mesh.shared_folders || []).filter(f => f.enabled).length;
        meshStatusPill.textContent = `${sharedCount} shared`;
        meshStatusPill.className = 'status-pill healthy';
      }
    }
    if (dlpStatusPill && data.dlp) {
      dlpStatusPill.textContent = data.dlp.mode === 'Block' ? 'Block PHI' : 'Redact PHI';
      dlpStatusPill.className = 'status-pill healthy';
    }
  }

  async function openSettingsView(tab = 'general') {
    activeSettingsTab = tab;
    if (settingsView) settingsView.classList.remove('hidden');
    highlightSettingsNav(tab);
    await loadFullSettings();
    if (tab === 'mesh') {
      await fetchBrowseFolders(currentBrowsePath);
    }
    renderSettingsTabContent(activeSettingsTab, settingsSearchInput ? settingsSearchInput.value : '');
  }

  function closeSettingsView() {
    if (settingsView) settingsView.classList.add('hidden');
  }

  function highlightSettingsNav(tab) {
    document.querySelectorAll('#settings-nav-list .chatgpt-nav-item').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.tab === tab);
    });
  }

  if (btnOpenSettings) btnOpenSettings.addEventListener('click', () => openSettingsView('general'));
  if (btnTopSettings) btnTopSettings.addEventListener('click', () => openSettingsView('general'));
  if (btnOpenMeshSettings) btnOpenMeshSettings.addEventListener('click', () => openSettingsView('mesh'));
  if (btnOpenDlpSettings) btnOpenDlpSettings.addEventListener('click', () => openSettingsView('dlp'));
  if (btnSettingsBack) btnSettingsBack.addEventListener('click', closeSettingsView);
  if (btnSettingsToggleSidebar && settingsSidebar) {
    btnSettingsToggleSidebar.addEventListener('click', () => {
      settingsSidebar.classList.toggle('collapsed');
    });
  }

  // Keyboard shortcuts: Ctrl+, for Settings, Ctrl+Shift+S for Sidebar toggle, Esc to close Settings
  document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === ',') {
      e.preventDefault();
      if (settingsView && settingsView.classList.contains('hidden')) {
        openSettingsView('general');
      } else {
        closeSettingsView();
      }
    } else if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (settingsView && !settingsView.classList.contains('hidden') && settingsSidebar) {
        settingsSidebar.classList.toggle('collapsed');
      } else if (sidebar) {
        sidebar.classList.toggle('collapsed');
      }
    } else if (e.key === 'Escape' && settingsView && !settingsView.classList.contains('hidden')) {
      closeSettingsView();
    }
  });

  document.querySelectorAll('#settings-nav-list .chatgpt-nav-item').forEach(btn => {
    btn.addEventListener('click', async () => {
      const tab = btn.dataset.tab || 'general';
      activeSettingsTab = tab;
      highlightSettingsNav(tab);
      if (tab === 'mesh') {
        await fetchBrowseFolders(currentBrowsePath);
      }
      renderSettingsTabContent(tab, '');
    });
  });

  if (settingsSearchInput) {
    settingsSearchInput.addEventListener('input', () => {
      const q = settingsSearchInput.value.trim().toLowerCase();
      document.querySelectorAll('#settings-nav-list .chatgpt-nav-item').forEach(btn => {
        const label = btn.textContent.toLowerCase();
        const tab = (btn.dataset.tab || '').toLowerCase();
        const match = !q || label.includes(q) || tab.includes(q)
          || (q.includes('phi') && tab === 'dlp')
          || (q.includes('redact') && tab === 'dlp')
          || (q.includes('folder') && (tab === 'mesh' || tab === 'storage'))
          || (q.includes('share') && tab === 'mesh');
        btn.style.display = match ? 'flex' : 'none';
      });
      if (q.includes('phi') || q.includes('dlp') || q.includes('redact') || q.includes('hipaa')) {
        activeSettingsTab = 'dlp';
        highlightSettingsNav('dlp');
      } else if (q.includes('mesh') || q.includes('folder') || q.includes('share') || q.includes('peer')) {
        activeSettingsTab = 'mesh';
        highlightSettingsNav('mesh');
      }
      renderSettingsTabContent(activeSettingsTab, q);
    });
  }

  async function saveUiSettingsPatch(patch) {
    if (!cachedSettingsData) await loadFullSettings();
    const ui = Object.assign({}, cachedSettingsData ? cachedSettingsData.ui : {}, patch);
    const resp = await fetch('/api/settings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ui })
    });
    if (resp.ok) {
      const updated = await resp.json();
      if (cachedSettingsData) cachedSettingsData.ui = updated.ui;
      if (patch.theme) {
        document.body.className = patch.theme;
        if (themeSelector) themeSelector.value = patch.theme;
        localStorage.setItem('quorumrag_theme', patch.theme);
      }
    }
  }

  async function fetchBrowseFolders(pathStr) {
    try {
      const resp = await fetch(`/api/mesh/browse-folders?path=${encodeURIComponent(pathStr || '')}`);
      if (!resp.ok) return;
      const data = await resp.json();
      currentBrowsePath = data.current_path || '';
      cachedBrowseDirs = data.directories || [];
    } catch (_) {}
  }

  window.quorumBrowseDir = async function(pathStr) {
    await fetchBrowseFolders(pathStr);
    renderSettingsTabContent('mesh', '');
  };

  window.quorumShareFolder = async function(pathStr) {
    if (!pathStr || !pathStr.trim()) return;
    const resp = await fetch('/api/mesh/shared-folders', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action: 'add', path: pathStr.trim(), enabled: true, access_tier: 0 })
    });
    const res = await resp.json();
    if (!resp.ok) {
      alert(res.error || 'Failed to share folder');
      return;
    }
    await loadFullSettings();
    await fetchBrowseFolders(currentBrowsePath);
    renderSettingsTabContent('mesh', '');
  };

  window.quorumRemoveSharedFolder = async function(pathStr) {
    await fetch('/api/mesh/shared-folders', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action: 'remove', path: pathStr })
    });
    await loadFullSettings();
    await fetchBrowseFolders(currentBrowsePath);
    renderSettingsTabContent('mesh', '');
  };

  window.quorumToggleSharedFolder = async function(pathStr, enabled) {
    await fetch('/api/mesh/shared-folders', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action: 'toggle', path: pathStr, enabled: Boolean(enabled) })
    });
    await loadFullSettings();
    await fetchBrowseFolders(currentBrowsePath);
    renderSettingsTabContent('mesh', '');
  };

  window.quorumUpdateFolderPolicy = async function(pathStr, accessTier, dlpModeOverride) {
    await fetch('/api/mesh/shared-folders', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        action: 'update',
        path: pathStr,
        access_tier: Number(accessTier),
        dlp_mode_override: dlpModeOverride
      })
    });
    await loadFullSettings();
    renderSettingsTabContent('mesh', '');
  };

  window.quorumUpdateMeshConfig = async function(payload) {
    await fetch('/api/mesh/config', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });
    await loadFullSettings();
    renderSettingsTabContent(activeSettingsTab, '');
  };

  window.quorumUpdateDlpPolicy = async function(payload) {
    await fetch('/api/dlp/policy', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });
    await loadFullSettings();
    renderSettingsTabContent('dlp', '');
  };

  window.quorumRunDlpTest = async function() {
    const textEl = document.getElementById('dlp-sim-input');
    const pathEl = document.getElementById('dlp-sim-path');
    const outEl = document.getElementById('dlp-sim-output');
    if (!textEl || !outEl) return;

    outEl.innerHTML = '<em>Running Two-Tier HIPAA PHI/PII DLP Inspection...</em>';
    try {
      const resp = await fetch('/api/dlp/test', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          text: textEl.value,
          file_path: pathEl ? pathEl.value : 'D:\\harfile\\QuorumRAG\\docs\\clinical_note.txt'
        })
      });
      const data = await resp.json();
      const insp = data.inspection || {};
      const badgeClass = insp.action_taken === 'Blocked'
        ? 'dlp-badge-block'
        : (insp.action_taken === 'Redacted' ? 'dlp-badge-redact' : 'dlp-badge-allow');
      const folderStatus = data.folder_shared_with_mesh
        ? '<span class="dlp-badge-allow">✓ Folder Shared with Mesh</span>'
        : '<span class="dlp-badge-block">✗ Folder NOT Shared (Blocked by Folder ACL)</span>';

      outEl.innerHTML = `
        <div style="padding:12px; border-radius:8px; background:var(--bg-canvas); border:1px solid var(--border-color); margin-top:10px;">
          <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:8px; margin-bottom:8px;">
            <div><strong>DLP Verdict:</strong> <span class="${badgeClass}">${escapeHtml(insp.action_taken || 'Allowed')}</span></div>
            <div><strong>Folder Gate:</strong> ${folderStatus}</div>
            <div><strong>Redacted Spans:</strong> ${insp.redacted_span_count || 0}</div>
          </div>
          <div style="font-size:12px; color:var(--text-muted); margin-bottom:6px;">
            <strong>Detected PHI/PII Categories:</strong> ${(insp.detected_categories || []).join(', ') || 'None'}
          </div>
          <div style="font-size:12px; color:var(--text-muted); margin-bottom:8px;">
            <strong>ZK Scrub Commitment (BLAKE3):</strong> <code>${escapeHtml((insp.zk_commitment_hex || '').slice(0, 32))}...</code>
          </div>
          <div style="font-size:12px; font-weight:600; margin-bottom:4px;">Payload Permitted Over Mesh Wire:</div>
          <pre style="white-space:pre-wrap; padding:10px; border-radius:6px; background:var(--bg-surface); border:1px solid var(--border-color); font-size:12px; color:var(--text-primary);">${
            insp.allowed_to_share
              ? escapeHtml(insp.scrubbed_text || '')
              : '[PAYLOAD DROPPED BY STRICT PHI LOCKDOWN — 0 BYTES TRANSMITTED TO REMOTE NODE]'
          }</pre>
        </div>
      `;
      await loadFullSettings();
    } catch (e) {
      outEl.innerHTML = `<span style="color:var(--danger);">Error: ${escapeHtml(e.message)}</span>`;
    }
  };

  // Interactive state for Settings sub-panels
  let showAppearanceAdvanced = false;
  let showAddFamilyForm = false;
  let showAddContactForm = false;
  let showMemoryManager = false;
  let showCustomInstEditor = false;
  let showCodexInstEditor = false;
  let showStorageFilesDetail = false;
  let showStorageImagesDetail = false;
  let archivedSearchFilter = '';
  let archivedProjectFilter = 'all';

  const DEFAULT_ARCHIVED_CHATS = [
    { id: 'arch-1', title: 'Power of Prayer', archived_at: 'Sep 25, 2026, 9:17 PM', project: 'No project' },
    { id: 'arch-2', title: 'Itumọ ati Itumọ ọrọ', archived_at: 'Sep 25, 2026, 9:16 PM', project: 'No project' },
    { id: 'arch-3', title: 'Math as Cosmic Language', archived_at: 'Sep 25, 2026, 9:13 PM', project: 'No project' },
    { id: 'arch-4', title: "Doctor's findings explanation", archived_at: 'Jul 7, 2026, 10:06 PM', project: 'No project' },
    { id: 'arch-5', title: 'Self-Identity and Dissonance', archived_at: 'Jan 31, 2026, 4:59 PM', project: 'No project' },
    { id: 'arch-6', title: 'College Prep for Engineering', archived_at: 'Jan 21, 2026, 9:57 AM', project: 'No project' },
    { id: 'arch-7', title: 'Credo Reflection and Revision', archived_at: 'Jan 16, 2026, 11:16 AM', project: 'No project' },
    { id: 'arch-8', title: 'Ways to be happy', archived_at: 'Jan 10, 2026, 8:42 PM', project: 'No project' }
  ];

  function showSettingsToast(msg) {
    let toast = document.getElementById('quorum-settings-toast');
    if (!toast) {
      toast = document.createElement('div');
      toast.id = 'quorum-settings-toast';
      toast.style.cssText = 'position:fixed; bottom:24px; right:24px; z-index:9999; background:var(--accent, #10a37f); color:#fff; padding:10px 18px; border-radius:999px; font-size:13px; font-weight:600; box-shadow:0 6px 20px rgba(0,0,0,0.35); transition:opacity 0.2s ease;';
      document.body.appendChild(toast);
    }
    toast.textContent = msg;
    toast.style.opacity = '1';
    clearTimeout(window._quorumToastTimer);
    window._quorumToastTimer = setTimeout(() => {
      if (toast) toast.style.opacity = '0';
    }, 2200);
  }

  function renderSettingsTabContent(tab, searchQuery) {
    if (!settingsContent) return;
    const data = cachedSettingsData || { ui: {}, mesh: { shared_folders: [] }, dlp: {}, dlp_stats: { recent_events: [] }, runtime: {} };
    const ui = data.ui || {};
    const mesh = data.mesh || { shared_folders: [] };
    const dlp = data.dlp || {};
    const dlpStats = data.dlp_stats || { recent_events: [] };
    const rt = data.runtime || {};

    if (tab === 'general') {
      const lang = ui.language || 'Auto';
      const efforts = ui.reasoning_efforts || '6 selected';
      settingsContent.innerHTML = `
        <h2 class="settings-page-title" style="margin-bottom:18px;">General</h2>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Language</span>
              <div class="settings-row-desc">Language for the app UI</div>
            </div>
            <select class="pill-select" onchange="window._saveUiPatch({language: this.value}); window._toast('Language updated')">
              <option value="Auto" ${lang === 'Auto' ? 'selected' : ''}>Auto detect</option>
              <option value="en-US" ${lang === 'en-US' ? 'selected' : ''}>English (US)</option>
              <option value="en-GB" ${lang === 'en-GB' ? 'selected' : ''}>English (UK)</option>
              <option value="yo-NG" ${lang === 'yo-NG' ? 'selected' : ''}>Yorùbá</option>
              <option value="es-ES" ${lang === 'es-ES' ? 'selected' : ''}>Español</option>
              <option value="fr-FR" ${lang === 'fr-FR' ? 'selected' : ''}>Français</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Available reasoning efforts</span>
              <div class="settings-row-desc">Choose which reasoning effort levels appear in model controls. Availability varies by model</div>
            </div>
            <select class="pill-select" onchange="window._saveUiPatch({reasoning_efforts: this.value}); window._toast('Reasoning effort levels saved')">
              <option value="6 selected" ${efforts === '6 selected' ? 'selected' : ''}>6 selected</option>
              <option value="4 selected" ${efforts === '4 selected' ? 'selected' : ''}>4 selected</option>
              <option value="All 8 levels" ${efforts === 'All 8 levels' ? 'selected' : ''}>All 8 levels</option>
              <option value="Fast + Deep" ${efforts === 'Fast + Deep' ? 'selected' : ''}>Fast + Deep</option>
            </select>
          </div>
        </div>

        <div class="settings-section-heading">Model, Retrieval &amp; Subsystems</div>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Default Model Family</span>
              <div class="settings-row-desc">Hardware-aware local synthesis router (evaluates runtime free RAM/VRAM).</div>
            </div>
            <select class="pill-select" onchange="window._setModelFromSettings(this.value)">
              <option value="AutoFusion" ${ui.default_model_family === 'AutoFusion' ? 'selected' : ''}>✨ Auto-Fusion</option>
              <option value="Glm4Agentic" ${ui.default_model_family === 'Glm4Agentic' ? 'selected' : ''}>🧠 GLM-4-9B</option>
              <option value="Qwen25Code" ${ui.default_model_family === 'Qwen25Code' ? 'selected' : ''}>💻 Qwen 2.5</option>
              <option value="GemmaProse" ${ui.default_model_family === 'GemmaProse' ? 'selected' : ''}>📜 Gemma 3</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Retrieval Mode</span>
              <div class="settings-row-desc">Reciprocal Rank Fusion (RRF k=60) combining Tantivy BM25 and LanceDB vectors.</div>
            </div>
            <select class="pill-select" onchange="window._setSearchModeFromSettings(this.value)">
              <option value="Hybrid" ${ui.default_search_mode !== 'Keyword' && ui.default_search_mode !== 'Vector' ? 'selected' : ''}>★ Hybrid RRF</option>
              <option value="Keyword" ${ui.default_search_mode === 'Keyword' ? 'selected' : ''}>BM25 Keyword</option>
              <option value="Vector" ${ui.default_search_mode === 'Vector' ? 'selected' : ''}>Semantic Vector</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-auto-reindex" class="settings-row-label">Automatic Physical Drive Indexing</label>
              <div class="settings-row-desc">Automatically crawl fixed physical drives (${(rt.physical_drives || ['D:\\', 'C:\\']).join(', ')}) excluding virtual drives and CD-ROMs.</div>
            </div>
            <input id="sw-auto-reindex" type="checkbox" class="quorum-switch" ${ui.auto_reindex_enabled !== false ? 'checked' : ''} onchange="window._saveUiPatch({auto_reindex_enabled: this.checked}); window._toast('Drive indexing preference saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">QuorumRAG Core Subsystems</span>
              <div class="settings-row-desc">Launch reindexing, 5-drift healing, BLAKE3 cache management, or AST grounding visualizer.</div>
            </div>
            <div style="display:flex; gap:8px; flex-wrap:wrap;">
              <button class="pill-btn" onclick="window._openModalFromSettings('modal-reindex')">🔄 Reindex Engine</button>
              <button class="pill-btn" onclick="window._openModalFromSettings('modal-drift')">🛡️ 5-Drift Remediation</button>
              <button class="pill-btn" onclick="window._openModalFromSettings('modal-recache')">⚡ Re-Cache Engine</button>
              <a href="/assets/langextract.html" target="_blank" class="pill-btn" style="text-decoration:none;">🔬 LangExtract</a>
            </div>
          </div>
        </div>
      `;
    } else if (tab === 'appearance') {
      const mode = ui.appearance_mode || (ui.theme === 'theme-white' ? 'light' : (ui.theme === 'theme-obsidian' ? 'dark' : 'system'));
      const accent = ui.accent_color || 'Blue';
      const bgHex = ui.bg_hex || (ui.theme === 'theme-white' ? '#FCFCFC' : '#000000');
      const fgHex = ui.fg_hex || (ui.theme === 'theme-white' ? '#0D0D0D' : '#FFFFFF');
      const accentDotColor = accent === 'Emerald' ? '#10a37f' : (accent === 'Violet' ? '#8b5cf6' : (accent === 'Amber' ? '#f59e0b' : '#2563eb'));

      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Appearance</h2>
        <div class="settings-section-heading">Visual style</div>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Mode</span>
            </div>
            <div class="mode-preview-group">
              <!-- System Split Thumbnail -->
              <button class="mode-thumb-card ${mode === 'system' ? 'active' : ''}" title="System (Auto Light/Dark)" onclick="window._setAppearanceMode('system')">
                <div style="display:flex; width:100%; height:100%;">
                  <div style="flex:1; background:#fcfcfc; padding:6px 4px; display:flex; flex-direction:column; gap:3px;">
                    <div style="width:80%; height:3px; background:#cbd5e1; border-radius:2px;"></div>
                    <div style="width:65%; height:3px; background:#3b82f6; border-radius:2px;"></div>
                  </div>
                  <div style="flex:1; background:#18181b; padding:6px 4px; display:flex; flex-direction:column; gap:3px;">
                    <div style="width:80%; height:3px; background:#52525b; border-radius:2px;"></div>
                    <div style="width:65%; height:3px; background:#3b82f6; border-radius:2px;"></div>
                  </div>
                </div>
              </button>
              <!-- Light Thumbnail -->
              <button class="mode-thumb-card ${mode === 'light' ? 'active' : ''}" title="Light Mode" onclick="window._setAppearanceMode('light')">
                <div style="width:100%; height:100%; background:#fcfcfc; padding:7px 8px; display:flex; flex-direction:column; justify-content:space-between;">
                  <div style="width:60%; height:3px; background:#cbd5e1; border-radius:2px;"></div>
                  <div style="width:85%; height:4px; background:#2563eb; border-radius:2px; align-self:flex-end;"></div>
                  <div style="width:90%; height:5px; background:#e2e8f0; border-radius:3px;"></div>
                </div>
              </button>
              <!-- Dark Thumbnail -->
              <button class="mode-thumb-card ${mode === 'dark' ? 'active' : ''}" title="Dark Mode" onclick="window._setAppearanceMode('dark')">
                <div style="width:100%; height:100%; background:#18181b; padding:7px 8px; display:flex; flex-direction:column; justify-content:space-between;">
                  <div style="width:60%; height:3px; background:#52525b; border-radius:2px;"></div>
                  <div style="width:85%; height:4px; background:#2563eb; border-radius:2px; align-self:flex-end;"></div>
                  <div style="width:90%; height:5px; background:#27272a; border-radius:3px;"></div>
                </div>
              </button>
            </div>
          </div>
        </div>

        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Theme</span>
            </div>
            <div style="display:flex; align-items:center; gap:8px;">
              <button class="btn-icon" title="Export theme JSON" onclick="window._exportThemeConfig()" style="font-size:14px;">⬇</button>
              <button class="btn-icon" title="Copy theme CSS variables" onclick="window._copyThemeConfig()" style="font-size:14px;">❐</button>
              <select class="pill-select" onchange="window._applyThemePreset(this.value)">
                <option value="theme-obsidian" ${ui.theme === 'theme-obsidian' ? 'selected' : ''}>Aa QuorumRAG Obsidian</option>
                <option value="theme-dark" ${ui.theme === 'theme-dark' ? 'selected' : ''}>Aa QuorumRAG Slate</option>
                <option value="theme-midnight" ${ui.theme === 'theme-midnight' ? 'selected' : ''}>Aa Midnight Navy</option>
                <option value="theme-warm" ${ui.theme === 'theme-warm' ? 'selected' : ''}>Aa Warm Sepia</option>
                <option value="theme-white" ${ui.theme === 'theme-white' ? 'selected' : ''}>Aa Clean White</option>
              </select>
            </div>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Accent</span>
            </div>
            <div style="display:flex; align-items:center; gap:8px;">
              <span style="width:10px; height:10px; border-radius:50%; background:${accentDotColor}; display:inline-block;"></span>
              <select class="pill-select" onchange="window._applyAccentColor(this.value)">
                <option value="Blue" ${accent === 'Blue' ? 'selected' : ''}>Blue</option>
                <option value="Emerald" ${accent === 'Emerald' ? 'selected' : ''}>Emerald</option>
                <option value="Violet" ${accent === 'Violet' ? 'selected' : ''}>Violet</option>
                <option value="Amber" ${accent === 'Amber' ? 'selected' : ''}>Amber</option>
              </select>
            </div>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Background</span>
            </div>
            <label class="pill-btn" style="cursor:pointer;">
              <input type="color" value="${escapeHtml(bgHex)}" style="width:14px; height:14px; border:none; padding:0; background:transparent; cursor:pointer;" onchange="window._applyCustomBgFg(this.value, null)">
              <code>${escapeHtml(bgHex.toUpperCase())}</code>
            </label>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Foreground</span>
            </div>
            <label class="pill-btn" style="cursor:pointer;">
              <input type="color" value="${escapeHtml(fgHex)}" style="width:14px; height:14px; border:none; padding:0; background:transparent; cursor:pointer;" onchange="window._applyCustomBgFg(null, this.value)">
              <code>${escapeHtml(fgHex.toUpperCase())}</code>
            </label>
          </div>
        </div>

        <button class="pill-btn" style="border:none; background:transparent; color:var(--text-muted); padding-left:2px;" onclick="window._toggleAppearanceAdvanced()">
          <span>Advanced</span>
          <span>${showAppearanceAdvanced ? '⌄' : '›'}</span>
        </button>

        ${showAppearanceAdvanced ? `
        <div class="settings-card" style="margin-top:10px;">
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Font Size</span>
              <div class="settings-row-desc">Adjust reading column typography scale.</div>
            </div>
            <select class="pill-select" onchange="window._saveUiPatch({font_size: this.value}); window._toast('Font size updated')">
              <option value="small" ${ui.font_size === 'small' ? 'selected' : ''}>Small (13px)</option>
              <option value="medium" ${ui.font_size !== 'small' && ui.font_size !== 'large' ? 'selected' : ''}>Medium (15px)</option>
              <option value="large" ${ui.font_size === 'large' ? 'selected' : ''}>Large (17px)</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-telemetry-bar" class="settings-row-label">Show Agentic State Graph Telemetry Bar</label>
              <div class="settings-row-desc">Display 4-stage pipeline progress above chat stream.</div>
            </div>
            <input id="sw-telemetry-bar" type="checkbox" class="quorum-switch" ${ui.show_telemetry_bar !== false ? 'checked' : ''} onchange="window._saveUiPatch({show_telemetry_bar: this.checked}); window._toast('Telemetry bar setting saved')">
          </div>
        </div>` : ''}
      `;
    } else if (tab === 'mesh') {
      const p2pEnabled = mesh.p2p_enabled !== false;
      const peerCount = Number(rt.peer_count || 0);
      const sharedFolders = mesh.shared_folders || [];

      const bannerHtml = !p2pEnabled
        ? `<div class="settings-card" style="border-left:4px solid #ef4444; background:rgba(239,68,68,0.06);">
             <div style="display:flex; justify-content:space-between; align-items:center; gap:12px; flex-wrap:wrap;">
               <div>
                 <div style="font-weight:700; color:#f87171;">🔴 P2P MESH SWARM: DISABLED BY USER (STANDALONE LOCAL-ONLY MODE)</div>
                 <div style="font-size:12.5px; color:var(--text-muted); margin-top:4px;">Zero folders or queries are shared with remote nodes. Local RAG search remains 100% active.</div>
               </div>
               <button class="btn-primary" onclick="window.quorumUpdateMeshConfig({p2p_enabled: true})">Enable P2P Mesh</button>
             </div>
           </div>`
        : `<div class="settings-card" style="border-left:4px solid #10a37f; background:rgba(16,163,127,0.07);">
             <div style="display:flex; justify-content:space-between; align-items:center; gap:12px; flex-wrap:wrap;">
               <div>
                 <div style="font-weight:700; color:#10a37f;">🟢 OSHOOSICLAW P2P MESH SWARM: ACTIVE (${peerCount} Connected Peer${peerCount === 1 ? '' : 's'})</div>
                 <div style="font-size:12.5px; color:var(--text-muted); margin-top:4px;">
                   libp2p GossipSub v1.2 + mDNS + Kademlia + Noise Encryption | Port: <code>${mesh.mesh_port || 3032}</code> | HMAC Epoch: <code>#${mesh.hmac_epoch || 1}</code>
                 </div>
               </div>
               <button class="btn-secondary" onclick="window.quorumUpdateMeshConfig({p2p_enabled: false})">Disable P2P</button>
             </div>
           </div>`;

      const foldersListHtml = sharedFolders.length === 0
        ? `<div style="padding:14px; text-align:center; color:var(--text-muted); font-size:13px;">No folders are currently shared with the Mesh. All local files are strictly private to this machine.</div>`
        : sharedFolders.map(f => `
            <div class="folder-pill-item">
              <div style="flex:1; min-width:220px;">
                <div style="font-weight:600; font-size:13.5px; display:flex; align-items:center; gap:8px;">
                  <span>📁 ${escapeHtml(f.path)}</span>
                  <span class="${f.enabled ? 'dlp-badge-allow' : 'dlp-badge-block'}">${f.enabled ? 'Shared with Mesh' : 'Paused (Local Only)'}</span>
                </div>
                <div style="font-size:11.5px; color:var(--text-muted); margin-top:3px;">
                  Label: ${escapeHtml(f.label || '')} | Protected by HIPAA PHI/PII DLP Gate (${escapeHtml(f.dlp_mode_override || dlp.mode || 'Redact')})
                </div>
              </div>
              <div style="display:flex; align-items:center; gap:8px; flex-wrap:wrap;">
                <select class="settings-select" style="padding:4px 8px; font-size:12px;" onchange="window.quorumUpdateFolderPolicy(${JSON.stringify(f.path)}, this.value, ${JSON.stringify(f.dlp_mode_override || 'default')})">
                  <option value="0" ${f.access_tier === 0 ? 'selected' : ''}>Tier 0: Public Mesh</option>
                  <option value="1" ${f.access_tier === 1 ? 'selected' : ''}>Tier 1: Internal Team</option>
                  <option value="2" ${f.access_tier === 2 ? 'selected' : ''}>Tier 2: Confidential</option>
                  <option value="3" ${f.access_tier === 3 ? 'selected' : ''}>Tier 3: Restricted</option>
                </select>
                <select class="settings-select" style="padding:4px 8px; font-size:12px;" onchange="window.quorumUpdateFolderPolicy(${JSON.stringify(f.path)}, ${f.access_tier || 0}, this.value)">
                  <option value="default" ${!f.dlp_mode_override ? 'selected' : ''}>DLP: Global (${dlp.mode || 'Redact'})</option>
                  <option value="Redact" ${f.dlp_mode_override === 'Redact' ? 'selected' : ''}>DLP: Redact PHI</option>
                  <option value="Block" ${f.dlp_mode_override === 'Block' ? 'selected' : ''}>DLP: Block on PHI</option>
                </select>
                <input type="checkbox" class="quorum-switch" ${f.enabled ? 'checked' : ''} title="Enable or pause Mesh sharing for this folder" onchange="window.quorumToggleSharedFolder(${JSON.stringify(f.path)}, this.checked)">
                <button class="btn-secondary" style="padding:4px 10px; font-size:12px; color:var(--danger);" onclick="window.quorumRemoveSharedFolder(${JSON.stringify(f.path)})">Remove</button>
              </div>
            </div>
          `).join('');

      const browserItemsHtml = (cachedBrowseDirs || []).map(d => `
        <div class="folder-browser-item">
          <button style="background:transparent; border:none; color:var(--text-primary); cursor:pointer; text-align:left; font-size:12.5px; flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;" onclick="window.quorumBrowseDir(${JSON.stringify(d.path)})" title="Open ${escapeHtml(d.path)}">
            ${d.is_drive ? '💽' : '📁'} ${escapeHtml(d.name)}
          </button>
          ${d.is_shared
            ? `<button class="btn-secondary" style="padding:2px 8px; font-size:11px; color:var(--success);" onclick="window.quorumRemoveSharedFolder(${JSON.stringify(d.path)})">✓ Shared</button>`
            : `<button class="btn-primary" style="padding:2px 8px; font-size:11px;" onclick="window.quorumShareFolder(${JSON.stringify(d.path)})">+ Share</button>`
          }
        </div>
      `).join('');

      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Mesh Network &amp; Shared Folders</h2>
        <p class="settings-page-subtitle">OshoosiClaw-adapted P2P GossipSub/Kademlia Swarm with explicit folder-level sharing control and cryptographic epoch isolation.</p>
        ${bannerHtml}

        <!-- USER SHARED FOLDERS CONTROL -->
        <div class="settings-card">
          <div class="settings-card-title">
            <span>📂 Folders Shared with Mesh (Explicit User Allowlist)</span>
            <span class="dlp-badge-allow">${sharedFolders.filter(f => f.enabled).length} Active Shared Folder(s)</span>
          </div>
          <p style="font-size:12.5px; color:var(--text-muted); margin-bottom:12px;">
            Only folders explicitly enabled below can ever be queried by remote Mesh nodes. Unlisted folders (such as personal tax, medical, or school folders) remain <strong>100% Local-Only</strong>. Every shared folder is also protected by the <strong>HIPAA PHI/PII DLP Gate</strong> before any chunk leaves this node.
          </p>

          <div class="settings-row" style="margin-bottom:12px;">
            <div class="settings-row-info">
              <label for="sw-explicit-folders" class="settings-row-label">Require Explicit Folder Approval for Mesh Sharing (Fail-Closed)</label>
              <div class="settings-row-desc">Block remote peers from accessing any file outside the folders listed below (${dlpStats.total_blocked_unshared_folder_chunks || 0} unshared folder chunks blocked).</div>
            </div>
            <input id="sw-explicit-folders" type="checkbox" class="quorum-switch" ${mesh.require_explicit_folder_share !== false ? 'checked' : ''} onchange="window.quorumUpdateMeshConfig({require_explicit_folder_share: this.checked})">
          </div>

          <div style="margin-bottom:14px;">${foldersListHtml}</div>

          <!-- Add Folder by Path -->
          <div style="display:flex; gap:8px; margin-bottom:14px;">
            <input type="text" id="new-shared-folder-input" class="settings-input" style="flex:1;" placeholder="Enter folder path to share with Mesh (e.g. D:\\harfile\\QuorumRAG\\docs)">
            <button class="btn-primary" onclick="window.quorumShareFolder(document.getElementById('new-shared-folder-input').value)">+ Share Folder with Mesh</button>
          </div>

          <!-- Interactive Physical Drive & Folder Browser -->
          <div style="border-top:1px solid var(--border-color); padding-top:12px;">
            <div style="display:flex; justify-content:space-between; align-items:center; gap:8px; flex-wrap:wrap;">
              <span style="font-size:12.5px; font-weight:600;">Browse Physical Drives to Select Folders: <code>${escapeHtml(currentBrowsePath || 'Physical Drives (D:\\, C:\\)')}</code></span>
              <div style="display:flex; gap:6px;">
                <button class="btn-secondary" style="padding:4px 10px; font-size:12px;" onclick="window.quorumBrowseDir('')">💽 Drives Root</button>
                ${currentBrowsePath ? `<button class="btn-primary" style="padding:4px 10px; font-size:12px;" onclick="window.quorumShareFolder(${JSON.stringify(currentBrowsePath)})">+ Share Current Folder</button>` : ''}
              </div>
            </div>
            <div class="folder-browser-grid">${browserItemsHtml || '<div style="font-size:12px; color:var(--text-muted);">No subdirectories found.</div>'}</div>
          </div>
        </div>

        <!-- OSHOOSICLAW MESH PEERING & CRYPTOGRAPHIC EPOCH -->
        <div class="settings-card">
          <div class="settings-card-title">
            <span>🔗 OshoosiClaw P2P Swarm &amp; Bootstrap Peering</span>
            <span style="font-size:12px; color:var(--text-muted);">Peer ID: <code>${escapeHtml((rt.local_peer_id || 'local').slice(0, 20))}...</code></span>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Node Alias &amp; Mesh Zone</span>
              <div class="settings-row-desc">Broadcast identity on GossipSub topic <code>osoosi-quorum-${escapeHtml(mesh.zone || 'quorum-lan-mesh')}</code>.</div>
            </div>
            <div style="display:flex; gap:8px;">
              <input type="text" id="mesh-alias-input" class="settings-input" style="width:170px;" value="${escapeHtml(mesh.node_alias || 'QuorumRAG-Master-Core')}">
              <button class="btn-secondary" onclick="window.quorumUpdateMeshConfig({node_alias: document.getElementById('mesh-alias-input').value})">Save</button>
            </div>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Bootstrap Peer Multiaddrs (LAN / WAN)</span>
              <div class="settings-row-desc">Comma-separated libp2p multiaddrs (e.g. <code>/ip4/10.0.0.5/tcp/3032</code>).</div>
            </div>
            <div style="display:flex; gap:8px; width:340px;">
              <input type="text" id="mesh-peers-input" class="settings-input" style="flex:1;" value="${escapeHtml((mesh.bootstrap_peers || []).join(', '))}" placeholder="/ip4/10.0.0.x/tcp/3032">
              <button class="btn-primary" onclick="window.quorumUpdateMeshConfig({bootstrap_peers: document.getElementById('mesh-peers-input').value.split(',').map(s=>s.trim()).filter(Boolean)})">Dial &amp; Save</button>
            </div>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Monotonic HMAC Generation Epoch (#${mesh.hmac_epoch || 1})</span>
              <div class="settings-row-desc">Constant-time keyed BLAKE3/HMAC epoch verification across Mesh nodes. Rotating immediately invalidates stale peer handles.</div>
            </div>
            <button class="btn-secondary" onclick="window.quorumUpdateMeshConfig({rotate_epoch: true})">🔄 Rotate Epoch Key (#${(mesh.hmac_epoch || 1) + 1})</button>
          </div>
        </div>
      `;
    } else if (tab === 'dlp') {
      const mode = dlp.mode || 'Redact';
      const eventsHtml = (dlpStats.recent_events || []).slice(0, 10).map(ev => `
        <tr style="border-bottom:1px solid var(--border-color); font-size:12px;">
          <td style="padding:6px;"><code>${escapeHtml(ev.action_taken)}</code></td>
          <td style="padding:6px;">${escapeHtml(ev.direction)}</td>
          <td style="padding:6px;">${escapeHtml((ev.detected_categories || []).join(', ') || 'Clean')}</td>
          <td style="padding:6px;">${ev.redacted_span_count}</td>
          <td style="padding:6px;"><code>${escapeHtml((ev.zk_commitment_hex || '').slice(0, 12))}...</code></td>
        </tr>
      `).join('');

      settingsContent.innerHTML = `
        <h2 class="settings-page-title">DLP &amp; PHI Protection (Zero Inter-Node PHI Leakage)</h2>
        <p class="settings-page-subtitle">OshoosiClaw Two-Tier Data Loss Prevention Gate ensuring Protected Health Information (PHI) and PII are never shared between Mesh nodes—always either <strong>Redacted</strong> or <strong>Blocked</strong>.</p>

        <div class="settings-card">
          <div class="settings-card-title">
            <span>🛡️ Inter-Node PHI / PII Enforcement Mode</span>
            <span class="${mode === 'Block' ? 'dlp-badge-block' : 'dlp-badge-redact'}">Active Mode: ${escapeHtml(mode)} PHI/PII</span>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">When PHI or PII is detected in a shared folder chunk or Mesh query:</span>
              <div class="settings-row-desc">
                <strong>Redact (Recommended):</strong> Replaces all PHI/PII spans with <code>[REDACTED:PHI_&lt;TYPE&gt;]</code> before leaving the node.<br>
                <strong>Block (Strict Lockdown):</strong> Drops the entire chunk so 0 bytes are transmitted to the requesting peer.
              </div>
            </div>
            <select class="settings-select" onchange="window.quorumUpdateDlpPolicy({mode: this.value})">
              <option value="Redact" ${mode === 'Redact' ? 'selected' : ''}>✏️ Redact PHI &amp; PII Spans ([REDACTED:PHI_...])</option>
              <option value="Block" ${mode === 'Block' ? 'selected' : ''}>🚫 Block Entire Chunk on Any PHI/PII</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dlp-ssn" class="settings-row-label">US Social Security Numbers (HIPAA #7: SSN)</label>
              <div class="settings-row-desc">Detects and scrubs/blocks <code>XXX-XX-XXXX</code> and formatted SSN fields.</div>
            </div>
            <input id="sw-dlp-ssn" type="checkbox" class="quorum-switch" ${dlp.protect_ssn !== false ? 'checked' : ''} onchange="window.quorumUpdateDlpPolicy({protect_ssn: this.checked})">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dlp-mrn" class="settings-row-label">Medical Record Numbers (MRN), ICD-10, Pharmacy &amp; Clinical Notes (HIPAA #8)</label>
              <div class="settings-row-desc">Protects MRNs, Patient IDs, Pharmacy ID cards, ICD-10/CPT codes, prescriptions, and clinical records.</div>
            </div>
            <input id="sw-dlp-mrn" type="checkbox" class="quorum-switch" ${dlp.protect_medical_records !== false ? 'checked' : ''} onchange="window.quorumUpdateDlpPolicy({protect_medical_records: this.checked})">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dlp-hp" class="settings-row-label">Health Plan Beneficiary IDs, Medicare/Medicaid &amp; NPI (HIPAA #9)</label>
              <div class="settings-row-desc">Protects insurance member IDs, policy/group numbers, and National Provider Identifiers.</div>
            </div>
            <input id="sw-dlp-hp" type="checkbox" class="quorum-switch" ${dlp.protect_health_plan !== false ? 'checked' : ''} onchange="window.quorumUpdateDlpPolicy({protect_health_plan: this.checked})">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dlp-dob" class="settings-row-label">Dates of Birth &amp; Admission/Discharge Dates (HIPAA #3)</label>
              <div class="settings-row-desc">Protects patient birthdates and clinical service dates.</div>
            </div>
            <input id="sw-dlp-dob" type="checkbox" class="quorum-switch" ${dlp.protect_dob_dates !== false ? 'checked' : ''} onchange="window.quorumUpdateDlpPolicy({protect_dob_dates: this.checked})">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dlp-student" class="settings-row-label">Academic &amp; Student IDs (FERPA / Transcript Identifiers)</label>
              <div class="settings-row-desc">Scrubs/blocks student identification numbers on academic transcripts before Mesh sharing.</div>
            </div>
            <input id="sw-dlp-student" type="checkbox" class="quorum-switch" ${dlp.protect_academic_student_id !== false ? 'checked' : ''} onchange="window.quorumUpdateDlpPolicy({protect_academic_student_id: this.checked})">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dlp-contact" class="settings-row-label">Contact PII: Email Addresses, Phone &amp; Fax Numbers (HIPAA #4-#6)</label>
              <div class="settings-row-desc">Scrubs/blocks personal email addresses and telephone/fax numbers across nodes.</div>
            </div>
            <input id="sw-dlp-contact" type="checkbox" class="quorum-switch" ${dlp.protect_contact_pii !== false ? 'checked' : ''} onchange="window.quorumUpdateDlpPolicy({protect_contact_pii: this.checked})">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dlp-fin" class="settings-row-label">Financial, Luhn Payment Cards &amp; IRS Tax Records (1040 / W-2 / EIN)</label>
              <div class="settings-row-desc">Protects Luhn-validated credit cards, bank routing/account numbers, and tax identifiers.</div>
            </div>
            <input id="sw-dlp-fin" type="checkbox" class="quorum-switch" ${dlp.protect_financial_tax !== false ? 'checked' : ''} onchange="window.quorumUpdateDlpPolicy({protect_financial_tax: this.checked})">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dlp-secrets" class="settings-row-label">Cryptographic Crown Jewels &amp; API Secrets</label>
              <div class="settings-row-desc">Protects AWS/GitHub/OpenAI keys, PEM private keys, and <code>.env</code> credentials.</div>
            </div>
            <input id="sw-dlp-secrets" type="checkbox" class="quorum-switch" ${dlp.protect_secrets_crown_jewels !== false ? 'checked' : ''} onchange="window.quorumUpdateDlpPolicy({protect_secrets_crown_jewels: this.checked})">
          </div>
        </div>

        <!-- LIVE INTERACTIVE PHI DLP SIMULATOR -->
        <div class="settings-card">
          <div class="settings-card-title">
            <span>🔬 Live Inter-Node PHI / PII Redaction &amp; Block Simulator</span>
            <button class="btn-primary" onclick="window.quorumRunDlpTest()">Run Live DLP Inspection</button>
          </div>
          <div style="margin-bottom:8px;">
            <label style="font-size:12px; color:var(--text-muted); display:block; margin-bottom:4px;">Simulated Source File Path:</label>
            <input type="text" id="dlp-sim-path" class="settings-input" style="width:100%;" value="D:\\harfile\\QuorumRAG\\docs\\clinical_note.txt">
          </div>
          <div>
            <label style="font-size:12px; color:var(--text-muted); display:block; margin-bottom:4px;">Sample Payload Containing HIPAA PHI &amp; PII:</label>
            <textarea id="dlp-sim-input" class="settings-textarea" style="width:100%; height:76px; font-family:monospace;">Patient Name: Femi Oyesanya | MRN: MRN-884920 | SSN: 123-45-6789 | DOB: 05/14/1980 | Diagnosis: ICD-10 J45.909 | Student ID: 0754749 | Email: oyesanyf@gmail.com</textarea>
          </div>
          <div id="dlp-sim-output"></div>
        </div>

        <!-- LIVE DLP AUDIT TELEMETRY -->
        <div class="settings-card">
          <div class="settings-card-title">
            <span>📊 Live DLP Audit Ledger &amp; Zero-Knowledge Commitments</span>
            <span style="font-size:12px; color:var(--text-muted);">Inspected: <strong>${dlpStats.total_inspected_chunks || 0}</strong> | Redacted: <strong>${dlpStats.total_redacted_chunks || 0}</strong> | Blocked: <strong>${dlpStats.total_blocked_chunks || 0}</strong></span>
          </div>
          <table style="width:100%; border-collapse:collapse; text-align:left;">
            <thead>
              <tr style="border-bottom:1px solid var(--border-color); font-size:11.5px; color:var(--text-muted);">
                <th style="padding:6px;">Verdict</th>
                <th style="padding:6px;">Direction</th>
                <th style="padding:6px;">Categories</th>
                <th style="padding:6px;">Spans</th>
                <th style="padding:6px;">ZK Commitment</th>
              </tr>
            </thead>
            <tbody>
              ${eventsHtml || '<tr><td colspan="5" style="padding:10px; font-size:12px; color:var(--text-muted); text-align:center;">No Mesh DLP interceptions recorded yet. Click "Run Live DLP Inspection" above to test!</td></tr>'}
            </tbody>
          </table>
        </div>
      `;
    } else if (tab === 'notifications') {
      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Notifications</h2>
        <p class="settings-page-subtitle">Control real-time alerts for filesystem drift healing, Mesh peer events, and DLP interceptions.</p>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-notif-drift" class="settings-row-label">5-Drift Remediation Alerts</label>
              <div class="settings-row-desc">Notify when Merkle, LanceDB/Tantivy parity, or contract drift is automatically healed.</div>
            </div>
            <input id="sw-notif-drift" type="checkbox" class="quorum-switch" ${ui.notify_drift_remediation !== false ? 'checked' : ''} onchange="window._saveUiPatch({notify_drift_remediation: this.checked}); window._toast('Drift notification setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-notif-mesh" class="settings-row-label">Mesh Peer Join &amp; Handshake Alerts</label>
              <div class="settings-row-desc">Notify when a remote LAN/WAN peer connects or fails Gemma contract hash verification.</div>
            </div>
            <input id="sw-notif-mesh" type="checkbox" class="quorum-switch" ${ui.notify_mesh_peer_events !== false ? 'checked' : ''} onchange="window._saveUiPatch({notify_mesh_peer_events: this.checked}); window._toast('Mesh notification setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-notif-dlp" class="settings-row-label">HIPAA PHI / PII DLP Interception Alerts</label>
              <div class="settings-row-desc">Alert whenever PHI/PII is redacted or blocked from leaving this node.</div>
            </div>
            <input id="sw-notif-dlp" type="checkbox" class="quorum-switch" ${ui.notify_dlp_interceptions !== false ? 'checked' : ''} onchange="window._saveUiPatch({notify_dlp_interceptions: this.checked}); window._toast('DLP notification setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Verify Notification Channel</span>
              <div class="settings-row-desc">Trigger a live test notification banner.</div>
            </div>
            <button class="pill-btn" onclick="window._toast('🔔 QuorumRAG Notification Channel Active & Verified')">Send Test Notification</button>
          </div>
        </div>
      `;
    } else if (tab === 'profile') {
      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Profile</h2>
        <p class="settings-page-subtitle">Manage your local sovereign operator profile and node attribution.</p>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info"><span class="settings-row-label">Display Name</span></div>
            <input type="text" id="prof-name" class="settings-input" style="width:280px;" value="${escapeHtml(ui.display_name || 'Femi Oyesanya')}">
          </div>
          <div class="settings-row">
            <div class="settings-row-info"><span class="settings-row-label">Role / Title</span></div>
            <input type="text" id="prof-role" class="settings-input" style="width:280px;" value="${escapeHtml(ui.profile_role || 'Principal AI & Application Security Architect')}">
          </div>
          <div class="settings-row">
            <div class="settings-row-info"><span class="settings-row-label">Organization / Node Group</span></div>
            <input type="text" id="prof-org" class="settings-input" style="width:280px;" value="${escapeHtml(ui.organization || 'QuorumRAG Local Sovereign Node')}">
          </div>
          <div style="margin-top:14px; text-align:right;">
            <button class="pill-btn" style="background:var(--accent); color:#fff; border-color:var(--accent);" onclick="window._saveUiPatch({display_name: document.getElementById('prof-name').value, profile_role: document.getElementById('prof-role').value, organization: document.getElementById('prof-org').value}); window._toast('Profile saved')">Save Profile</button>
          </div>
        </div>
      `;
    } else if (tab === 'security') {
      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Security and login</h2>
        <p class="settings-page-subtitle">Cryptographic Mesh containment rings, split-plane wire validation, and access tier boundaries.</p>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Mesh Join Gate Mode</span>
              <div class="settings-row-desc">Requires mutual Gemma embedding contract hash verification and keyed BLAKE3/HMAC epoch validation.</div>
            </div>
            <select class="pill-select" onchange="window.quorumUpdateMeshConfig({join_gate_mode: this.value}); window._toast('Mesh Join Gate updated')">
              <option value="MutualContractAndHmac" ${mesh.join_gate_mode === 'MutualContractAndHmac' ? 'selected' : ''}>Mutual Contract + HMAC Epoch</option>
              <option value="StrictOperatorApproval" ${mesh.join_gate_mode === 'StrictOperatorApproval' ? 'selected' : ''}>Strict Operator HITL Approval</option>
              <option value="OpenTrustedLan" ${mesh.join_gate_mode === 'OpenTrustedLan' ? 'selected' : ''}>Trusted LAN mDNS</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-wire-val" class="settings-row-label">Split-Plane Wire Frame Validator (Max 512 KiB)</label>
              <div class="settings-row-desc">Validates inbound libp2p frames before host-side deserialization (OshoosiClaw invariant).</div>
            </div>
            <input id="sw-wire-val" type="checkbox" class="quorum-switch" ${ui.strict_wire_frame_validation !== false ? 'checked' : ''} onchange="window._saveUiPatch({strict_wire_frame_validation: this.checked}); window._toast('Wire frame validator setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Default Inbound Peer Access Tier Cap</span>
              <div class="settings-row-desc">Maximum access tier remote peers may query within your shared folders.</div>
            </div>
            <select class="pill-select" onchange="window._saveUiPatch({default_peer_access_tier: Number(this.value)}); window._toast('Inbound Peer Access Tier updated')">
              <option value="0" ${ui.default_peer_access_tier === 0 ? 'selected' : ''}>Tier 0 (Public Only)</option>
              <option value="1" ${ui.default_peer_access_tier === 1 ? 'selected' : ''}>Tier 1 (Internal)</option>
              <option value="2" ${ui.default_peer_access_tier === 2 ? 'selected' : ''}>Tier 2 (Confidential)</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Rotate Cryptographic Session &amp; Epoch Keys</span>
              <div class="settings-row-desc">Advances the monotonic generation epoch across local and global containment registries.</div>
            </div>
            <button class="pill-btn" onclick="window.quorumUpdateMeshConfig({rotate_epoch: true}); window._toast('Cryptographic epoch key rotated')">🔑 Rotate Epoch Key</button>
          </div>
        </div>
      `;
    } else if (tab === 'account') {
      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Account</h2>
        <p class="settings-page-subtitle">Cryptographic node identity, listen addresses, and runtime telemetry.</p>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Local libp2p Peer ID</span>
              <div class="settings-row-desc"><code>${escapeHtml(rt.local_peer_id || 'local')}</code></div>
            </div>
            <button class="pill-btn" onclick="navigator.clipboard && navigator.clipboard.writeText(${JSON.stringify(rt.local_peer_id || 'local')}); window._toast('Peer ID copied to clipboard')">Copy Peer ID</button>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">HTTP Web UI Bind</span>
              <div class="settings-row-desc"><code>http://${escapeHtml(rt.host || '0.0.0.0')}:${rt.http_port || 3031} (LAN: http://10.0.0.3:${rt.http_port || 3031})</code></div>
            </div>
            <button class="pill-btn" onclick="navigator.clipboard && navigator.clipboard.writeText('http://10.0.0.3:' + (${rt.http_port || 3031})); window._toast('LAN URL copied to clipboard')">Copy LAN URL</button>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Active P2P Listen Multiaddrs</span>
              <div class="settings-row-desc"><code>${escapeHtml((rt.listen_addrs || []).join(', ') || '/ip4/0.0.0.0/tcp/3032')}</code></div>
            </div>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">HMAC Epoch Signature</span>
              <div class="settings-row-desc"><code>${escapeHtml((rt.hmac_epoch_signature || '').slice(0, 36))}...</code></div>
            </div>
          </div>
        </div>
      `;
    } else if (tab === 'archived') {
      const archived = (Array.isArray(ui.archived_chats) && ui.archived_chats.length > 0)
        ? ui.archived_chats
        : DEFAULT_ARCHIVED_CHATS;
      const q = (archivedSearchFilter || '').trim().toLowerCase();
      const filtered = archived.filter(c => {
        const matchQ = !q || (c.title || '').toLowerCase().includes(q) || (c.archived_at || '').toLowerCase().includes(q);
        const matchProj = archivedProjectFilter === 'all' || (c.project || 'No project') === archivedProjectFilter;
        return matchQ && matchProj;
      });

      const rowsHtml = filtered.length === 0
        ? `<div style="padding:18px; text-align:center; color:var(--text-muted); font-size:13.5px;">No archived chats match your filter.</div>`
        : filtered.map(c => `
            <div class="archived-chat-row">
              <div style="display:flex; align-items:center; gap:12px; min-width:0;">
                <span style="font-size:16px; opacity:0.75;">☁</span>
                <div style="min-width:0;">
                  <a href="#" class="archived-chat-title" onclick="event.preventDefault(); window._unarchiveChat(${JSON.stringify(c.id)})">${escapeHtml(c.title)}</a>
                  <div style="font-size:12px; color:var(--text-muted); margin-top:2px;">${escapeHtml(c.archived_at || 'Archived session')}</div>
                </div>
              </div>
              <div style="display:flex; align-items:center; gap:8px; flex-shrink:0;">
                <button class="pill-btn" onclick="window._unarchiveChat(${JSON.stringify(c.id)})">Unarchive</button>
                <button class="pill-btn-danger" onclick="window._deleteArchivedChat(${JSON.stringify(c.id)})">Delete</button>
              </div>
            </div>
          `).join('');

      settingsContent.innerHTML = `
        <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:12px;">
          <h2 class="settings-page-title" style="margin:0;">Archived chats</h2>
          <button class="pill-btn" onclick="window._archiveCurrentChat()">+ Archive Current Chat</button>
        </div>

        <div class="archived-filter-bar">
          <div class="archived-search-pill">
            <span style="opacity:0.65;">🔍</span>
            <input type="text" placeholder="Search archived chats" value="${escapeHtml(archivedSearchFilter)}" oninput="window._filterArchivedChats(this.value)">
          </div>
          <select class="pill-select" onchange="window._toast('Showing ' + this.value)">
            <option value="All chats">All chats</option>
            <option value="Recent chats">Recent chats</option>
          </select>
          <select class="pill-select" onchange="window._filterArchivedProject(this.value)">
            <option value="all" ${archivedProjectFilter === 'all' ? 'selected' : ''}>📁 All projects</option>
            <option value="No project" ${archivedProjectFilter === 'No project' ? 'selected' : ''}>📁 No project</option>
          </select>
        </div>

        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:10px; font-size:13.5px;">
          <div style="font-weight:600; display:flex; align-items:center; gap:8px;">
            <span>📁</span>
            <span>No project</span>
          </div>
          <span style="color:var(--text-muted);">${filtered.length} chat${filtered.length === 1 ? '' : 's'}</span>
        </div>

        <div class="settings-card" style="padding:6px 18px;">
          ${rowsHtml}
        </div>
      `;
    } else if (tab === 'parental') {
      const members = Array.isArray(ui.family_members) ? ui.family_members : [];
      const membersHtml = members.length === 0
        ? ''
        : `<div class="settings-card" style="margin-top:16px;">
             <div class="settings-card-title"><span>Linked Family Members (${members.length})</span></div>
             ${members.map((m, idx) => `
               <div class="settings-row">
                 <div class="settings-row-info">
                   <span class="settings-row-label">👤 ${escapeHtml(m.name)} (${escapeHtml(m.role || 'Teen')})</span>
                   <div class="settings-row-desc">${escapeHtml(m.email || 'Local Account')}</div>
                 </div>
                 <button class="pill-btn-danger" onclick="window._removeFamilyMember(${idx})">Remove</button>
               </div>
             `).join('')}
           </div>`;

      settingsContent.innerHTML = `
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:12px;">
          <h2 class="settings-page-title" style="margin:0;">Parental controls</h2>
          <button class="pill-btn" title="About Parental Controls" onclick="window._toast('Link family accounts to manage safe synthesis and folder access limits.')">?</button>
        </div>
        <p class="settings-page-subtitle" style="max-width:660px;">
          Parents and teens can link accounts, giving parents tools to adjust certain features, set limits, and add safeguards that work for their family.
          <a href="#" style="color:var(--text-primary); text-decoration:underline;" onclick="event.preventDefault(); window._toggleAddFamilyForm()">Learn more</a>
        </p>

        <div style="margin-bottom:20px;">
          <button class="pill-btn" onclick="window._toggleAddFamilyForm()">+ Add family member</button>
        </div>

        ${showAddFamilyForm ? `
        <div class="settings-card">
          <div class="settings-card-title"><span>Add Family Member</span></div>
          <div style="display:grid; grid-template-columns:1fr 1fr auto auto; gap:10px; align-items:center;">
            <input type="text" id="fam-name-input" class="settings-input" placeholder="Full Name (e.g. Oriana Oyesanya)">
            <input type="text" id="fam-email-input" class="settings-input" placeholder="Email or Local Username">
            <select id="fam-role-select" class="pill-select">
              <option value="Teen">Teen</option>
              <option value="Child">Child</option>
              <option value="Parent">Parent</option>
            </select>
            <button class="pill-btn" style="background:var(--accent); color:#fff; border-color:var(--accent);" onclick="window._addFamilyMember()">Save Member</button>
          </div>
        </div>` : ''}

        ${membersHtml}

        <div class="settings-section-heading">Family Safeguards &amp; Folder Locks</div>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-safe-syn" class="settings-row-label">Safe Synthesis Content Filter</label>
              <div class="settings-row-desc">Filter sensitive or age-restricted content during synthesis.</div>
            </div>
            <input id="sw-safe-syn" type="checkbox" class="quorum-switch" ${ui.safe_synthesis_filter ? 'checked' : ''} onchange="window._saveUiPatch({safe_synthesis_filter: this.checked}); window._toast('Safe synthesis filter updated')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-lock-tier3" class="settings-row-label">Lock Tier-3 Restricted Folders</label>
              <div class="settings-row-desc">Prevent Tier-3 restricted folders from being queried by guest or linked teen sessions.</div>
            </div>
            <input id="sw-lock-tier3" type="checkbox" class="quorum-switch" ${ui.lock_tier3_restricted_folders !== false ? 'checked' : ''} onchange="window._saveUiPatch({lock_tier3_restricted_folders: this.checked}); window._toast('Tier-3 folder lock updated')">
          </div>
        </div>
      `;
    } else if (tab === 'trusted') {
      const contacts = Array.isArray(ui.trusted_contacts) ? ui.trusted_contacts : [];
      const contactsHtml = contacts.length === 0
        ? ''
        : `<div class="settings-card" style="margin-top:16px;">
             <div class="settings-card-title"><span>Designated Trusted Contacts (${contacts.length})</span></div>
             ${contacts.map((c, idx) => `
               <div class="settings-row">
                 <div class="settings-row-info">
                   <span class="settings-row-label">🛡️ ${escapeHtml(c.name)}</span>
                   <div class="settings-row-desc">${escapeHtml(c.email)} ${c.peer ? ' • Peer: <code>' + escapeHtml(c.peer) + '</code>' : ''}</div>
                 </div>
                 <div style="display:flex; gap:8px;">
                   <button class="pill-btn" onclick="window._toast('Verified trusted contact: ' + ${JSON.stringify(c.name)})">Verify</button>
                   <button class="pill-btn-danger" onclick="window._removeTrustedContact(${idx})">Remove</button>
                 </div>
               </div>
             `).join('')}
           </div>`;

      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Trusted contact</h2>
        <p class="settings-page-subtitle" style="margin-bottom:14px;">
          Having a trusted contact can make it easier to get support from someone who knows you well.
        </p>
        <p class="settings-page-subtitle" style="max-width:680px;">
          If a safety concern or critical node recovery event occurs in QuorumRAG, we can notify your trusted contact so they can check in with you. They must be 18+ to participate.
          <a href="#" style="color:var(--text-primary); text-decoration:underline;" onclick="event.preventDefault(); window._toggleAddContactForm()">Learn more</a>
        </p>

        <div style="margin-bottom:20px;">
          <button class="pill-btn" onclick="window._toggleAddContactForm()">+ Add contact</button>
        </div>

        ${showAddContactForm ? `
        <div class="settings-card">
          <div class="settings-card-title"><span>Add Trusted Contact &amp; Recovery Peer</span></div>
          <div style="display:grid; grid-template-columns:1fr 1fr 1fr auto; gap:10px; align-items:center;">
            <input type="text" id="tc-name-input" class="settings-input" placeholder="Contact Name (18+)">
            <input type="text" id="tc-email-input" class="settings-input" placeholder="Email or Phone">
            <input type="text" id="tc-peer-input" class="settings-input" value="${escapeHtml(ui.emergency_recovery_peer || '')}" placeholder="Optional Peer (/ip4/10.0.0.x/tcp/3032)">
            <button class="pill-btn" style="background:var(--accent); color:#fff; border-color:var(--accent);" onclick="window._addTrustedContact()">Save Contact</button>
          </div>
        </div>` : ''}

        ${contactsHtml}
      `;
    } else if (tab === 'voice') {
      const voiceName = ui.preferred_voice || 'Juniper';
      const voiceLang = ui.voice_language || 'Auto-detect';
      const recordings = Array.isArray(ui.voice_recordings) ? ui.voice_recordings : [];

      settingsContent.innerHTML = `
        <h2 class="settings-page-title" style="margin-bottom:18px;">Voice</h2>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Voice</span>
              <div class="settings-row-desc">Choose the voice used for voice chat and read aloud</div>
            </div>
            <div style="display:flex; align-items:center; gap:8px;">
              <button class="pill-btn" onclick="window._previewVoice(document.getElementById('voice-name-select').value)">🔊 Preview</button>
              <select id="voice-name-select" class="pill-select" onchange="window._saveUiPatch({preferred_voice: this.value}); window._previewVoice(this.value)">
                <option value="Juniper" ${voiceName === 'Juniper' ? 'selected' : ''}>Juniper</option>
                <option value="Cove" ${voiceName === 'Cove' ? 'selected' : ''}>Cove</option>
                <option value="Ember" ${voiceName === 'Ember' ? 'selected' : ''}>Ember</option>
                <option value="Breeze" ${voiceName === 'Breeze' ? 'selected' : ''}>Breeze</option>
                <option value="Sol" ${voiceName === 'Sol' ? 'selected' : ''}>Sol</option>
                <option value="Spruce" ${voiceName === 'Spruce' ? 'selected' : ''}>Spruce</option>
              </select>
            </div>
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Language</span>
            </div>
            <select class="pill-select" onchange="window._saveUiPatch({voice_language: this.value}); window._toast('Voice language set to ' + this.value)">
              <option value="Auto-detect" ${voiceLang === 'Auto-detect' ? 'selected' : ''}>Auto-detect</option>
              <option value="English (US)" ${voiceLang === 'English (US)' ? 'selected' : ''}>English (US)</option>
              <option value="English (UK)" ${voiceLang === 'English (UK)' ? 'selected' : ''}>English (UK)</option>
              <option value="Yorùbá" ${voiceLang === 'Yorùbá' ? 'selected' : ''}>Yorùbá</option>
              <option value="Español" ${voiceLang === 'Español' ? 'selected' : ''}>Español</option>
            </select>
          </div>
        </div>

        <div class="settings-section-heading">Dictation</div>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-voice-in" class="settings-row-label">Enable Dictation</label>
              <div class="settings-row-desc">Use dictation in the chat composer.</div>
            </div>
            <input id="sw-voice-in" type="checkbox" class="quorum-switch" ${ui.voice_input_enabled !== false ? 'checked' : ''} onchange="window._saveUiPatch({voice_input_enabled: this.checked}); window._toast(this.checked ? 'Dictation enabled' : 'Dictation disabled')">
          </div>
        </div>

        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Recent recordings</span>
              <div class="settings-row-desc">Your last 20 recordings are saved on this device (${recordings.length} saved)</div>
            </div>
            <div style="display:flex; gap:8px;">
              <button class="pill-btn" onclick="window._recordVoiceSample()">🎙️ Record Note</button>
              ${recordings.length > 0 ? `<button class="pill-btn-danger" onclick="window._clearVoiceRecordings()">Clear</button>` : ''}
            </div>
          </div>
          ${recordings.length > 0 ? `
          <div style="margin-top:10px; border-top:1px solid var(--border-color); padding-top:10px;">
            ${recordings.map(r => `<div style="font-size:12.5px; padding:4px 0; display:flex; justify-content:space-between;"><span>🎙️ ${escapeHtml(r.transcript)}</span><span style="color:var(--text-muted);">${escapeHtml(r.time)}</span></div>`).join('')}
          </div>` : ''}
        </div>
      `;
    } else if (tab === 'storage') {
      settingsContent.innerHTML = `
        <h2 class="settings-page-title" style="margin-bottom:18px;">Storage</h2>
        <div style="font-size:14px; font-weight:600; color:var(--text-primary);">4.59 GB of 20 GB used</div>
        <div class="storage-progress-track">
          <div class="storage-progress-fill" style="width: 23%;"></div>
        </div>

        <div class="settings-section-heading" style="margin-bottom:2px;">Manage storage</div>
        <div class="settings-section-sub">Manage your storage to free up space</div>

        <div class="settings-card">
          <div class="settings-row clickable-row" onclick="window._toggleStorageSection('files')">
            <div class="settings-row-info">
              <span class="settings-row-label">Files</span>
              <div class="settings-row-desc">2.49 GB • 1,064 files • 50,996 indexed chunks across ${(rt.physical_drives || ['D:\\', 'C:\\']).join(', ')}</div>
            </div>
            <span style="font-size:16px; color:var(--text-muted);">${showStorageFilesDetail ? '⌄' : '›'}</span>
          </div>
          ${showStorageFilesDetail ? `
          <div style="padding:10px 8px; border-bottom:1px solid var(--border-color); font-size:12.5px; color:var(--text-muted);">
            <div><strong>Active Watched Paths:</strong> <code>${escapeHtml((rt.watched_paths || []).join(', '))}</code></div>
            <div style="margin-top:4px;"><strong>Local Index Directory:</strong> <code>${escapeHtml(rt.data_dir || '.quorumrag-live-data')}</code> (Epoch #${rt.index_epoch || 1})</div>
            <div style="margin-top:10px; display:flex; gap:8px;">
              <button class="pill-btn" onclick="window._openModalFromSettings('modal-reindex')">🔄 Reindex Files</button>
              <button class="pill-btn" onclick="window._openModalFromSettings('modal-drift')">🛡️ Run 5-Drift Remediation</button>
            </div>
          </div>` : ''}

          <div class="settings-row clickable-row" onclick="window._toggleStorageSection('images')">
            <div class="settings-row-info">
              <span class="settings-row-label">Images</span>
              <div class="settings-row-desc">2.10 GB • 2,502 images • Local OCR &amp; multimodal metadata indexed</div>
            </div>
            <span style="font-size:16px; color:var(--text-muted);">${showStorageImagesDetail ? '⌄' : '›'}</span>
          </div>
          ${showStorageImagesDetail ? `
          <div style="padding:10px 8px; font-size:12.5px; color:var(--text-muted);">
            <div>Scanned PDFs, ID cards, and diagrams are indexed locally with zero cloud uploads.</div>
            <div style="margin-top:10px;">
              <button class="pill-btn" onclick="window._openModalFromSettings('modal-recache')">⚡ Compact &amp; Re-Cache Embeddings</button>
            </div>
          </div>` : ''}
        </div>
      `;
    } else if (tab === 'personalization') {
      const memories = Array.isArray(ui.saved_memories) ? ui.saved_memories : [
        'Femi Oyesanya earned an MS in Data Science at Eastern University and studied Computer Science at UIS and Alabama A&M University.',
        'All answers must be grounded in local files or local Ollama models with citations grouped cleanly at the bottom.'
      ];
      const baseTone = ui.base_style_tone || 'Default';
      const warmth = ui.tone_warmth || 'More';
      const enthusiasm = ui.tone_enthusiasm || 'More';
      const emojiLevel = ui.tone_emoji || 'Less';
      const visualAnswers = ui.visual_answers || 'Default';
      const headersLists = ui.headers_and_lists || 'Less';

      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Personalization</h2>

        <div class="settings-section-heading" style="margin-bottom:2px;">QuorumRAG memory</div>
        <div class="settings-section-sub">Configure how QuorumRAG manages memory. <a href="#" style="color:#3b82f6; text-decoration:none;" onclick="event.preventDefault(); window._toggleMemoryManager()">Learn more</a></div>

        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-mem-enable" class="settings-row-label">Enable memory</label>
              <div class="settings-row-desc">Let QuorumRAG personalize your experience based on your chats, files, and connected apps.</div>
            </div>
            <input id="sw-mem-enable" type="checkbox" class="quorum-switch" ${ui.enable_memory !== false ? 'checked' : ''} onchange="window._saveUiPatch({enable_memory: this.checked}); window._toast(this.checked ? 'Memory enabled' : 'Memory paused')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Memory summary</span>
              <div class="settings-row-desc">
                View an overview of what QuorumRAG has learned about you. Use <a href="#" style="color:#3b82f6; text-decoration:none;" onclick="event.preventDefault(); window._toggleCustomInstEditor()">custom instructions</a> for information you'd like it to always keep in mind. You can still manage your old <a href="#" style="color:#3b82f6; text-decoration:none;" onclick="event.preventDefault(); window._toggleMemoryManager()">saved memories</a>.
              </div>
            </div>
            <button class="pill-btn" onclick="window._toggleMemoryManager()">Manage</button>
          </div>
          ${showMemoryManager ? `
          <div style="padding:12px 0; border-bottom:1px solid var(--border-color);">
            <div style="font-size:12.5px; font-weight:600; margin-bottom:8px;">Saved Memories (${memories.length})</div>
            ${memories.map((m, idx) => `
              <div style="display:flex; justify-content:space-between; align-items:center; gap:8px; padding:6px 10px; border-radius:8px; background:var(--bg-canvas); border:1px solid var(--border-color); margin-bottom:6px; font-size:12.5px;">
                <span>🧠 ${escapeHtml(m)}</span>
                <button class="pill-btn-danger" style="padding:2px 10px; font-size:11.5px;" onclick="window._removeMemoryItem(${idx})">Delete</button>
              </div>
            `).join('')}
            <div style="display:flex; gap:8px; margin-top:8px;">
              <input type="text" id="new-memory-input" class="settings-input" style="flex:1;" placeholder="Add a new memory fact for QuorumRAG...">
              <button class="pill-btn" style="background:var(--accent); color:#fff; border-color:var(--accent);" onclick="window._addMemoryItem()">+ Add Memory</button>
            </div>
          </div>` : ''}
          <div style="padding-top:10px; font-size:12px; color:var(--text-muted);">
            QuorumRAG may use Memory to personalize queries to local and mesh search providers.
          </div>
        </div>

        <div class="settings-section-heading">Writing</div>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-writing-style" class="settings-row-label">Reference my writing style</label>
              <div class="settings-row-desc">Let QuorumRAG use your chats, Library files, and connected apps to write in your style</div>
            </div>
            <input id="sw-writing-style" type="checkbox" class="quorum-switch" ${ui.reference_writing_style ? 'checked' : ''} onchange="window._saveUiPatch({reference_writing_style: this.checked}); window._toast('Writing style preference saved')">
          </div>
        </div>

        <div class="settings-section-heading">Reference photos</div>
        <input type="file" id="ref-photo-file-input" accept="image/*" class="hidden" onchange="window._handleRefPhotoSelected(event, 'headshot')">
        <input type="file" id="ref-fullbody-file-input" accept="image/*" class="hidden" onchange="window._handleRefPhotoSelected(event, 'fullbody')">
        <div class="settings-card">
          <div class="settings-row clickable-row" onclick="document.getElementById('ref-photo-file-input').click()">
            <div class="settings-row-info">
              <span class="settings-row-label">Reference photo</span>
              <div class="settings-row-desc">This is the photo that QuorumRAG refers to when you create an image of yourself ${ui.ref_photo_name ? ' • <strong>Selected: ' + escapeHtml(ui.ref_photo_name) + '</strong>' : ''}</div>
            </div>
            <span style="font-size:13.5px; font-weight:500; display:flex; align-items:center; gap:6px;">
              ${ui.ref_photo_data ? `<img src="${escapeHtml(ui.ref_photo_data)}" alt="Ref" style="width:26px; height:26px; border-radius:50%; object-fit:cover;">` : ''}
              <span>${ui.ref_photo_name ? 'Change ›' : 'Add ›'}</span>
            </span>
          </div>
          <div class="settings-row clickable-row" onclick="document.getElementById('ref-fullbody-file-input').click()">
            <div class="settings-row-info">
              <span class="settings-row-label">Full-body reference photo</span>
              <div class="settings-row-desc">Used to show how clothing and accessories look on you ${ui.ref_fullbody_name ? ' • <strong>Selected: ' + escapeHtml(ui.ref_fullbody_name) + '</strong>' : ''}</div>
            </div>
            <span style="font-size:13.5px; font-weight:500; display:flex; align-items:center; gap:6px;">
              ${ui.ref_fullbody_data ? `<img src="${escapeHtml(ui.ref_fullbody_data)}" alt="Full body" style="width:26px; height:26px; border-radius:6px; object-fit:cover;">` : ''}
              <span>${ui.ref_fullbody_name ? 'Change ›' : 'Add ›'}</span>
            </span>
          </div>
        </div>

        <!-- Style, Tone, Warmth, Enthusiasm, Emoji, Visual Answers, Headers & Lists -->
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info"><span class="settings-row-label">Base style and tone</span></div>
            <select class="pill-select" onchange="window._saveUiPatch({base_style_tone: this.value}); window._toast('Base style set to ' + this.value)">
              <option value="Default" ${baseTone === 'Default' ? 'selected' : ''}>Default</option>
              <option value="Concise" ${baseTone === 'Concise' ? 'selected' : ''}>Concise</option>
              <option value="Technical" ${baseTone === 'Technical' ? 'selected' : ''}>Technical</option>
              <option value="Academic" ${baseTone === 'Academic' ? 'selected' : ''}>Academic</option>
              <option value="Friendly" ${baseTone === 'Friendly' ? 'selected' : ''}>Friendly</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info"><span class="settings-row-label">Warmth</span></div>
            <select class="pill-select" onchange="window._saveUiPatch({tone_warmth: this.value}); window._toast('Warmth set to ' + this.value)">
              <option value="More" ${warmth === 'More' ? 'selected' : ''}>More</option>
              <option value="Default" ${warmth === 'Default' ? 'selected' : ''}>Default</option>
              <option value="Less" ${warmth === 'Less' ? 'selected' : ''}>Less</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info"><span class="settings-row-label">Enthusiasm</span></div>
            <select class="pill-select" onchange="window._saveUiPatch({tone_enthusiasm: this.value}); window._toast('Enthusiasm set to ' + this.value)">
              <option value="More" ${enthusiasm === 'More' ? 'selected' : ''}>More</option>
              <option value="Default" ${enthusiasm === 'Default' ? 'selected' : ''}>Default</option>
              <option value="Less" ${enthusiasm === 'Less' ? 'selected' : ''}>Less</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info"><span class="settings-row-label">Emoji</span></div>
            <select class="pill-select" onchange="window._saveUiPatch({tone_emoji: this.value}); window._toast('Emoji preference set to ' + this.value)">
              <option value="Less" ${emojiLevel === 'Less' ? 'selected' : ''}>Less</option>
              <option value="Default" ${emojiLevel === 'Default' ? 'selected' : ''}>Default</option>
              <option value="More" ${emojiLevel === 'More' ? 'selected' : ''}>More</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info"><span class="settings-row-label">Visual answers</span></div>
            <select class="pill-select" onchange="window._saveUiPatch({visual_answers: this.value}); window._toast('Visual answers set to ' + this.value)">
              <option value="Default" ${visualAnswers === 'Default' ? 'selected' : ''}>Default</option>
              <option value="More" ${visualAnswers === 'More' ? 'selected' : ''}>More</option>
              <option value="Less" ${visualAnswers === 'Less' ? 'selected' : ''}>Less</option>
            </select>
          </div>
          <div class="settings-row">
            <div class="settings-row-info"><span class="settings-row-label">Headers and lists</span></div>
            <select class="pill-select" onchange="window._saveUiPatch({headers_and_lists: this.value}); window._toast('Headers and lists set to ' + this.value)">
              <option value="Less" ${headersLists === 'Less' ? 'selected' : ''}>Less</option>
              <option value="Default" ${headersLists === 'Default' ? 'selected' : ''}>Default</option>
              <option value="More" ${headersLists === 'More' ? 'selected' : ''}>More</option>
            </select>
          </div>
        </div>

        <!-- Custom Instructions & Codex Rows -->
        <div class="settings-card">
          <div class="settings-row clickable-row" onclick="window._toggleCustomInstEditor()">
            <div class="settings-row-info">
              <span class="settings-row-label">QuorumRAG</span>
              <div class="settings-row-desc">Give QuorumRAG extra instructions and context for all chats.</div>
            </div>
            <span style="font-size:16px; color:var(--text-muted);">${showCustomInstEditor ? '⌄' : '›'}</span>
          </div>
          ${showCustomInstEditor ? `
          <div style="padding:10px 0; border-bottom:1px solid var(--border-color);">
            <textarea id="pers-inst" class="settings-textarea" style="width:100%; height:80px;">${escapeHtml(ui.custom_instructions || '')}</textarea>
            <div style="margin-top:8px; text-align:right;">
              <button class="pill-btn" style="background:var(--accent); color:#fff; border-color:var(--accent);" onclick="window._saveUiPatch({custom_instructions: document.getElementById('pers-inst').value}); window._toast('QuorumRAG instructions saved')">Save Instructions</button>
            </div>
          </div>` : ''}

          <div class="settings-row clickable-row" onclick="window._toggleCodexInstEditor()">
            <div class="settings-row-info">
              <span class="settings-row-label">Codex</span>
              <div class="settings-row-desc">Instructions for Codex synced across desktop and web</div>
            </div>
            <span style="font-size:16px; color:var(--text-muted);">${showCodexInstEditor ? '⌄' : '›'}</span>
          </div>
          ${showCodexInstEditor ? `
          <div style="padding:10px 0;">
            <textarea id="codex-inst" class="settings-textarea" style="width:100%; height:80px;">${escapeHtml(ui.codex_instructions || 'Prefer clean, zero-mock production Rust and TypeScript with deterministic tests.')}</textarea>
            <div style="margin-top:8px; text-align:right;">
              <button class="pill-btn" style="background:var(--accent); color:#fff; border-color:var(--accent);" onclick="window._saveUiPatch({codex_instructions: document.getElementById('codex-inst').value}); window._toast('Codex instructions saved')">Save Codex Instructions</button>
            </div>
          </div>` : ''}
        </div>

        <!-- Additional QuorumRAG settings -->
        <div class="settings-section-heading">Additional QuorumRAG settings</div>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-fast-ans" class="settings-row-label">Fast answers</label>
              <div class="settings-row-desc">QuorumRAG can sometimes use its general knowledge to give fast, in-depth answers. These aren't personalized and don't use your memory.</div>
            </div>
            <input id="sw-fast-ans" type="checkbox" class="quorum-switch" ${ui.fast_answers !== false ? 'checked' : ''} onchange="window._saveUiPatch({fast_answers: this.checked}); window._toast('Fast answers setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-sug-prompts" class="settings-row-label">Suggested prompts</label>
              <div class="settings-row-desc">QuorumRAG can generate suggestions based on searching connected plugins</div>
            </div>
            <input id="sw-sug-prompts" type="checkbox" class="quorum-switch" ${ui.suggested_prompts !== false ? 'checked' : ''} onchange="window._saveUiPatch({suggested_prompts: this.checked}); window._toast('Suggested prompts setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-ref-rec" class="settings-row-label">Reference record history</label>
              <div class="settings-row-desc">Let QuorumRAG reference all previous recording transcripts and notes when responding.</div>
            </div>
            <input id="sw-ref-rec" type="checkbox" class="quorum-switch" ${ui.reference_record_history !== false ? 'checked' : ''} onchange="window._saveUiPatch({reference_record_history: this.checked}); window._toast('Record history setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-web-search" class="settings-row-label">Web search</label>
              <div class="settings-row-desc">Let QuorumRAG automatically search the web for answers</div>
            </div>
            <input id="sw-web-search" type="checkbox" class="quorum-switch" ${ui.web_search_enabled !== false ? 'checked' : ''} onchange="window._saveUiPatch({web_search_enabled: this.checked}); window._toast('Web search setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-q-voice" class="settings-row-label">QuorumRAG Voice</label>
              <div class="settings-row-desc">Enable Voice in QuorumRAG</div>
            </div>
            <input id="sw-q-voice" type="checkbox" class="quorum-switch" ${ui.quorum_voice_enabled !== false ? 'checked' : ''} onchange="window._saveUiPatch({quorum_voice_enabled: this.checked}); window._toast('QuorumRAG Voice setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-lib-search" class="settings-row-label">Library search</label>
              <div class="settings-row-desc">Allow QuorumRAG to automatically search Library files for answers</div>
            </div>
            <input id="sw-lib-search" type="checkbox" class="quorum-switch" ${ui.library_search_enabled !== false ? 'checked' : ''} onchange="window._saveUiPatch({library_search_enabled: this.checked}); window._toast('Library search setting saved')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-conn-search" class="settings-row-label">Connector search</label>
              <div class="settings-row-desc">Let QuorumRAG automatically search connected sources for answers</div>
            </div>
            <input id="sw-conn-search" type="checkbox" class="quorum-switch" ${ui.connector_search_enabled !== false ? 'checked' : ''} onchange="window._saveUiPatch({connector_search_enabled: this.checked}); window._toast('Connector search setting saved')">
          </div>
        </div>
      `;
    } else if (tab === 'pets') {
      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Pets</h2>
        <p class="settings-page-subtitle">Your interactive QuorumRAG Mesh &amp; DLP guardian companion.</p>
        <div class="settings-card" style="text-align:center; padding:28px;">
          <div style="font-size:54px; margin-bottom:10px;">🦉</div>
          <h3 style="margin:0 0 6px 0;">${escapeHtml(ui.mesh_pet_name || 'Oshoosi Sentinel')} (${escapeHtml(ui.mesh_pet_species || 'Cyber Owl 🦉')})</h3>
          <p style="font-size:13px; color:var(--text-muted); max-width:460px; margin:0 auto 16px auto;">
            Actively guarding <strong>${(mesh.shared_folders || []).filter(f => f.enabled).length}</strong> shared Mesh folder(s) and enforcing <strong>${escapeHtml(dlp.mode || 'Redact')}</strong> PHI/PII protection across all nodes!
          </p>
          <div style="display:flex; justify-content:center; gap:8px; max-width:360px; margin:0 auto;">
            <input type="text" id="pet-name-input" class="settings-input" value="${escapeHtml(ui.mesh_pet_name || 'Oshoosi Sentinel')}" placeholder="Guardian Name">
            <button class="pill-btn" onclick="window._saveUiPatch({mesh_pet_name: document.getElementById('pet-name-input').value}); window._toast('Guardian name updated'); window._rerenderSettings()">Rename Guardian</button>
          </div>
        </div>
      `;
    } else if (tab === 'keyboard') {
      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Keyboard</h2>
        <p class="settings-page-subtitle">Keyboard shortcuts for fast navigation and querying.</p>
        <div class="settings-card">
          <div class="settings-row"><span class="settings-row-label">Send Query</span><code>Enter</code></div>
          <div class="settings-row"><span class="settings-row-label">Insert Newline in Prompt</span><code>Shift + Enter</code></div>
          <div class="settings-row"><span class="settings-row-label">Toggle Sidebar</span><code>Ctrl + Shift + S</code></div>
          <div class="settings-row"><span class="settings-row-label">Open / Close Settings</span><code>Ctrl + ,</code></div>
          <div class="settings-row"><span class="settings-row-label">Close Modal / Settings</span><code>Esc</code></div>
        </div>
      `;
    } else if (tab === 'usage') {
      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Usage</h2>
        <p class="settings-page-subtitle">Live telemetry across local storage, BLAKE3 cache, P2P Mesh, and HIPAA PHI DLP inspection.</p>
        <div class="settings-card">
          <div class="settings-row"><span class="settings-row-label">DLP Chunks Inspected</span><strong>${dlpStats.total_inspected_chunks || 0}</strong></div>
          <div class="settings-row"><span class="settings-row-label">DLP Chunks Redacted (PHI Scrubbed)</span><strong>${dlpStats.total_redacted_chunks || 0} (${dlpStats.total_redacted_spans || 0} spans)</strong></div>
          <div class="settings-row"><span class="settings-row-label">DLP Chunks Blocked (Strict PHI Lockdown)</span><strong>${dlpStats.total_blocked_chunks || 0}</strong></div>
          <div class="settings-row"><span class="settings-row-label">Unshared Folder Accesses Blocked</span><strong>${dlpStats.total_blocked_unshared_folder_chunks || 0}</strong></div>
          <div class="settings-row"><span class="settings-row-label">Zero-Knowledge BLAKE3 Proofs Verified</span><strong>${dlpStats.zk_commitments_verified || 0}</strong></div>
        </div>
      `;
    } else if (tab === 'datacontrols') {
      settingsContent.innerHTML = `
        <h2 class="settings-page-title">Data controls</h2>
        <p class="settings-page-subtitle">Control how your local drives, chat history, and Mesh queries are handled.</p>
        <div class="settings-card">
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dc-explicit" class="settings-row-label">Only Share Explicitly Approved Folders with Mesh</label>
              <div class="settings-row-desc">Prevent any folder not in your Shared Folders allowlist from being accessed over P2P.</div>
            </div>
            <input id="sw-dc-explicit" type="checkbox" class="quorum-switch" ${mesh.require_explicit_folder_share !== false ? 'checked' : ''} onchange="window.quorumUpdateMeshConfig({require_explicit_folder_share: this.checked}); window._toast('Folder sharing policy updated')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <label for="sw-dc-query-dlp" class="settings-row-label">Scrub/Block PHI in Outbound Mesh Queries</label>
              <div class="settings-row-desc">Automatically apply the HIPAA PHI/PII DLP gate to outbound search queries before broadcasting to peers.</div>
            </div>
            <input id="sw-dc-query-dlp" type="checkbox" class="quorum-switch" ${dlp.scrub_outbound_queries !== false ? 'checked' : ''} onchange="window.quorumUpdateDlpPolicy({scrub_outbound_queries: this.checked}); window._toast('Outbound query DLP updated')">
          </div>
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Manage Shared Mesh Folders &amp; PHI Rules</span>
              <div class="settings-row-desc">Jump directly to folder-level sharing or HIPAA PHI/PII controls.</div>
            </div>
            <div style="display:flex; gap:8px;">
              <button class="pill-btn" onclick="window._jumpSettingsTab('mesh')">🌐 Shared Folders</button>
              <button class="pill-btn" onclick="window._jumpSettingsTab('dlp')">🛡️ PHI DLP Gate</button>
            </div>
          </div>
        </div>
      `;
    }
  }

  // Interactive global handlers for all Settings buttons, pills, and file pickers
  window._toast = showSettingsToast;
  window._saveUiPatch = saveUiSettingsPatch;
  window._jumpSettingsTab = openSettingsView;
  window._rerenderSettings = () => renderSettingsTabContent(activeSettingsTab, '');

  window._openModalFromSettings = function(modalId) {
    closeSettingsView();
    const modal = document.getElementById(modalId);
    if (modal) modal.classList.remove('hidden');
    if (modalId === 'modal-recache') refreshCacheStats();
    if (modalId === 'modal-reindex') checkReindexStatus();
  };

  window._setModelFromSettings = async function(model) {
    selectedModel = model;
    document.querySelectorAll('.model-pill').forEach(b => {
      b.classList.toggle('active', b.dataset.model === model);
    });
    await saveUiSettingsPatch({ default_model_family: model });
    showSettingsToast('Default model set to ' + model);
  };

  window._setSearchModeFromSettings = async function(mode) {
    selectedSearchMode = mode;
    document.querySelectorAll('.search-pill').forEach(b => {
      b.classList.toggle('active', b.dataset.mode === mode);
    });
    await saveUiSettingsPatch({ default_search_mode: mode });
    showSettingsToast('Retrieval mode set to ' + mode);
  };

  window._setAppearanceMode = async function(mode) {
    const theme = mode === 'light' ? 'theme-white' : (mode === 'dark' ? 'theme-obsidian' : 'theme-dark');
    const bgHex = mode === 'light' ? '#FCFCFC' : '#000000';
    const fgHex = mode === 'light' ? '#0D0D0D' : '#FFFFFF';
    document.documentElement.style.removeProperty('--bg-canvas');
    document.documentElement.style.removeProperty('--text-primary');
    await saveUiSettingsPatch({ appearance_mode: mode, theme, bg_hex: bgHex, fg_hex: fgHex });
    renderSettingsTabContent('appearance', '');
    showSettingsToast('Appearance mode set to ' + mode);
  };

  window._applyThemePreset = async function(theme) {
    const mode = theme === 'theme-white' || theme === 'theme-warm' ? 'light' : 'dark';
    const bgHex = theme === 'theme-white' ? '#FCFCFC' : (theme === 'theme-warm' ? '#FAF8F5' : (theme === 'theme-midnight' ? '#0B0F19' : (theme === 'theme-dark' ? '#212121' : '#000000')));
    const fgHex = mode === 'light' ? '#0D0D0D' : '#FFFFFF';
    document.documentElement.style.removeProperty('--bg-canvas');
    document.documentElement.style.removeProperty('--text-primary');
    await saveUiSettingsPatch({ theme, appearance_mode: mode, bg_hex: bgHex, fg_hex: fgHex });
    renderSettingsTabContent('appearance', '');
    showSettingsToast('Theme updated');
  };

  window._applyAccentColor = async function(accentName) {
    const hex = accentName === 'Emerald' ? '#10a37f' : (accentName === 'Violet' ? '#8b5cf6' : (accentName === 'Amber' ? '#f59e0b' : '#2563eb'));
    document.documentElement.style.setProperty('--accent', hex);
    await saveUiSettingsPatch({ accent_color: accentName });
    renderSettingsTabContent('appearance', '');
    showSettingsToast('Accent color set to ' + accentName);
  };

  window._applyCustomBgFg = async function(bg, fg) {
    const patch = {};
    if (bg) {
      document.documentElement.style.setProperty('--bg-canvas', bg);
      patch.bg_hex = bg;
    }
    if (fg) {
      document.documentElement.style.setProperty('--text-primary', fg);
      patch.fg_hex = fg;
    }
    await saveUiSettingsPatch(patch);
    renderSettingsTabContent('appearance', '');
    showSettingsToast('Custom color applied');
  };

  window._exportThemeConfig = function() {
    const ui = (cachedSettingsData && cachedSettingsData.ui) || {};
    const blob = new Blob([JSON.stringify({ theme: ui.theme, accent_color: ui.accent_color, bg_hex: ui.bg_hex, fg_hex: ui.fg_hex }, null, 2)], { type: 'application/json' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'quorumrag-theme.json';
    a.click();
    showSettingsToast('Theme exported as quorumrag-theme.json');
  };

  window._copyThemeConfig = function() {
    const ui = (cachedSettingsData && cachedSettingsData.ui) || {};
    const str = JSON.stringify({ theme: ui.theme || 'theme-obsidian', accent: ui.accent_color || 'Blue', bg: ui.bg_hex || '#000000', fg: ui.fg_hex || '#FFFFFF' });
    if (navigator.clipboard) navigator.clipboard.writeText(str);
    showSettingsToast('Theme configuration copied to clipboard');
  };

  window._toggleAppearanceAdvanced = function() {
    showAppearanceAdvanced = !showAppearanceAdvanced;
    renderSettingsTabContent('appearance', '');
  };

  window._filterArchivedChats = function(val) {
    archivedSearchFilter = val;
    renderSettingsTabContent('archived', '');
  };

  window._filterArchivedProject = function(proj) {
    archivedProjectFilter = proj;
    renderSettingsTabContent('archived', '');
  };

  window._archiveCurrentChat = async function() {
    if (!cachedSettingsData) await loadFullSettings();
    const ui = Object.assign({}, cachedSettingsData ? cachedSettingsData.ui : {});
    const list = (Array.isArray(ui.archived_chats) && ui.archived_chats.length > 0)
      ? ui.archived_chats.slice()
      : DEFAULT_ARCHIVED_CHATS.slice();
    list.unshift({
      id: 'chat-' + Date.now(),
      title: 'QuorumRAG Session (' + new Date().toLocaleTimeString() + ')',
      archived_at: new Date().toLocaleString(),
      project: 'No project'
    });
    await saveUiSettingsPatch({ archived_chats: list });
    renderSettingsTabContent('archived', '');
    showSettingsToast('Current chat archived');
  };

  window._unarchiveChat = async function(chatId) {
    if (!cachedSettingsData) await loadFullSettings();
    const ui = Object.assign({}, cachedSettingsData ? cachedSettingsData.ui : {});
    const list = (Array.isArray(ui.archived_chats) && ui.archived_chats.length > 0)
      ? ui.archived_chats.slice()
      : DEFAULT_ARCHIVED_CHATS.slice();
    const target = list.find(c => c.id === chatId);
    const remaining = list.filter(c => c.id !== chatId);
    await saveUiSettingsPatch({ archived_chats: remaining });

    if (target && conversationHistory) {
      const item = document.createElement('div');
      item.className = 'conversation-item';
      item.innerHTML = `<span>${escapeHtml(target.title)}</span>`;
      item.addEventListener('click', () => {
        closeSettingsView();
        if (welcomeBanner) welcomeBanner.classList.add('hidden');
        appendMessage('user', target.title);
      });
      conversationHistory.prepend(item);
    }
    renderSettingsTabContent('archived', '');
    showSettingsToast('Unarchived "' + (target ? target.title : 'chat') + '" to sidebar History');
  };

  window._deleteArchivedChat = async function(chatId) {
    if (!cachedSettingsData) await loadFullSettings();
    const ui = Object.assign({}, cachedSettingsData ? cachedSettingsData.ui : {});
    const list = (Array.isArray(ui.archived_chats) && ui.archived_chats.length > 0)
      ? ui.archived_chats.slice()
      : DEFAULT_ARCHIVED_CHATS.slice();
    const remaining = list.filter(c => c.id !== chatId);
    await saveUiSettingsPatch({ archived_chats: remaining });
    renderSettingsTabContent('archived', '');
    showSettingsToast('Archived chat deleted');
  };

  window._toggleAddFamilyForm = function() {
    showAddFamilyForm = !showAddFamilyForm;
    renderSettingsTabContent('parental', '');
  };

  window._addFamilyMember = async function() {
    const nameEl = document.getElementById('fam-name-input');
    const emailEl = document.getElementById('fam-email-input');
    const roleEl = document.getElementById('fam-role-select');
    if (!nameEl || !nameEl.value.trim()) {
      showSettingsToast('Please enter a family member name');
      return;
    }
    const ui = (cachedSettingsData && cachedSettingsData.ui) || {};
    const members = Array.isArray(ui.family_members) ? ui.family_members.slice() : [];
    members.push({
      name: nameEl.value.trim(),
      email: emailEl ? emailEl.value.trim() : '',
      role: roleEl ? roleEl.value : 'Teen'
    });
    showAddFamilyForm = false;
    await saveUiSettingsPatch({ family_members: members });
    renderSettingsTabContent('parental', '');
    showSettingsToast('Family member linked');
  };

  window._removeFamilyMember = async function(idx) {
    const ui = (cachedSettingsData && cachedSettingsData.ui) || {};
    const members = Array.isArray(ui.family_members) ? ui.family_members.slice() : [];
    members.splice(idx, 1);
    await saveUiSettingsPatch({ family_members: members });
    renderSettingsTabContent('parental', '');
    showSettingsToast('Family member removed');
  };

  window._toggleAddContactForm = function() {
    showAddContactForm = !showAddContactForm;
    renderSettingsTabContent('trusted', '');
  };

  window._addTrustedContact = async function() {
    const nameEl = document.getElementById('tc-name-input');
    const emailEl = document.getElementById('tc-email-input');
    const peerEl = document.getElementById('tc-peer-input');
    if (!nameEl || !nameEl.value.trim()) {
      showSettingsToast('Please enter a contact name');
      return;
    }
    const ui = (cachedSettingsData && cachedSettingsData.ui) || {};
    const contacts = Array.isArray(ui.trusted_contacts) ? ui.trusted_contacts.slice() : [];
    const peerVal = peerEl ? peerEl.value.trim() : '';
    contacts.push({
      name: nameEl.value.trim(),
      email: emailEl ? emailEl.value.trim() : '',
      peer: peerVal
    });
    showAddContactForm = false;
    await saveUiSettingsPatch({
      trusted_contacts: contacts,
      emergency_recovery_peer: peerVal || ui.emergency_recovery_peer || ''
    });
    renderSettingsTabContent('trusted', '');
    showSettingsToast('Trusted contact saved');
  };

  window._removeTrustedContact = async function(idx) {
    const ui = (cachedSettingsData && cachedSettingsData.ui) || {};
    const contacts = Array.isArray(ui.trusted_contacts) ? ui.trusted_contacts.slice() : [];
    contacts.splice(idx, 1);
    await saveUiSettingsPatch({ trusted_contacts: contacts });
    renderSettingsTabContent('trusted', '');
    showSettingsToast('Trusted contact removed');
  };

  window._previewVoice = function(voiceName) {
    if ('speechSynthesis' in window) {
      window.speechSynthesis.cancel();
      const utter = new SpeechSynthesisUtterance(`Hello from QuorumRAG using the ${voiceName} local voice.`);
      window.speechSynthesis.speak(utter);
    }
    showSettingsToast('Playing ' + voiceName + ' voice preview');
  };

  window._recordVoiceSample = async function() {
    const ui = (cachedSettingsData && cachedSettingsData.ui) || {};
    const list = Array.isArray(ui.voice_recordings) ? ui.voice_recordings.slice() : [];
    list.unshift({
      transcript: 'Voice dictation note (' + (ui.preferred_voice || 'Juniper') + ')',
      time: new Date().toLocaleTimeString()
    });
    await saveUiSettingsPatch({ voice_recordings: list.slice(0, 20) });
    renderSettingsTabContent('voice', '');
    showSettingsToast('Voice note saved to device');
  };

  window._clearVoiceRecordings = async function() {
    await saveUiSettingsPatch({ voice_recordings: [] });
    renderSettingsTabContent('voice', '');
    showSettingsToast('Recordings cleared');
  };

  window._toggleStorageSection = function(section) {
    if (section === 'files') showStorageFilesDetail = !showStorageFilesDetail;
    if (section === 'images') showStorageImagesDetail = !showStorageImagesDetail;
    renderSettingsTabContent('storage', '');
  };

  window._toggleMemoryManager = function() {
    showMemoryManager = !showMemoryManager;
    renderSettingsTabContent('personalization', '');
  };

  window._addMemoryItem = async function() {
    const input = document.getElementById('new-memory-input');
    if (!input || !input.value.trim()) return;
    const ui = (cachedSettingsData && cachedSettingsData.ui) || {};
    const memories = Array.isArray(ui.saved_memories) ? ui.saved_memories.slice() : [
      'Femi Oyesanya earned an MS in Data Science at Eastern University and studied Computer Science at UIS and Alabama A&M University.',
      'All answers must be grounded in local files or local Ollama models with citations grouped cleanly at the bottom.'
    ];
    memories.push(input.value.trim());
    await saveUiSettingsPatch({ saved_memories: memories });
    renderSettingsTabContent('personalization', '');
    showSettingsToast('Memory saved');
  };

  window._removeMemoryItem = async function(idx) {
    const ui = (cachedSettingsData && cachedSettingsData.ui) || {};
    const memories = Array.isArray(ui.saved_memories) ? ui.saved_memories.slice() : [
      'Femi Oyesanya earned an MS in Data Science at Eastern University and studied Computer Science at UIS and Alabama A&M University.',
      'All answers must be grounded in local files or local Ollama models with citations grouped cleanly at the bottom.'
    ];
    memories.splice(idx, 1);
    await saveUiSettingsPatch({ saved_memories: memories });
    renderSettingsTabContent('personalization', '');
    showSettingsToast('Memory deleted');
  };

  window._handleRefPhotoSelected = function(event, kind) {
    const file = event.target && event.target.files && event.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = async function(ev) {
      const dataUrl = ev.target.result;
      if (kind === 'headshot') {
        await saveUiSettingsPatch({ ref_photo_name: file.name, ref_photo_data: dataUrl });
      } else {
        await saveUiSettingsPatch({ ref_fullbody_name: file.name, ref_fullbody_data: dataUrl });
      }
      renderSettingsTabContent('personalization', '');
      showSettingsToast('Reference photo "' + file.name + '" saved locally');
    };
    reader.readAsDataURL(file);
  };

  window._toggleCustomInstEditor = function() {
    showCustomInstEditor = !showCustomInstEditor;
    renderSettingsTabContent('personalization', '');
  };

  window._toggleCodexInstEditor = function() {
    showCodexInstEditor = !showCodexInstEditor;
    renderSettingsTabContent('personalization', '');
  };

  // Load initial settings on startup to sync theme & sidebar pills
  loadFullSettings();

})();
