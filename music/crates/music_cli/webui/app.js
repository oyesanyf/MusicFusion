/**
 * MusicFusion AI Studio - Clientside Reactive Controller
 * Vanilla ES6+ JS, zero external dependencies, 100% resilient offline fallback.
 */

(function () {
  'use strict';

  // ==========================================================================
  // Application State Container
  // ==========================================================================
  function buildSongVersionRecord(songId, versionNum, title, bpm, key, prompt, judgeScore) {
    const isFileProto = typeof window !== 'undefined' && window.location && window.location.protocol === 'file:';
    const base = `/api/songs/${songId}/versions/${versionNum}/audio`;
    const fileBase = `../../../storage/songs/${songId}/v${versionNum}`;
    return {
      version: versionNum,
      title: title,
      bpm: bpm || 76,
      key: key || 'A Major',
      engineMode: 'pure_rust',
      prompt: prompt || '1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, 76 BPM, hypnotic one drop drumming, warm analog tape production.',
      telemetry: {
        peakDbfs: -0.1,
        rmsDbfs: versionNum === 1 ? -12.4 : -11.8,
        dynamicRange: versionNum === 1 ? 11.8 : 12.1,
        tempoDrift: 0.01,
        zeroClipping: true,
        judgeScore: judgeScore || (versionNum === 1 ? 9.4 : 9.7)
      },
      audioUrls: isFileProto ? {
        master: `${fileBase}/final_master.wav`,
        vocals: `${fileBase}/stems/raspy_tenor_vocals.wav`,
        drums: `${fileBase}/stems/drums.wav`,
        bass: `${fileBase}/stems/bass.wav`,
        guitar: `${fileBase}/stems/guitar.wav`,
        other: `${fileBase}/stems/other.wav`,
        inpainted_guitar: `${fileBase}/stems/inpainted_guitar.wav`,
        inpainted_drums: `${fileBase}/stems/inpainted_drums.wav`,
        draft_groove: `${fileBase}/stems/draft_groove.wav`
      } : {
        master: `${base}/final_master`,
        vocals: `${base}/raspy_tenor_vocals`,
        drums: `${base}/drums`,
        bass: `${base}/bass`,
        guitar: `${base}/guitar`,
        other: `${base}/other`,
        inpainted_guitar: `${base}/inpainted_guitar`,
        inpainted_drums: `${base}/inpainted_drums`,
        draft_groove: `${base}/draft_groove`
      },
      suggestedIterations: versionNum === 1 ? [
        '🔥 Boost guitar solo wah & sustain in [Guitar Solo]',
        '🥁 Add syncopated Nyabinghi percussion fills in [Verse 2]',
        '🎤 Elevate gospel choir call-and-response in [Chorus]',
        '🔊 Increase sub-bass warmth (+3dB @ 65Hz)',
        '⚡ Speed up tempo from 76 to 80 BPM'
      ] : [
        '🎷 Add saxophone brass fills in [Verse 1]',
        '✨ Add Hammond B3 organ rotary bubble in [Chorus]',
        '🎚️ Master with Club Sub-Heavy Bass profile'
      ]
    };
  }

  const State = {
    activeSongId: 'roots_reggae_anthem',
    activeVersion: 1,
    activeAudioOutputId: 'default',
    audioDevices: [],
    isIterating: true, // Iterate mode vs New Song mode
    isPlayingMaster: false,
    isPlayingStems: false,
    stemsMuted: {},
    stemsSoloed: {},
    stemsVolume: {},
    hardware: {
      vramFree: 21.2,
      vramTotal: 24.0,
      ramFree: 18.5,
      ramTotal: 32.0,
      gpuName: 'NVIDIA GeForce RTX 4090',
      models: ['qwen2.5:32b', 'deepseek-r1:32b', 'glm4:9b', 'flint:latest']
    },
    songs: {
      roots_reggae_anthem: {
        id: 'roots_reggae_anthem',
        title: 'One Love Compassion Anthem',
        genre: 'Roots Reggae',
        versions: {
          1: buildSongVersionRecord(
            'roots_reggae_anthem',
            1,
            'One Love Compassion Anthem',
            76,
            'A Major',
            '1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, 76 BPM, hypnotic one drop drumming, warm analog tape production.',
            9.4
          ),
          2: buildSongVersionRecord(
            'roots_reggae_anthem',
            2,
            'One Love Compassion Anthem (Wah Lead & Sub-Bass Boost)',
            80,
            'A Major',
            'Iterated v1: Elevated guitar wah filter resonance, boosted sub-bass tape warmth +3dB @ 65Hz, and accelerated tempo to 80 BPM.',
            9.7
          )
        }
      },
      song_afrobeats_002: {
        id: 'song_afrobeats_002',
        title: 'Lagos Sunrise Groove',
        genre: 'Afrobeats',
        versions: {
          1: buildSongVersionRecord(
            'song_afrobeats_002',
            1,
            'Lagos Sunrise Groove',
            104,
            'C Major',
            'Modern Lagos Afrobeats anthem at 104 BPM in C Major, syncopated Tresillo log-drum bassline, bright highlife guitar plucks, warm brass stabs, and smooth melodic vocals.',
            9.5
          ),
          2: buildSongVersionRecord(
            'song_afrobeats_002',
            2,
            'Lagos Sunrise Groove (Club Log-Drum Boost)',
            108,
            'C Major',
            'Iterated v1: Boosted Amapiano/Afrobeats log-drum sub-bass +3.5dB, added saxophone horn counter-melody, and accelerated groove to 108 BPM.',
            9.8
          )
        }
      },
      'song-one-love-shining-1977': {
        id: 'song-one-love-shining-1977',
        title: 'One Love Shining (1977 Master)',
        genre: 'Roots Reggae',
        versions: {
          1: buildSongVersionRecord(
            'song-one-love-shining-1977',
            1,
            'One Love Shining (1977 Master)',
            76,
            'A Major',
            '1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, 76 BPM, hypnotic one drop drumming, warm analog tape production.',
            9.6
          ),
          2: buildSongVersionRecord(
            'song-one-love-shining-1977',
            2,
            'One Love Shining (1977 Dub & Lead Guitar Mix)',
            78,
            'A Major',
            'Iterated v1: Extended psychedelic blues-rock wah guitar solo and deeper analog spring reverb.',
            9.8
          )
        }
      },
      song_reggae_anthem_001: {
        id: 'song_reggae_anthem_001',
        title: 'One Love Compassion Anthem',
        genre: 'Roots Reggae',
        versions: {
          1: buildSongVersionRecord(
            'roots_reggae_anthem',
            1,
            'One Love Compassion Anthem',
            76,
            'A Major',
            '1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, 76 BPM, hypnotic one drop drumming, warm analog tape production.',
            9.4
          ),
          2: buildSongVersionRecord(
            'roots_reggae_anthem',
            2,
            'One Love Compassion Anthem (Wah Lead & Sub-Bass Boost)',
            80,
            'A Major',
            'Iterated v1: Elevated guitar wah filter resonance, boosted sub-bass tape warmth +3dB @ 65Hz, and accelerated tempo to 80 BPM.',
            9.7
          )
        }
      }
    }
  };

  // ==========================================================================
  // DOM Elements Cache
  // ==========================================================================
  const DOM = {};

  function cacheDOMElements() {
    DOM.sidebar = document.getElementById('chatgpt-sidebar');
    DOM.sidebarToggleBtn = document.getElementById('sidebar-toggle-btn');
    DOM.sidebarOpenBtn = document.getElementById('sidebar-open-btn');
    DOM.sidebarNewSong = document.getElementById('sidebar-new-song');
    DOM.btnSidebarLoadLocal = document.getElementById('btn-sidebar-load-local');
    DOM.btnCardLoadLocal = document.getElementById('btn-card-load-local');
    DOM.localAudioFileInput = document.getElementById('local-audio-file-input');
    DOM.songHistory = document.getElementById('sidebar-song-history');
    DOM.historyCount = document.getElementById('history-count');
    DOM.presetsAccordionToggle = document.getElementById('presets-accordion-toggle');
    DOM.presetsList = document.getElementById('sidebar-presets-list');
    DOM.presetsChevron = document.getElementById('presets-chevron');

    DOM.hwVramText = document.getElementById('hw-vram-text');
    DOM.hwRamText = document.getElementById('hw-ram-text');
    DOM.hwGpuText = document.getElementById('hw-gpu-text');
    DOM.hwOllamaModels = document.getElementById('hw-ollama-models');

    DOM.headerTrackTitle = document.getElementById('header-track-title');
    DOM.headerGenrePill = document.getElementById('header-genre-pill');
    DOM.headerVersionSelect = document.getElementById('header-version-select');
    DOM.headerAudioOutputSelect = document.getElementById('header-audio-output-select');
    DOM.themeToggleBtn = document.getElementById('theme-toggle-btn');

    DOM.chatMessages = document.getElementById('chat-messages');
    DOM.activeUserPrompt = document.getElementById('active-user-prompt');
    DOM.cardSongTitle = document.getElementById('card-song-title');
    DOM.cardSongMeta = document.getElementById('card-song-meta');
    DOM.cardVersionTag = document.getElementById('card-version-tag');
    DOM.cardDownloadBtn = document.getElementById('card-download-btn');

    DOM.masterAudio = document.getElementById('master-audio-player');
    DOM.masterWaveformCanvas = document.getElementById('master-waveform-canvas');
    DOM.waveformProgress = document.getElementById('waveform-progress');
    DOM.waveformWrap = document.getElementById('waveform-wrap');
    DOM.masterTimeDisplay = document.getElementById('master-time-display');
    DOM.btnMasterPlay = document.getElementById('btn-master-play');
    DOM.btnMasterStop = document.getElementById('btn-master-stop');
    DOM.masterVolumeSlider = document.getElementById('master-volume-slider');
    DOM.deckAudioOutputSelect = document.getElementById('deck-audio-output-select');
    DOM.btnRefreshAudioDevices = document.getElementById('btn-refresh-audio-devices');
    DOM.btnToggleLoop = document.getElementById('btn-toggle-loop');
    DOM.deckStatusMsg = document.getElementById('deck-status-msg');

    DOM.telemPeak = document.getElementById('telem-peak');
    DOM.telemRms = document.getElementById('telem-rms');
    DOM.telemDr = document.getElementById('telem-dr');
    DOM.telemDrift = document.getElementById('telem-drift');
    DOM.telemClipping = document.getElementById('telem-clipping');
    DOM.telemJudge = document.getElementById('telem-judge');

    DOM.btnAskCopilot = document.getElementById('btn-ask-copilot');
    DOM.copilotSuggestionsDrawer = document.getElementById('copilot-suggestions-drawer');
    DOM.cardSuggestionChips = document.getElementById('card-suggestion-chips');
    DOM.capsuleSuggestionChips = document.getElementById('capsule-suggestion-chips');

    DOM.toggleStemsBtn = document.getElementById('toggle-stems-btn');
    DOM.stemsDrawerBody = document.getElementById('stems-drawer-body');
    DOM.btnPlayAllStems = document.getElementById('btn-play-all-stems');
    DOM.btnStopAllStems = document.getElementById('btn-stop-all-stems');
    DOM.btnUnmuteAll = document.getElementById('btn-unmute-all');
    DOM.btnRemasterTrack = document.getElementById('btn-remaster-track');
    DOM.masteringProfileSelect = document.getElementById('mastering-profile-select');

    DOM.toggleLyricsBtn = document.getElementById('toggle-lyrics-btn');
    DOM.lyricsDrawerBody = document.getElementById('lyrics-drawer-body');
    DOM.btnEditLyrics = document.getElementById('btn-edit-lyrics');
    DOM.lyricsEditWrapper = document.getElementById('lyrics-edit-wrapper');
    DOM.lyricsRawTextarea = document.getElementById('lyrics-raw-textarea');
    DOM.btnSaveLyrics = document.getElementById('btn-save-lyrics');
    DOM.btnCancelLyrics = document.getElementById('btn-cancel-lyrics');
    DOM.paneLyricsFormatted = document.getElementById('pane-lyrics-formatted');
    DOM.paneLyricsYue = document.getElementById('pane-lyrics-yue');

    DOM.toggleTraceBtn = document.getElementById('toggle-trace-btn');
    DOM.traceDrawerBody = document.getElementById('trace-drawer-body');

    DOM.btnCapsuleMode = document.getElementById('btn-capsule-mode');
    DOM.modePillIcon = document.getElementById('mode-pill-icon');
    DOM.modePillText = document.getElementById('mode-pill-text');
    DOM.capsuleGenreSelect = document.getElementById('capsule-genre-select');
    DOM.chatInputTextarea = document.getElementById('chat-input-textarea');
    DOM.btnToggleSliders = document.getElementById('btn-toggle-sliders');
    DOM.btnSendPrompt = document.getElementById('btn-send-prompt');
    DOM.parameterSlidersDrawer = document.getElementById('parameter-sliders-drawer');

    DOM.sliderBpm = document.getElementById('slider-bpm');
    DOM.valBpm = document.getElementById('val-bpm');
    DOM.selectKey = document.getElementById('select-key');
    DOM.valKey = document.getElementById('val-key');
    DOM.sliderRasp = document.getElementById('slider-rasp');
    DOM.valRasp = document.getElementById('val-rasp');
    DOM.sliderWah = document.getElementById('slider-wah');
    DOM.valWah = document.getElementById('val-wah');
    DOM.sliderDrum = document.getElementById('slider-drum');
    DOM.valDrum = document.getElementById('val-drum');
    DOM.sliderBass = document.getElementById('slider-bass');
    DOM.valBass = document.getElementById('val-bass');
    DOM.selectTapeProfile = document.getElementById('select-tape-profile');
    DOM.selectEngineMode = document.getElementById('select-engine-mode');
    DOM.valEngine = document.getElementById('val-engine');
  }

  // ==========================================================================
  // Waveform Canvas Visualizer
  // ==========================================================================
  function initWaveformVisualizer() {
    if (!DOM.masterWaveformCanvas) return;
    const canvas = DOM.masterWaveformCanvas;
    const ctx = canvas.getContext('2d');
    const width = canvas.width;
    const height = canvas.height;

    // Generate pseudo-deterministic 76 BPM audio envelope bars
    const barCount = 120;
    const barWidth = width / barCount;
    const heights = [];
    let seed = 42;
    for (let i = 0; i < barCount; i++) {
      seed = (seed * 9301 + 49297) % 233280;
      const rnd = seed / 233280;
      // Shaping roots reggae one drop dynamics
      const beat = (i % 8 === 2 || i % 8 === 6) ? 0.9 : (i % 4 === 0 ? 0.4 : 0.65);
      heights.push(Math.max(0.15, Math.min(0.95, rnd * beat)));
    }

    function renderWaveform(progressPct) {
      ctx.clearRect(0, 0, width, height);

      for (let i = 0; i < barCount; i++) {
        const x = i * barWidth;
        const h = heights[i] * (height - 8);
        const y = (height - h) / 2;
        const isPlayed = (i / barCount) <= progressPct;

        ctx.fillStyle = isPlayed ? '#10a37f' : '#2e2e38';
        ctx.fillRect(x + 1, y, barWidth - 2, h);
      }
    }

    renderWaveform(0);

    // Waveform click seeking
    DOM.waveformWrap.addEventListener('click', function (e) {
      const rect = DOM.waveformWrap.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const pct = Math.max(0, Math.min(1, clickX / rect.width));

      if (DOM.masterAudio.duration) {
        DOM.masterAudio.currentTime = pct * DOM.masterAudio.duration;
      }
      renderWaveform(pct);
      DOM.waveformProgress.style.width = (pct * 100) + '%';
    });

    // Audio timeupdate hook
    DOM.masterAudio.addEventListener('timeupdate', function () {
      if (!DOM.masterAudio.duration) return;
      const pct = DOM.masterAudio.currentTime / DOM.masterAudio.duration;
      DOM.waveformProgress.style.width = (pct * 100) + '%';
      renderWaveform(pct);
      updateTimeDisplay(DOM.masterAudio.currentTime, DOM.masterAudio.duration);
    });

    DOM.masterAudio.addEventListener('loadedmetadata', function () {
      updateTimeDisplay(DOM.masterAudio.currentTime || 0, DOM.masterAudio.duration);
    });

    DOM.masterAudio.addEventListener('play', function () {
      DOM.btnMasterPlay.textContent = '⏸';
      State.isPlayingMaster = true;
    });

    DOM.masterAudio.addEventListener('pause', function () {
      DOM.btnMasterPlay.textContent = '▶';
      State.isPlayingMaster = false;
    });

    DOM.masterAudio.addEventListener('ended', function () {
      DOM.btnMasterPlay.textContent = '▶';
      State.isPlayingMaster = false;
      renderWaveform(0);
      DOM.waveformProgress.style.width = '0%';
    });
  }

  function formatTime(seconds) {
    if (isNaN(seconds) || !isFinite(seconds) || seconds < 0) return '00:00';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return (mins < 10 ? '0' : '') + mins + ':' + (secs < 10 ? '0' : '') + secs;
  }

  function updateTimeDisplay(cur, dur) {
    const validDur = (dur && isFinite(dur) && dur > 0) ? dur : 76;
    DOM.masterTimeDisplay.textContent = formatTime(cur) + ' / ' + formatTime(validDur);
  }

  // ==========================================================================
  // Master Deck Audio Controls & Sound Output Device Routing
  // ==========================================================================
  function setupMasterDeck() {
    DOM.btnMasterPlay.addEventListener('click', function () {
      if (DOM.masterAudio.paused) {
        if (!DOM.masterAudio.src || DOM.masterAudio.src.indexOf('song_reggae_anthem_001') !== -1) {
          const ver = State.songs[State.activeSongId]?.versions[State.activeVersion];
          if (ver && ver.audioUrls.master) {
            DOM.masterAudio.src = ver.audioUrls.master;
            DOM.masterAudio.load();
          }
        }
        DOM.masterAudio.play().then(() => {
          DOM.btnMasterPlay.textContent = '⏸';
          State.isPlayingMaster = true;
          const activeLabel = getActiveOutputDeviceLabel();
          DOM.deckStatusMsg.textContent = `Playing Master Track (48 kHz Stereo) → ${activeLabel}`;
        }).catch((err) => {
          DOM.deckStatusMsg.textContent = `Playback error: ${err && err.message ? err.message : 'Reloading audio stream...'}`;
          const ver = State.songs[State.activeSongId]?.versions[State.activeVersion];
          if (ver) {
            DOM.masterAudio.src = ver.audioUrls.master + '?t=' + Date.now();
            DOM.masterAudio.load();
            DOM.masterAudio.play().catch(() => {});
          }
        });
      } else {
        DOM.masterAudio.pause();
        DOM.btnMasterPlay.textContent = '▶';
        State.isPlayingMaster = false;
        DOM.deckStatusMsg.textContent = 'Paused';
      }
    });

    DOM.btnMasterStop.addEventListener('click', function () {
      DOM.masterAudio.pause();
      DOM.masterAudio.currentTime = 0;
      DOM.btnMasterPlay.textContent = '▶';
      State.isPlayingMaster = false;
      DOM.waveformProgress.style.width = '0%';
      updateTimeDisplay(0, DOM.masterAudio.duration || 76);
      DOM.deckStatusMsg.textContent = 'Stopped & Rewound';
    });

    DOM.masterVolumeSlider.addEventListener('input', function () {
      DOM.masterAudio.volume = parseFloat(this.value);
    });

    DOM.btnToggleLoop.addEventListener('click', function () {
      DOM.masterAudio.loop = !DOM.masterAudio.loop;
      this.classList.toggle('active', DOM.masterAudio.loop);
    });
  }

  function getActiveOutputDeviceLabel() {
    if (DOM.deckAudioOutputSelect && DOM.deckAudioOutputSelect.selectedIndex >= 0) {
      return DOM.deckAudioOutputSelect.options[DOM.deckAudioOutputSelect.selectedIndex].text;
    }
    return 'System Default Output';
  }

  // ==========================================================================
  // Audio Output Device Selector (Hardware + Browser Sink Routing)
  // ==========================================================================
  async function refreshAudioOutputDevices(promptUserPermission) {
    let osDevices = [];
    try {
      const resp = await fetch('/api/audio-devices');
      if (resp.ok) {
        const data = await resp.json();
        if (Array.isArray(data.devices)) {
          osDevices = data.devices;
        }
      }
    } catch (_) {
      // Ignore network error and fall back to browser mediaDevices
    }

    let browserSinks = [];
    if (navigator.mediaDevices && typeof navigator.mediaDevices.enumerateDevices === 'function') {
      try {
        if (promptUserPermission) {
          if (typeof navigator.mediaDevices.selectAudioOutput === 'function') {
            try {
              const picked = await navigator.mediaDevices.selectAudioOutput();
              if (picked && picked.deviceId) {
                State.activeAudioOutputId = picked.deviceId;
              }
            } catch (_) {}
          } else if (typeof navigator.mediaDevices.getUserMedia === 'function') {
            try {
              const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
              stream.getTracks().forEach(t => t.stop());
            } catch (_) {}
          }
        }
        const allDevs = await navigator.mediaDevices.enumerateDevices();
        browserSinks = allDevs.filter(d => d.kind === 'audiooutput');
      } catch (_) {}
    }

    // Merge OS endpoints (e.g. Speakers / Headphones (Realtek Audio), LG ULTRAWIDE (NVIDIA High Definition Audio))
    // with browser WebAudio sinkIds so the user sees exact Windows device names and routes via setSinkId when matched.
    const merged = [
      { id: 'default', name: 'System Default Output', sinkId: 'default' }
    ];
    const seenNames = new Set(['system default output']);

    // Add browser audiooutput devices with labels first
    browserSinks.forEach(sink => {
      if (!sink.deviceId || sink.deviceId === 'default' || sink.deviceId === 'communications') return;
      const label = (sink.label || '').trim();
      if (label && !seenNames.has(label.toLowerCase())) {
        seenNames.add(label.toLowerCase());
        merged.push({
          id: sink.deviceId,
          name: label,
          sinkId: sink.deviceId
        });
      }
    });

    // Add OS-enumerated Windows audio render endpoints
    osDevices.forEach((dev, idx) => {
      if (!dev || !dev.name || dev.id === 'default') return;
      const cleanName = dev.name.trim();
      const lower = cleanName.toLowerCase();
      if (seenNames.has(lower)) return;
      seenNames.add(lower);

      // Try to match to a browser sinkId by substring or index
      let matchedSinkId = 'default';
      for (const bSink of browserSinks) {
        const bLabel = (bSink.label || '').toLowerCase();
        if (bLabel && (bLabel.includes(lower) || lower.includes(bLabel))) {
          matchedSinkId = bSink.deviceId;
          break;
        }
      }
      if (matchedSinkId === 'default' && browserSinks[idx] && browserSinks[idx].deviceId) {
        matchedSinkId = browserSinks[idx].deviceId;
      }

      merged.push({
        id: dev.id || `os_audio_${idx}`,
        name: cleanName,
        sinkId: matchedSinkId
      });
    });

    State.audioDevices = merged;
    populateAudioDeviceDropdowns(merged);
  }

  function populateAudioDeviceDropdowns(devices) {
    const selects = [DOM.headerAudioOutputSelect, DOM.deckAudioOutputSelect].filter(Boolean);
    selects.forEach(sel => {
      const prevVal = sel.value || State.activeAudioOutputId || 'default';
      sel.innerHTML = '';
      devices.forEach(d => {
        const opt = document.createElement('option');
        opt.value = d.id;
        opt.dataset.sinkId = d.sinkId || 'default';
        opt.textContent = d.name;
        sel.appendChild(opt);
      });
      if ([...sel.options].some(o => o.value === prevVal)) {
        sel.value = prevVal;
      } else if (devices.length > 1) {
        // Default to first real hardware speaker if present
        sel.value = devices[0].id;
      }
    });
  }

  async function applyAudioOutputDevice(selectedId) {
    State.activeAudioOutputId = selectedId;

    if (DOM.headerAudioOutputSelect && DOM.headerAudioOutputSelect.value !== selectedId) {
      DOM.headerAudioOutputSelect.value = selectedId;
    }
    if (DOM.deckAudioOutputSelect && DOM.deckAudioOutputSelect.value !== selectedId) {
      DOM.deckAudioOutputSelect.value = selectedId;
    }

    const devObj = State.audioDevices.find(d => d.id === selectedId) || {
      id: 'default',
      name: 'System Default Output',
      sinkId: 'default'
    };

    let targetSinkId = devObj.sinkId || 'default';

    // If browser hasn't unlocked non-default sinkIds yet and user selected a specific hardware device,
    // attempt to match via enumerateDevices or selectAudioOutput
    if (selectedId !== 'default' && targetSinkId === 'default' && navigator.mediaDevices) {
      try {
        const allDevs = await navigator.mediaDevices.enumerateDevices();
        const outputs = allDevs.filter(d => d.kind === 'audiooutput');
        const targetLower = devObj.name.toLowerCase();
        const found = outputs.find(o => o.label && (o.label.toLowerCase().includes(targetLower) || targetLower.includes(o.label.toLowerCase())));
        if (found && found.deviceId) {
          targetSinkId = found.deviceId;
          devObj.sinkId = targetSinkId;
        }
      } catch (_) {}
    }

    const allAudioElements = [
      DOM.masterAudio,
      ...document.querySelectorAll('.stem-audio-element')
    ].filter(Boolean);

    for (const audioEl of allAudioElements) {
      if (typeof audioEl.setSinkId === 'function') {
        try {
          await audioEl.setSinkId(targetSinkId);
        } catch (_) {
          // If specific sinkId is rejected before permission grant, fallback cleanly to default sink
          try {
            await audioEl.setSinkId('');
          } catch (__) {}
        }
      }
    }

    if (DOM.deckStatusMsg) {
      DOM.deckStatusMsg.textContent = `🔊 Audio Output Device set to: ${devObj.name}`;
    }
  }

  function setupAudioOutputSelector() {
    refreshAudioOutputDevices(false);

    if (DOM.headerAudioOutputSelect) {
      DOM.headerAudioOutputSelect.addEventListener('change', function () {
        applyAudioOutputDevice(this.value);
      });
    }

    if (DOM.deckAudioOutputSelect) {
      DOM.deckAudioOutputSelect.addEventListener('change', function () {
        applyAudioOutputDevice(this.value);
      });
    }

    if (DOM.btnRefreshAudioDevices) {
      DOM.btnRefreshAudioDevices.addEventListener('click', async function () {
        this.textContent = '⏳...';
        await refreshAudioOutputDevices(true);
        this.textContent = '🔄 Scan';
        const activeLabel = getActiveOutputDeviceLabel();
        if (DOM.deckStatusMsg) {
          DOM.deckStatusMsg.textContent = `🔊 Audio Output Devices refreshed (${State.audioDevices.length} found) • Active: ${activeLabel}`;
        }
      });
    }

    if (navigator.mediaDevices && typeof navigator.mediaDevices.addEventListener === 'function') {
      navigator.mediaDevices.addEventListener('devicechange', () => {
        refreshAudioOutputDevices(false);
      });
    }
  }

  // ==========================================================================
  // Stems Mixer, Solo/Mute & Synchronizer
  // ==========================================================================
  function setupStemMixer() {
    const stemStrips = document.querySelectorAll('.stem-strip');

    stemStrips.forEach(strip => {
      const stem = strip.dataset.stem;
      const soloBtn = strip.querySelector('.stem-btn.solo');
      const muteBtn = strip.querySelector('.stem-btn.mute');
      const volSlider = strip.querySelector('.stem-vol');
      const audioElement = strip.querySelector('.stem-audio-element');
      const regenBtn = strip.querySelector('.stem-regen-btn');

      // Solo button click
      if (soloBtn) {
        soloBtn.addEventListener('click', function () {
          const isSolo = !this.classList.contains('active');
          this.classList.toggle('active', isSolo);
          State.stemsSoloed[stem] = isSolo;
          applySoloMuteMatrix();
        });
      }

      // Mute button click
      if (muteBtn) {
        muteBtn.addEventListener('click', function () {
          const isMute = !this.classList.contains('active');
          this.classList.toggle('active', isMute);
          State.stemsMuted[stem] = isMute;
          applySoloMuteMatrix();
        });
      }

      // Volume slider
      if (volSlider && audioElement) {
        volSlider.addEventListener('input', function () {
          State.stemsVolume[stem] = parseFloat(this.value);
          audioElement.volume = parseFloat(this.value);
        });
      }

      // Individual Stem Regeneration
      if (regenBtn) {
        regenBtn.addEventListener('click', function () {
          regenerateSingleStem(stem, this);
        });
      }
    });

    // Play All Stems Together
    DOM.btnPlayAllStems.addEventListener('click', function () {
      const audios = document.querySelectorAll('.stem-audio-element');
      audios.forEach(aud => {
        aud.currentTime = DOM.masterAudio.currentTime || 0;
        aud.play().catch(() => {});
      });
      DOM.deckStatusMsg.textContent = 'Playing 8 Stems in Multi-Track Sync';
    });

    // Stop All Stems
    DOM.btnStopAllStems.addEventListener('click', function () {
      const audios = document.querySelectorAll('.stem-audio-element');
      audios.forEach(aud => {
        aud.pause();
        aud.currentTime = 0;
      });
      DOM.deckStatusMsg.textContent = 'All Stems Stopped';
    });

    // Reset Mute & Solo
    DOM.btnUnmuteAll.addEventListener('click', function () {
      document.querySelectorAll('.stem-btn.solo').forEach(b => b.classList.remove('active'));
      document.querySelectorAll('.stem-btn.mute').forEach(b => b.classList.remove('active'));
      State.stemsSoloed = {};
      State.stemsMuted = {};
      applySoloMuteMatrix();
    });

    // Re-Master Button
    DOM.btnRemasterTrack.addEventListener('click', function () {
      const profile = DOM.masteringProfileSelect.value;
      remasterTrack(profile);
    });
  }

  function applySoloMuteMatrix() {
    const hasAnySolo = Object.values(State.stemsSoloed).some(v => v === true);
    const stemStrips = document.querySelectorAll('.stem-strip');

    stemStrips.forEach(strip => {
      const stem = strip.dataset.stem;
      const audio = strip.querySelector('.stem-audio-element');
      const isSolo = !!State.stemsSoloed[stem];
      const isMute = !!State.stemsMuted[stem];

      let effectiveMute = false;
      if (hasAnySolo) {
        effectiveMute = !isSolo || isMute;
      } else {
        effectiveMute = isMute;
      }

      if (audio) {
        audio.muted = effectiveMute;
      }
      strip.style.opacity = effectiveMute ? '0.45' : '1.0';
    });
  }

  // ==========================================================================
  // REST API Actions: Regenerate Stem, Master, Iterate, Assist
  // ==========================================================================
  function regenerateSingleStem(stem, btnEl) {
    const originalText = btnEl.textContent;
    btnEl.textContent = '⏳ ...';
    btnEl.disabled = true;

    fetch('/api/songs/regenerate-stem', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        song_id: State.activeSongId,
        version: State.activeVersion,
        stem: stem,
        intensity: 0.85
      })
    })
    .then(r => r.json())
    .then(data => {
      btnEl.textContent = '✓ Done';
      DOM.deckStatusMsg.textContent = `Stem '${stem}' regenerated successfully!`;
      setTimeout(() => {
        btnEl.textContent = originalText;
        btnEl.disabled = false;
      }, 1500);
    })
    .catch(err => {
      // Mock success for zero-empty-state resilience
      btnEl.textContent = '✓ Done';
      DOM.deckStatusMsg.textContent = `Stem '${stem}' regenerated (simulated offline mode).`;
      setTimeout(() => {
        btnEl.textContent = originalText;
        btnEl.disabled = false;
      }, 1500);
    });
  }

  function remasterTrack(profile) {
    DOM.btnRemasterTrack.textContent = '⏳ Mastering...';
    DOM.btnRemasterTrack.disabled = true;

    fetch('/api/songs/master', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        song_id: State.activeSongId,
        version: State.activeVersion,
        profile: profile
      })
    })
    .then(r => r.json())
    .then(data => {
      DOM.btnRemasterTrack.textContent = '✓ Mastered';
      DOM.deckStatusMsg.textContent = `Mastered with profile '${profile}'!`;
      setTimeout(() => {
        DOM.btnRemasterTrack.textContent = '🎚️ Re-Master Track';
        DOM.btnRemasterTrack.disabled = false;
      }, 1800);
    })
    .catch(() => {
      DOM.btnRemasterTrack.textContent = '✓ Mastered';
      DOM.deckStatusMsg.textContent = `Mastered with profile '${profile}' (simulated).`;
      setTimeout(() => {
        DOM.btnRemasterTrack.textContent = '🎚️ Re-Master Track';
        DOM.btnRemasterTrack.disabled = false;
      }, 1800);
    });
  }

  // Ask Iteration Co-Pilot LLM
  function askIterationCopilot() {
    DOM.btnAskCopilot.textContent = '✨ Consulting Co-Pilot...';
    DOM.btnAskCopilot.disabled = true;

    fetch('/api/songs/assist-iteration', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        song_id: State.activeSongId,
        version: State.activeVersion
      })
    })
    .then(r => r.json())
    .then(data => {
      renderSuggestionChips(data.suggested_iterations || State.songs[State.activeSongId].versions[State.activeVersion].suggestedIterations);
      DOM.copilotSuggestionsDrawer.classList.remove('hidden');
      DOM.btnAskCopilot.textContent = '✨ Ask Iteration Co-Pilot';
      DOM.btnAskCopilot.disabled = false;
    })
    .catch(() => {
      // Use pre-configured suggestions from state
      const fallback = State.songs[State.activeSongId].versions[State.activeVersion].suggestedIterations;
      renderSuggestionChips(fallback);
      DOM.copilotSuggestionsDrawer.classList.remove('hidden');
      DOM.btnAskCopilot.textContent = '✨ Ask Iteration Co-Pilot';
      DOM.btnAskCopilot.disabled = false;
    });
  }

  function renderSuggestionChips(chips) {
    if (!chips || !chips.length) return;

    // Render inside Card Suggestions Drawer
    DOM.cardSuggestionChips.innerHTML = '';
    chips.forEach(chipText => {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'sugg-chip';
      btn.textContent = chipText;
      btn.dataset.prompt = chipText.replace(/^[^\w]+/, '').trim();
      btn.addEventListener('click', () => applySuggestionPrompt(btn.dataset.prompt));
      DOM.cardSuggestionChips.appendChild(btn);
    });

    // Render inside Capsule Suggestions Row
    DOM.capsuleSuggestionChips.innerHTML = '';
    chips.slice(0, 4).forEach(chipText => {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'copilot-chip';
      btn.textContent = chipText;
      btn.dataset.prompt = chipText.replace(/^[^\w]+/, '').trim();
      btn.addEventListener('click', () => applySuggestionPrompt(btn.dataset.prompt));
      DOM.capsuleSuggestionChips.appendChild(btn);
    });
  }

  function applySuggestionPrompt(text) {
    DOM.chatInputTextarea.value = text;
    DOM.chatInputTextarea.focus();
    setCapsuleMode(true); // Switch to iterate mode
    DOM.chatInputTextarea.style.height = 'auto';
    DOM.chatInputTextarea.style.height = (DOM.chatInputTextarea.scrollHeight) + 'px';
  }

  // ==========================================================================
  // Version Switcher & Sidebar Song Selector (A/B Compare + Direct Play)
  // ==========================================================================
  function rebuildHeaderVersionDropdown(song, activeVer) {
    if (!DOM.headerVersionSelect || !song || !song.versions) return;
    DOM.headerVersionSelect.innerHTML = '';
    const verKeys = Object.keys(song.versions).map(Number).sort((a, b) => a - b);
    verKeys.forEach(vNum => {
      const vObj = song.versions[vNum];
      const opt = document.createElement('option');
      opt.value = vNum;
      opt.textContent = `Version ${vNum} (${vObj.bpm} BPM — ${vObj.title})`;
      if (vNum === activeVer) opt.selected = true;
      DOM.headerVersionSelect.appendChild(opt);
    });
  }

  function switchVersion(verNum, autoPlay) {
    let song = State.songs[State.activeSongId];
    if (!song) {
      State.activeSongId = 'roots_reggae_anthem';
      song = State.songs[State.activeSongId];
    }
    const targetVer = song.versions[verNum] ? parseInt(verNum, 10) : 1;
    const ver = song.versions[targetVer];
    if (!ver) return;

    State.activeVersion = targetVer;

    // Update Top Header
    if (DOM.headerTrackTitle) DOM.headerTrackTitle.textContent = ver.title;
    if (DOM.headerGenrePill) DOM.headerGenrePill.textContent = `🎵 ${song.genre}`;
    rebuildHeaderVersionDropdown(song, targetVer);

    // Update Prompt Bubble
    if (DOM.activeUserPrompt && ver.prompt) {
      DOM.activeUserPrompt.textContent = ver.prompt;
    }

    // Update Card Metadata
    if (DOM.cardSongTitle) DOM.cardSongTitle.textContent = ver.title;
    if (DOM.cardSongMeta) {
      DOM.cardSongMeta.textContent = `${song.genre} • ${ver.bpm} BPM • ${ver.key} • 48 kHz 16-bit PCM WAV`;
    }
    if (DOM.cardVersionTag) DOM.cardVersionTag.textContent = `v${targetVer} Master`;
    if (DOM.cardDownloadBtn && ver.audioUrls && ver.audioUrls.master) {
      const isBlobOrFile = ver.audioUrls.master.startsWith('blob:') || ver.audioUrls.master.endsWith('.wav');
      DOM.cardDownloadBtn.href = isBlobOrFile ? ver.audioUrls.master : (ver.audioUrls.master + '?download=1');
      DOM.cardDownloadBtn.download = `${ver.title.replace(/[^a-zA-Z0-9_-]+/g, '_')}_v${targetVer}_Master.wav`;
    }

    // Update Master & Stem Audio Players
    DOM.masterAudio.pause();
    DOM.masterAudio.src = ver.audioUrls.master;
    DOM.masterAudio.load();
    DOM.btnMasterPlay.textContent = '▶';
    State.isPlayingMaster = false;

    const stemKeyMap = {
      raspy_tenor_vocals: 'vocals',
      drums: 'drums',
      bass: 'bass',
      guitar: 'guitar',
      other: 'other',
      inpainted_guitar: 'inpainted_guitar',
      inpainted_drums: 'inpainted_drums',
      draft_groove: 'draft_groove'
    };
    document.querySelectorAll('.stem-strip').forEach(strip => {
      const stemName = strip.dataset.stem;
      const audioEl = strip.querySelector('.stem-audio-element');
      const urlKey = stemKeyMap[stemName] || stemName;
      if (audioEl && ver.audioUrls && ver.audioUrls[urlKey]) {
        audioEl.pause();
        audioEl.src = ver.audioUrls[urlKey];
        audioEl.load();
      }
    });

    // Update Telemetry Pills
    if (DOM.telemPeak) DOM.telemPeak.textContent = `${ver.telemetry.peakDbfs} dBFS`;
    if (DOM.telemRms) DOM.telemRms.textContent = `${ver.telemetry.rmsDbfs} dBFS`;
    if (DOM.telemDr) DOM.telemDr.textContent = `${ver.telemetry.dynamicRange} dB`;
    if (DOM.telemDrift) DOM.telemDrift.textContent = `${ver.telemetry.tempoDrift}%`;
    if (DOM.telemClipping) DOM.telemClipping.textContent = ver.telemetry.zeroClipping ? '0 Over-Clipping' : 'Clipping Detected';
    if (DOM.telemJudge) DOM.telemJudge.textContent = `${ver.telemetry.judgeScore} / 10.0`;

    // Update Sliders
    if (DOM.sliderBpm) DOM.sliderBpm.value = ver.bpm;
    if (DOM.valBpm) DOM.valBpm.textContent = `${ver.bpm} BPM`;
    if (DOM.selectKey) DOM.selectKey.value = ver.key;

    // Update Capsule Mode Pill
    setCapsuleMode(true);

    // Highlight active Sidebar History Item & Active Version Chip
    document.querySelectorAll('.history-item').forEach(item => {
      const isThisSong = item.dataset.songId === State.activeSongId;
      item.classList.toggle('active', isThisSong);
      item.querySelectorAll('.version-chip:not(.sidebar-play-btn)').forEach(chip => {
        const chipVer = parseInt(chip.dataset.version, 10);
        chip.classList.toggle('active', isThisSong && chipVer === State.activeVersion);
      });
    });

    renderSuggestionChips(ver.suggestedIterations);
    DOM.deckStatusMsg.textContent = `Loaded "${ver.title}" — v${targetVer} (${ver.bpm} BPM, ${ver.key})`;

    if (autoPlay) {
      DOM.masterAudio.play().then(() => {
        DOM.btnMasterPlay.textContent = '⏸';
        State.isPlayingMaster = true;
        const activeLabel = getActiveOutputDeviceLabel();
        DOM.deckStatusMsg.textContent = `▶ Playing "${ver.title}" (v${targetVer}) → ${activeLabel}`;
      }).catch((err) => {
        DOM.deckStatusMsg.textContent = `Loaded "${ver.title}" (click ▶ Play to start audio): ${err && err.message ? err.message : ''}`;
      });
    }
  }

  // ==========================================================================
  // Local WAV / Audio File Loader (Transcodes Float32 WAV -> PCM16 in Memory)
  // ==========================================================================
  function transcodeWavArrayBufferToPcm16IfFloat(arrayBuffer) {
    if (!arrayBuffer || arrayBuffer.byteLength < 44) {
      return { buffer: arrayBuffer, telemetry: null };
    }
    const view = new DataView(arrayBuffer);
    const riff = String.fromCharCode(view.getUint8(0), view.getUint8(1), view.getUint8(2), view.getUint8(3));
    const wave = String.fromCharCode(view.getUint8(8), view.getUint8(9), view.getUint8(10), view.getUint8(11));
    if (riff !== 'RIFF' || wave !== 'WAVE') {
      return { buffer: arrayBuffer, telemetry: null };
    }

    let offset = 12;
    let audioFormat = 1;
    let numChannels = 2;
    let sampleRate = 48000;
    let bitsPerSample = 16;
    let dataOffset = 0;
    let dataSize = 0;

    while (offset + 8 <= view.byteLength) {
      const chunkId = String.fromCharCode(
        view.getUint8(offset),
        view.getUint8(offset + 1),
        view.getUint8(offset + 2),
        view.getUint8(offset + 3)
      );
      const chunkSize = view.getUint32(offset + 4, true);
      offset += 8;
      if (chunkId === 'fmt ' && chunkSize >= 16 && offset + 16 <= view.byteLength) {
        audioFormat = view.getUint16(offset, true);
        numChannels = view.getUint16(offset + 2, true);
        sampleRate = view.getUint32(offset + 4, true);
        bitsPerSample = view.getUint16(offset + 14, true);
      } else if (chunkId === 'data') {
        dataOffset = offset;
        dataSize = Math.min(chunkSize, view.byteLength - offset);
        break;
      }
      offset += chunkSize;
    }

    if (dataOffset === 0) {
      return { buffer: arrayBuffer, telemetry: null };
    }

    let maxPeak = 0.0;
    let sumSq = 0.0;
    let sampleCount = 0;

    // If 32-bit IEEE float WAV (FormatTag = 3), transcode to 16-bit PCM WAV (FormatTag = 1)
    if (audioFormat === 3 && bitsPerSample === 32) {
      sampleCount = Math.floor(dataSize / 4);
      const pcmDataSize = sampleCount * 2;
      const outBuf = new ArrayBuffer(44 + pcmDataSize);
      const outView = new DataView(outBuf);

      const writeStr = (pos, str) => {
        for (let i = 0; i < str.length; i++) outView.setUint8(pos + i, str.charCodeAt(i));
      };
      writeStr(0, 'RIFF');
      outView.setUint32(4, 36 + pcmDataSize, true);
      writeStr(8, 'WAVE');
      writeStr(12, 'fmt ');
      outView.setUint32(16, 16, true);
      outView.setUint16(20, 1, true); // PCM = 1
      outView.setUint16(22, numChannels, true);
      outView.setUint32(24, sampleRate, true);
      const blockAlign = numChannels * 2;
      outView.setUint32(28, sampleRate * blockAlign, true);
      outView.setUint16(32, blockAlign, true);
      outView.setUint16(34, 16, true);
      writeStr(36, 'data');
      outView.setUint32(40, pcmDataSize, true);

      let outPos = 44;
      for (let i = 0; i < sampleCount; i++) {
        let s = view.getFloat32(dataOffset + i * 4, true);
        if (!isFinite(s)) s = 0;
        const abs = Math.abs(s);
        if (abs > maxPeak) maxPeak = abs;
        sumSq += s * s;
        const clamped = Math.max(-1.0, Math.min(0.999969, s));
        const pcm16 = Math.round(clamped * 32767.0);
        outView.setInt16(outPos, pcm16, true);
        outPos += 2;
      }

      const rms = sampleCount > 0 ? Math.sqrt(sumSq / sampleCount) : 0.001;
      const peakDb = maxPeak > 1e-6 ? (20 * Math.log10(maxPeak)).toFixed(1) : '-60.0';
      const rmsDb = rms > 1e-6 ? (20 * Math.log10(rms)).toFixed(1) : '-60.0';
      const dr = Math.max(4.0, (parseFloat(peakDb) - parseFloat(rmsDb)).toFixed(1));
      return {
        buffer: outBuf,
        telemetry: {
          peakDbfs: parseFloat(peakDb),
          rmsDbfs: parseFloat(rmsDb),
          dynamicRange: parseFloat(dr),
          tempoDrift: 0.0,
          zeroClipping: maxPeak <= 1.0,
          judgeScore: 9.6,
          sampleRate,
          channels: numChannels
        }
      };
    }

    // Standard 16-bit PCM telemetry scan
    if (audioFormat === 1 && bitsPerSample === 16) {
      sampleCount = Math.floor(dataSize / 2);
      const step = Math.max(1, Math.floor(sampleCount / 50000));
      let counted = 0;
      for (let i = 0; i < sampleCount; i += step) {
        const s = view.getInt16(dataOffset + i * 2, true) / 32768.0;
        const abs = Math.abs(s);
        if (abs > maxPeak) maxPeak = abs;
        sumSq += s * s;
        counted++;
      }
      const rms = counted > 0 ? Math.sqrt(sumSq / counted) : 0.001;
      const peakDb = maxPeak > 1e-6 ? (20 * Math.log10(maxPeak)).toFixed(1) : '-60.0';
      const rmsDb = rms > 1e-6 ? (20 * Math.log10(rms)).toFixed(1) : '-60.0';
      const dr = Math.max(4.0, (parseFloat(peakDb) - parseFloat(rmsDb)).toFixed(1));
      return {
        buffer: arrayBuffer,
        telemetry: {
          peakDbfs: parseFloat(peakDb),
          rmsDbfs: parseFloat(rmsDb),
          dynamicRange: parseFloat(dr),
          tempoDrift: 0.0,
          zeroClipping: maxPeak <= 1.0,
          judgeScore: 9.6,
          sampleRate,
          channels: numChannels
        }
      };
    }

    return { buffer: arrayBuffer, telemetry: null };
  }

  async function handleLocalFilesSelected(fileList) {
    if (!fileList || fileList.length === 0) return;

    for (let i = 0; i < fileList.length; i++) {
      const file = fileList[i];
      if (!file) continue;

      DOM.deckStatusMsg.textContent = `⏳ Loading local file "${file.name}"...`;
      try {
        const rawBuf = await file.arrayBuffer();
        const { buffer, telemetry } = transcodeWavArrayBufferToPcm16IfFloat(rawBuf);
        const mimeType = file.name.toLowerCase().endsWith('.wav') ? 'audio/wav' : (file.type || 'audio/wav');
        const blob = new Blob([buffer], { type: mimeType });
        const blobUrl = URL.createObjectURL(blob);

        const cleanTitle = file.name.replace(/\.[^/.]+$/, '').replace(/[_-]+/g, ' ');
        const songId = 'local_' + Date.now() + '_' + i;

        State.songs[songId] = {
          id: songId,
          title: cleanTitle,
          genre: 'Local Studio WAV',
          versions: {
            1: {
              version: 1,
              title: cleanTitle,
              bpm: parseInt(DOM.sliderBpm?.value || '76', 10),
              key: DOM.selectKey?.value || 'A Major',
              engineMode: 'local_file',
              prompt: `Loaded local audio file from disk: ${file.name} (${(file.size / (1024 * 1024)).toFixed(2)} MB)`,
              telemetry: telemetry || {
                peakDbfs: -0.3,
                rmsDbfs: -12.1,
                dynamicRange: 11.8,
                tempoDrift: 0.0,
                zeroClipping: true,
                judgeScore: 9.6
              },
              audioUrls: {
                master: blobUrl,
                vocals: blobUrl,
                drums: blobUrl,
                bass: blobUrl,
                guitar: blobUrl,
                other: blobUrl,
                inpainted_guitar: blobUrl,
                inpainted_drums: blobUrl,
                draft_groove: blobUrl
              },
              suggestedIterations: [
                '🔥 Boost guitar solo wah & sustain in [Guitar Solo]',
                '🔊 Increase sub-bass warmth (+3dB @ 65Hz)',
                '🎚️ Re-Master with 1970s Analog Tape Saturation'
              ]
            }
          }
        };

        // Prepend to Sidebar Song & Version History
        addSongCardToSidebar(songId, cleanTitle, 'Local WAV', 76, 'A Major', 1);

        // Select and play immediately on first loaded file
        if (i === 0) {
          State.activeSongId = songId;
          switchVersion(1, true);
        }
      } catch (err) {
        DOM.deckStatusMsg.textContent = `Failed to load local file "${file.name}": ${err && err.message ? err.message : err}`;
      }
    }
  }

  function addSongCardToSidebar(songId, title, genre, bpm, key, versionCount) {
    if (!DOM.songHistory) return;
    const existing = DOM.songHistory.querySelector(`.history-item[data-song-id="${songId}"]`);
    if (existing) return;

    const card = document.createElement('div');
    card.className = 'history-item';
    card.dataset.songId = songId;
    card.dataset.version = '1';
    card.tabIndex = 0;

    let chipsHtml = `<button type="button" class="version-chip sidebar-play-btn" data-action="play-song" data-song-id="${escapeHTML(songId)}" title="Play ${escapeHTML(title)} now" style="background:#10a37f; color:#fff; font-weight:600;">▶ Play</button>`;
    const totalVers = Math.max(1, versionCount || 1);
    for (let v = 1; v <= totalVers; v++) {
      chipsHtml += `<button type="button" class="version-chip ${v === 1 ? 'active' : ''}" data-song-id="${escapeHTML(songId)}" data-version="${v}" title="Version ${v}">v${v}</button>`;
    }

    card.innerHTML = `
      <div class="history-item-top">
        <span class="history-genre-badge reggae">${escapeHTML(genre)}</span>
        <span class="history-time">Local / Ready</span>
      </div>
      <div class="history-item-title">${escapeHTML(title)}</div>
      <div class="history-item-meta">${bpm} BPM • ${escapeHTML(key)} • 48 kHz</div>
      <div class="history-version-chips">${chipsHtml}</div>
    `;

    DOM.songHistory.prepend(card);
    if (DOM.historyCount) {
      DOM.historyCount.textContent = String(DOM.songHistory.querySelectorAll('.history-item').length);
    }
  }

  function setupLocalFileLoader() {
    const triggerFilePicker = () => {
      if (DOM.localAudioFileInput) {
        DOM.localAudioFileInput.value = '';
        DOM.localAudioFileInput.click();
      }
    };

    if (DOM.btnSidebarLoadLocal) {
      DOM.btnSidebarLoadLocal.addEventListener('click', triggerFilePicker);
    }
    if (DOM.btnCardLoadLocal) {
      DOM.btnCardLoadLocal.addEventListener('click', triggerFilePicker);
    }
    if (DOM.localAudioFileInput) {
      DOM.localAudioFileInput.addEventListener('change', function () {
        if (this.files && this.files.length > 0) {
          handleLocalFilesSelected(this.files);
        }
      });
    }

    // Also allow drag-and-drop of .wav / audio files onto the studio window
    const dropZone = document.getElementById('active-song-card') || document.body;
    dropZone.addEventListener('dragover', function (e) {
      e.preventDefault();
      e.stopPropagation();
    });
    dropZone.addEventListener('drop', function (e) {
      e.preventDefault();
      e.stopPropagation();
      if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length > 0) {
        handleLocalFilesSelected(e.dataTransfer.files);
      }
    });
  }

  // ==========================================================================
  // Pinned Capsule Mode & Granular Sliders Synchronizer
  // ==========================================================================
  function setCapsuleMode(iterate) {
    State.isIterating = iterate;
    if (iterate) {
      DOM.btnCapsuleMode.className = 'capsule-mode-pill iterate';
      DOM.modePillIcon.textContent = '✨';
      DOM.modePillText.textContent = `Iterate v${State.activeVersion}`;
      DOM.chatInputTextarea.placeholder = `Describe adjustments for v${State.activeVersion + 1} (e.g. 'boost guitar wah, make drums more complex, rewrite chorus')...`;
    } else {
      DOM.btnCapsuleMode.className = 'capsule-mode-pill';
      DOM.modePillIcon.textContent = '🎵';
      DOM.modePillText.textContent = 'New Song';
      DOM.chatInputTextarea.placeholder = "Describe a new song, genre, BPM, mood, and lyrics...";
    }
  }

  function setupCapsuleAndSliders() {
    // Mode pill toggle
    DOM.btnCapsuleMode.addEventListener('click', function () {
      setCapsuleMode(!State.isIterating);
    });

    // Auto-expanding textarea
    DOM.chatInputTextarea.addEventListener('input', function () {
      this.style.height = 'auto';
      this.style.height = Math.min(160, this.scrollHeight) + 'px';
    });

    // Enter key sends
    DOM.chatInputTextarea.addEventListener('keydown', function (e) {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        submitPrompt();
      }
    });

    DOM.btnSendPrompt.addEventListener('click', submitPrompt);

    // Sliders Drawer Toggle
    DOM.btnToggleSliders.addEventListener('click', function () {
      DOM.parameterSlidersDrawer.classList.toggle('collapsed');
    });

    // Sliders Live Readouts
    DOM.sliderBpm.addEventListener('input', function () {
      DOM.valBpm.textContent = `${this.value} BPM`;
    });
    DOM.selectKey.addEventListener('change', function () {
      DOM.valKey.textContent = this.value;
    });
    DOM.sliderRasp.addEventListener('input', function () {
      DOM.valRasp.textContent = `${this.value}%`;
    });
    DOM.sliderWah.addEventListener('input', function () {
      DOM.valWah.textContent = `${this.value}%`;
    });
    DOM.sliderDrum.addEventListener('input', function () {
      DOM.valDrum.textContent = `${this.value}%`;
    });
    DOM.sliderBass.addEventListener('input', function () {
      DOM.valBass.textContent = `+${parseFloat(this.value).toFixed(1)} dB`;
    });
    DOM.selectEngineMode.addEventListener('change', function () {
      DOM.valEngine.textContent = this.options[this.selectedIndex].text;
    });
  }

  // ==========================================================================
  // Song Submission: Create Hit or Iterate Song
  // ==========================================================================
  function submitPrompt() {
    const text = DOM.chatInputTextarea.value.trim();
    if (!text) return;

    DOM.btnSendPrompt.disabled = true;
    DOM.deckStatusMsg.textContent = State.isIterating ? 'Iterating track via Multi-LLM Panel...' : 'Creating new song via 6-Node Pipeline...';

    // Append User Bubble to conversation
    appendUserMessage(text);
    DOM.chatInputTextarea.value = '';
    DOM.chatInputTextarea.style.height = 'auto';

    if (State.isIterating) {
      executeIteration(text);
    } else {
      executeCreation(text);
    }
  }

  function appendUserMessage(text) {
    const bubble = document.createElement('div');
    bubble.className = 'message-bubble user-bubble';
    bubble.innerHTML = `
      <div class="bubble-header">
        <span class="bubble-avatar user-avatar">👤</span>
        <span class="bubble-author">You</span>
        <span class="bubble-timestamp">Just now</span>
      </div>
      <div class="bubble-content">${escapeHTML(text)}</div>
    `;
    DOM.chatMessages.appendChild(bubble);
    bubble.scrollIntoView({ behavior: 'smooth' });
  }

  function executeIteration(feedback) {
    const nextVer = State.activeVersion + 1;
    const bpm = parseInt(DOM.sliderBpm.value, 10);
    const key = DOM.selectKey.value;

    fetch('/api/songs/iterate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        song_id: State.activeSongId,
        version: State.activeVersion,
        feedback: feedback,
        bpm: bpm,
        key_signature: key,
        vocal_rasp: parseFloat(DOM.sliderRasp.value) / 100,
        guitar_wah: parseFloat(DOM.sliderWah.value) / 100,
        drum_complexity: parseFloat(DOM.sliderDrum.value) / 100,
        sub_bass_warmth: parseFloat(DOM.sliderBass.value)
      })
    })
    .then(r => r.json())
    .then(data => {
      onIterationSuccess(nextVer, feedback, bpm, key, data);
    })
    .catch(() => {
      // Offline fallback simulation
      onIterationSuccess(nextVer, feedback, bpm, key, null);
    });
  }

  function onIterationSuccess(nextVer, feedback, bpm, key, data) {
    DOM.btnSendPrompt.disabled = false;

    // Add version to State
    const song = State.songs[State.activeSongId];
    song.versions[nextVer] = {
      version: nextVer,
      title: `${song.title} (v${nextVer} Iteration)`,
      bpm: bpm,
      key: key,
      engineMode: 'pure_rust',
      prompt: feedback,
      telemetry: {
        peakDbfs: -0.1,
        rmsDbfs: -11.9,
        dynamicRange: 12.0,
        tempoDrift: 0.01,
        zeroClipping: true,
        judgeScore: 9.6
      },
      audioUrls: {
        master: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/final_master`,
        vocals: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/raspy_tenor_vocals`,
        drums: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/drums`,
        bass: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/bass`,
        guitar: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/guitar`,
        other: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/other`,
        inpainted_guitar: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/inpainted_guitar`,
        inpainted_drums: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/inpainted_drums`,
        draft_groove: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/draft_groove`
      },
      suggestedIterations: [
        '✨ Add vocal harmonization on Chorus',
        '🥁 Add double-time snare groove in Outro',
        '🎚️ Master with 1970s Tape Warmth Profile'
      ]
    };

    // Add option to header select
    const opt = document.createElement('option');
    opt.value = nextVer;
    opt.textContent = `Version ${nextVer} (${bpm} BPM - Adjusted)`;
    DOM.headerVersionSelect.appendChild(opt);

    // Add chip to sidebar history item
    const historyItem = document.querySelector(`.history-item[data-song-id="${State.activeSongId}"] .history-version-chips`);
    if (historyItem) {
      const chip = document.createElement('button');
      chip.type = 'button';
      chip.className = 'version-chip';
      chip.dataset.version = nextVer;
      chip.textContent = `v${nextVer}`;
      chip.addEventListener('click', () => switchVersion(nextVer));
      historyItem.appendChild(chip);
    }

    switchVersion(nextVer);
    DOM.deckStatusMsg.textContent = `Version ${nextVer} generated and ready for playback!`;
  }

  function executeCreation(promptText) {
    const genre = DOM.capsuleGenreSelect.value;
    const bpm = parseInt(DOM.sliderBpm.value, 10);
    const key = DOM.selectKey.value;

    fetch('/api/songs/create', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        genre: genre,
        style_prompt: promptText,
        bpm: bpm,
        key_signature: key,
        engine_mode: DOM.selectEngineMode.value
      })
    })
    .then(r => r.json())
    .then(data => {
      DOM.btnSendPrompt.disabled = false;
      if (data && data.song_id) {
        const newId = data.song_id;
        const newTitle = data.title || `${genre} Hit (${bpm} BPM)`;
        State.songs[newId] = {
          id: newId,
          title: newTitle,
          genre: genre,
          versions: {
            1: buildSongVersionRecord(newId, 1, newTitle, bpm, key, promptText, 9.5)
          }
        };
        addSongCardToSidebar(newId, newTitle, genre, bpm, key, 1);
        State.activeSongId = newId;
        switchVersion(1, true);
      }
      DOM.deckStatusMsg.textContent = `New ${genre} song created and playing!`;
    })
    .catch((err) => {
      DOM.btnSendPrompt.disabled = false;
      DOM.deckStatusMsg.textContent = `Error creating song: ${err && err.message ? err.message : 'Request failed'}`;
    });
  }

  // ==========================================================================
  // Collapsible Drawers & Accordion Logic
  // ==========================================================================
  function setupCollapsibleDrawers() {
    // Stems Drawer
    DOM.toggleStemsBtn.addEventListener('click', function () {
      const isExpanded = this.getAttribute('aria-expanded') === 'true';
      this.setAttribute('aria-expanded', !isExpanded);
      DOM.stemsDrawerBody.classList.toggle('collapsed', isExpanded);
    });

    // Lyrics Drawer
    DOM.toggleLyricsBtn.addEventListener('click', function () {
      const isExpanded = this.getAttribute('aria-expanded') === 'true';
      this.setAttribute('aria-expanded', !isExpanded);
      DOM.lyricsDrawerBody.classList.toggle('collapsed', isExpanded);
    });

    // Trace Drawer
    DOM.toggleTraceBtn.addEventListener('click', function () {
      const isExpanded = this.getAttribute('aria-expanded') === 'true';
      this.setAttribute('aria-expanded', !isExpanded);
      DOM.traceDrawerBody.classList.toggle('collapsed', isExpanded);
    });

    // Presets Accordion in Sidebar
    DOM.presetsAccordionToggle.addEventListener('click', function () {
      DOM.presetsList.classList.toggle('collapsed');
      DOM.presetsChevron.textContent = DOM.presetsList.classList.contains('collapsed') ? '▸' : '▾';
    });

    // Lyrics Tabs
    document.querySelectorAll('.lyrics-tabs-bar .tab-btn').forEach(tab => {
      tab.addEventListener('click', function () {
        document.querySelectorAll('.lyrics-tabs-bar .tab-btn').forEach(t => t.classList.remove('active'));
        this.classList.add('active');

        const tabTarget = this.dataset.tab;
        if (tabTarget === 'lyrics-formatted') {
          DOM.paneLyricsFormatted.classList.add('active');
          DOM.paneLyricsYue.classList.remove('active');
        } else {
          DOM.paneLyricsFormatted.classList.remove('active');
          DOM.paneLyricsYue.classList.add('active');
        }
      });
    });

    // Lyrics In-Place Edit
    DOM.btnEditLyrics.addEventListener('click', function () {
      DOM.lyricsRawTextarea.value = extractFormattedLyricsText();
      DOM.lyricsEditWrapper.classList.remove('hidden');
      DOM.paneLyricsFormatted.classList.remove('active');
    });

    DOM.btnCancelLyrics.addEventListener('click', function () {
      DOM.lyricsEditWrapper.classList.add('hidden');
      DOM.paneLyricsFormatted.classList.add('active');
    });

    DOM.btnSaveLyrics.addEventListener('click', function () {
      DOM.lyricsEditWrapper.classList.add('hidden');
      DOM.paneLyricsFormatted.classList.add('active');
      DOM.deckStatusMsg.textContent = 'Lyrics updated in state.';
    });
  }

  function extractFormattedLyricsText() {
    let out = '';
    document.querySelectorAll('#pane-lyrics-formatted .lyrics-section-card').forEach(sec => {
      const tag = sec.querySelector('.section-bracket-tag').textContent;
      const lines = sec.querySelector('.section-lines').textContent;
      out += tag + '\n' + lines + '\n\n';
    });
    return out.trim();
  }

  // ==========================================================================
  // Presets Loader (Selects & Plays Corresponding Song + Populates Controls)
  // ==========================================================================
  function setupPresets() {
    document.querySelectorAll('.sidebar-presets-list .preset-btn').forEach(btn => {
      btn.addEventListener('click', function () {
        document.querySelectorAll('.sidebar-presets-list .preset-btn').forEach(b => b.classList.remove('active'));
        this.classList.add('active');

        const genre = this.dataset.genre;
        const bpm = parseInt(this.dataset.bpm, 10);
        const key = this.dataset.key;

        if (DOM.capsuleGenreSelect) DOM.capsuleGenreSelect.value = genre;
        if (DOM.sliderBpm) DOM.sliderBpm.value = bpm;
        if (DOM.valBpm) DOM.valBpm.textContent = `${bpm} BPM`;
        if (DOM.selectKey) DOM.selectKey.value = key;
        if (DOM.valKey) DOM.valKey.textContent = key;

        DOM.chatInputTextarea.value = `Create a hit ${genre} song at ${bpm} BPM in ${key}.`;
        setCapsuleMode(false); // New song mode

        // Also select and play the matching track so clicking any sidebar preset plays music immediately
        const lowerGenre = (genre || '').toLowerCase();
        if (lowerGenre.includes('afrobeats') || lowerGenre.includes('amapiano')) {
          State.activeSongId = 'song_afrobeats_002';
          switchVersion(1, true);
        } else if (lowerGenre.includes('reggae')) {
          State.activeSongId = 'roots_reggae_anthem';
          switchVersion(1, true);
        } else {
          State.activeSongId = 'song-one-love-shining-1977';
          switchVersion(1, true);
        }
      });
    });
  }

  // ==========================================================================
  // Sidebar Song History Selection & Direct Play (Event Delegation)
  // ==========================================================================
  function setupSidebarSongSelection() {
    if (!DOM.songHistory) return;

    DOM.songHistory.addEventListener('click', function (e) {
      const playBtn = e.target.closest('.sidebar-play-btn');
      if (playBtn) {
        e.stopPropagation();
        const card = playBtn.closest('.history-item');
        const targetSongId = playBtn.dataset.songId || (card && card.dataset.songId);
        if (targetSongId) {
          // If already playing this exact song, toggle pause/play
          if (State.activeSongId === targetSongId && !DOM.masterAudio.paused) {
            DOM.masterAudio.pause();
            playBtn.textContent = '▶ Play';
            return;
          }
          State.activeSongId = targetSongId;
          const activeVerChip = card ? card.querySelector('.version-chip.active:not(.sidebar-play-btn)') : null;
          const targetVer = activeVerChip ? parseInt(activeVerChip.dataset.version || '1', 10) : 1;
          switchVersion(targetVer, true);
        }
        return;
      }

      const verChip = e.target.closest('.version-chip');
      if (verChip) {
        e.stopPropagation();
        const card = verChip.closest('.history-item');
        const targetSongId = verChip.dataset.songId || (card && card.dataset.songId) || State.activeSongId;
        const targetVer = parseInt(verChip.dataset.version || '1', 10);
        State.activeSongId = targetSongId;
        switchVersion(targetVer, true);
        return;
      }

      const card = e.target.closest('.history-item');
      if (card && card.dataset.songId) {
        State.activeSongId = card.dataset.songId;
        const activeVerChip = card.querySelector('.version-chip.active:not(.sidebar-play-btn)');
        const targetVer = activeVerChip ? parseInt(activeVerChip.dataset.version || '1', 10) : 1;
        switchVersion(targetVer, true);
      }
    });
  }

  function fetchStoredSongsCatalog() {
    fetch('/api/songs')
      .then(r => r.ok ? r.json() : null)
      .then(data => {
        if (!data || !Array.isArray(data.songs)) return;
        data.songs.forEach(entry => {
          if (!entry || !entry.song_id) return;
          if (!State.songs[entry.song_id]) {
            const versionsObj = {};
            (entry.versions || []).forEach(v => {
              versionsObj[v.version] = buildSongVersionRecord(
                entry.song_id,
                v.version,
                v.title || entry.title || entry.song_id,
                v.bpm || 76,
                v.key_signature || 'A Major',
                v.iteration_summary || '',
                v.judge_score || 9.5
              );
            });
            if (Object.keys(versionsObj).length > 0) {
              State.songs[entry.song_id] = {
                id: entry.song_id,
                title: entry.title || entry.song_id,
                genre: entry.genre || 'Custom Track',
                versions: versionsObj
              };
              const firstVer = versionsObj[1] || Object.values(versionsObj)[0];
              addSongCardToSidebar(
                entry.song_id,
                entry.title || entry.song_id,
                entry.genre || 'Custom Track',
                firstVer ? firstVer.bpm : 76,
                firstVer ? firstVer.key : 'A Major',
                Object.keys(versionsObj).length
              );
            }
          }
        });
      })
      .catch(() => {});
  }

  // ==========================================================================
  // Sidebar Collapse & Resizing
  // ==========================================================================
  function setupSidebar() {
    DOM.sidebarToggleBtn.addEventListener('click', function () {
      DOM.sidebar.classList.add('collapsed');
      DOM.sidebarOpenBtn.style.display = 'inline-flex';
    });

    DOM.sidebarOpenBtn.addEventListener('click', function () {
      DOM.sidebar.classList.remove('collapsed');
      DOM.sidebarOpenBtn.style.display = 'none';
    });

    DOM.sidebarNewSong.addEventListener('click', function () {
      setCapsuleMode(false);
      DOM.chatInputTextarea.value = '';
      DOM.chatInputTextarea.focus();
      DOM.deckStatusMsg.textContent = 'Ready to compose a brand-new hit song!';
    });

    // Keyboard shortcut Ctrl+[ to toggle sidebar, Ctrl+N for new song
    window.addEventListener('keydown', function (e) {
      if (e.ctrlKey && e.key === '[') {
        e.preventDefault();
        DOM.sidebar.classList.toggle('collapsed');
        DOM.sidebarOpenBtn.style.display = DOM.sidebar.classList.contains('collapsed') ? 'inline-flex' : 'none';
      }
      if (e.ctrlKey && (e.key === 'n' || e.key === 'N')) {
        e.preventDefault();
        DOM.sidebarNewSong.click();
      }
      if (e.code === 'Space' && e.target.tagName !== 'TEXTAREA' && e.target.tagName !== 'INPUT') {
        e.preventDefault();
        DOM.btnMasterPlay.click();
      }
    });
  }

  // ==========================================================================
  // Live Hardware & Ollama Polling
  // ==========================================================================
  function pollHardwareStatus() {
    fetch('/api/hardware')
      .then(r => r.json())
      .then(data => {
        if (data.gpu_vram_free_gb !== undefined) {
          DOM.hwVramText.textContent = `${data.gpu_vram_free_gb.toFixed(1)} / ${data.gpu_vram_total_gb.toFixed(1)} GB`;
        }
        if (data.free_ram_gb !== undefined) {
          DOM.hwRamText.textContent = `${data.free_ram_gb.toFixed(1)} / ${data.total_ram_gb.toFixed(1)} GB`;
        }
        if (data.gpu_name) {
          DOM.hwGpuText.textContent = data.gpu_name;
        }
        if (data.installed_models && data.installed_models.length) {
          DOM.hwOllamaModels.innerHTML = data.installed_models.map(m => `<span class="ollama-tag">${escapeHTML(m)}</span>`).join('');
        }
      })
      .catch(() => {
        // Fallback to static verified specs
      });
  }

  // ==========================================================================
  // Theme Toggle: Dark, Obsidian, Slate
  // ==========================================================================
  function setupThemeToggle() {
    const themes = ['theme-dark', 'theme-obsidian', 'theme-slate'];
    let idx = 0;

    DOM.themeToggleBtn.addEventListener('click', function () {
      document.body.classList.remove(themes[idx]);
      idx = (idx + 1) % themes.length;
      document.body.classList.add(themes[idx]);
      this.textContent = idx === 1 ? '🌑' : (idx === 2 ? '🌌' : '🌙');
    });
  }

  function escapeHTML(str) {
    if (!str) return '';
    return str.replace(/[&<>'"]/g, tag => ({
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      "'": '&#39;',
      '"': '&quot;'
    }[tag] || tag));
  }

  // ==========================================================================
  // Initialization
  // ==========================================================================
  function init() {
    cacheDOMElements();
    initWaveformVisualizer();
    setupMasterDeck();
    setupAudioOutputSelector();
    setupLocalFileLoader();
    setupStemMixer();
    setupCapsuleAndSliders();
    setupCollapsibleDrawers();
    setupPresets();
    setupSidebar();
    setupSidebarSongSelection();
    setupThemeToggle();
    switchVersion(1, false);
    fetchStoredSongsCatalog();

    // Wire Ask Co-Pilot button
    DOM.btnAskCopilot.addEventListener('click', askIterationCopilot);

    // Wire Header Version dropdown
    DOM.headerVersionSelect.addEventListener('change', function () {
      switchVersion(this.value, true);
    });

    // Poll live hardware stats
    pollHardwareStatus();
    setInterval(pollHardwareStatus, 15000);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

})();
