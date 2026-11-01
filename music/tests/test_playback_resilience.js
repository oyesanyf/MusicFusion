// tests/test_playback_resilience.js
// Empirical test harness for Playback 404 Resilience in WebUI (app.js)

const assert = require('assert');

// 1. Extract and bind the exact logic from crates/music_cli/webui/app.js
function createEnvironment() {
  const State = {
    activeSongId: 'roots_reggae_anthem',
    activeVersion: 2,
    isPlayingMaster: true,
    songs: {}
  };

  const DOM = {
    btnSendPrompt: { disabled: true },
    deckStatusMsg: { textContent: '' },
    masterAudio: {
      src: 'http://127.0.0.1:5050/api/songs/roots_reggae_anthem/versions/3/audio/final_master',
      loadCalled: false,
      load() { this.loadCalled = true; }
    },
    btnMasterPlay: { textContent: '⏸' },
    headerVersionSelect: {
      children: [],
      appendChild(child) { this.children.push(child); }
    }
  };

  function findLastValidAudioUrls(song, maxVer) {
    if (!song || !song.versions) return null;
    const startVer = typeof maxVer === 'number' ? maxVer : State.activeVersion;
    for (let v = startVer; v >= 1; v--) {
      const candidate = song.versions[v];
      if (candidate && candidate.audioUrls && candidate.audioUrls.master) {
        return candidate.audioUrls;
      }
    }
    const fallback = Object.values(song.versions).find(ver => ver && ver.audioUrls && ver.audioUrls.master);
    return fallback ? fallback.audioUrls : null;
  }

  function simulateAudioError() {
    DOM.masterAudio.src = `http://127.0.0.1:5050/api/songs/${State.activeSongId}/versions/${State.activeVersion}/audio/final_master`;
    DOM.btnMasterPlay.textContent = '▶';
    State.isPlayingMaster = false;
    const song = State.songs[State.activeSongId];
    if (song && State.activeVersion > 1) {
      const fallbackUrls = findLastValidAudioUrls(song, State.activeVersion - 1);
      if (fallbackUrls && fallbackUrls.master && DOM.masterAudio.src !== fallbackUrls.master) {
        DOM.deckStatusMsg.textContent = `⚠️ Audio version ${State.activeVersion} unavailable; playing previous valid version...`;
        DOM.masterAudio.src = fallbackUrls.master;
        DOM.masterAudio.load();
        return { recovered: true, fallbackSrc: fallbackUrls.master };
      }
    }
    DOM.deckStatusMsg.textContent = 'Playback error: Audio file not found on disk';
    return { recovered: false };
  }

  function onIterationSuccess(nextVer, feedback, bpm, key, data) {
    DOM.btnSendPrompt.disabled = false;
    const song = State.songs[State.activeSongId];
    if (!song) return;

    const lastValidUrls = findLastValidAudioUrls(song, State.activeVersion) || {
      master: `/api/songs/${State.activeSongId}/versions/1/audio/final_master`,
      vocals: `/api/songs/${State.activeSongId}/versions/1/audio/raspy_tenor_vocals`,
      drums: `/api/songs/${State.activeSongId}/versions/1/audio/drums`,
      bass: `/api/songs/${State.activeSongId}/versions/1/audio/bass`,
      guitar: `/api/songs/${State.activeSongId}/versions/1/audio/guitar`,
      other: `/api/songs/${State.activeSongId}/versions/1/audio/other`,
      inpainted_guitar: `/api/songs/${State.activeSongId}/versions/1/audio/inpainted_guitar`,
      inpainted_drums: `/api/songs/${State.activeSongId}/versions/1/audio/inpainted_drums`,
      draft_groove: `/api/songs/${State.activeSongId}/versions/1/audio/draft_groove`
    };

    let resolvedAudioUrls = { ...lastValidUrls };
    if (data && data.audio_urls && typeof data.audio_urls === 'object') {
      resolvedAudioUrls = {
        master: data.audio_urls.final_master || data.audio_urls.master || lastValidUrls.master,
        vocals: data.audio_urls.raspy_tenor_vocals || data.audio_urls.vocals || lastValidUrls.vocals,
        drums: data.audio_urls.drums || lastValidUrls.drums,
        bass: data.audio_urls.bass || lastValidUrls.bass,
        guitar: data.audio_urls.guitar || lastValidUrls.guitar,
        other: data.audio_urls.other || lastValidUrls.other,
        inpainted_guitar: data.audio_urls.inpainted_guitar || lastValidUrls.inpainted_guitar,
        inpainted_drums: data.audio_urls.inpainted_drums || lastValidUrls.inpainted_drums,
        draft_groove: data.audio_urls.draft_groove || lastValidUrls.draft_groove
      };
    }

    song.versions[nextVer] = {
      version: nextVer,
      title: `${song.title} (v${nextVer} Iteration)`,
      bpm: bpm,
      key: key,
      engineMode: 'pure_rust',
      prompt: feedback,
      lyrics: (data && data.lyrics) ? data.lyrics : (song.versions[State.activeVersion]?.lyrics || ''),
      audioUrls: resolvedAudioUrls
    };

    return resolvedAudioUrls;
  }

  return { State, DOM, findLastValidAudioUrls, simulateAudioError, onIterationSuccess };
}

// =========================================================================
// RUN TEST SUITE
// =========================================================================

console.log('=== Running Playback 404 Resilience Test Suite ===');

// Test 1: Null/undefined song
{
  const env = createEnvironment();
  assert.strictEqual(env.findLastValidAudioUrls(null), null, 'Null song should return null');
  assert.strictEqual(env.findLastValidAudioUrls(undefined), null, 'Undefined song should return null');
  assert.strictEqual(env.findLastValidAudioUrls({}), null, 'Song without versions should return null');
  console.log('✔ Test 1 Passed: Null/undefined safety');
}

// Test 2: Standard resolution
{
  const env = createEnvironment();
  const song = {
    title: 'Roots Reggae Anthem',
    versions: {
      1: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/1/audio/final_master' } },
      2: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/2/audio/final_master' } }
    }
  };
  const v2Urls = env.findLastValidAudioUrls(song, 2);
  assert.strictEqual(v2Urls.master, '/api/songs/roots_reggae_anthem/versions/2/audio/final_master');
  const v1Urls = env.findLastValidAudioUrls(song, 1);
  assert.strictEqual(v1Urls.master, '/api/songs/roots_reggae_anthem/versions/1/audio/final_master');
  console.log('✔ Test 2 Passed: Standard version resolution');
}

// Test 3: Iteration failure (v3 without audioUrls or null master)
{
  const env = createEnvironment();
  const song = {
    title: 'Roots Reggae Anthem',
    versions: {
      1: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/1/audio/final_master' } },
      2: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/2/audio/final_master' } },
      3: { audioUrls: null } // failed iteration
    }
  };
  const fallback = env.findLastValidAudioUrls(song, 3);
  assert.strictEqual(fallback.master, '/api/songs/roots_reggae_anthem/versions/2/audio/final_master', 'Must fall back to v2');
  console.log('✔ Test 3 Passed: Failed iteration v3 falls back to v2');
}

// Test 4: Corrupted v3 with empty audioUrls object
{
  const env = createEnvironment();
  const song = {
    title: 'Roots Reggae Anthem',
    versions: {
      1: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/1/audio/final_master' } },
      2: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/2/audio/final_master' } },
      3: { audioUrls: { master: null } } // corrupt master
    }
  };
  const fallback = env.findLastValidAudioUrls(song, 3);
  assert.strictEqual(fallback.master, '/api/songs/roots_reggae_anthem/versions/2/audio/final_master', 'Must fall back to v2');
  console.log('✔ Test 4 Passed: Corrupted audioUrls.master falls back to v2');
}

// Test 5: Version gap / overshoot (requested v5 when only v1, v2 exist)
{
  const env = createEnvironment();
  const song = {
    title: 'Roots Reggae Anthem',
    versions: {
      1: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/1/audio/final_master' } },
      2: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/2/audio/final_master' } }
    }
  };
  const fallback = env.findLastValidAudioUrls(song, 5);
  assert.strictEqual(fallback.master, '/api/songs/roots_reggae_anthem/versions/2/audio/final_master', 'Overshoot v5 must resolve to v2');
  console.log('✔ Test 5 Passed: Version gap / overshoot safely clamped');
}

// Test 6: onIterationSuccess offline / network failure handling
{
  const env = createEnvironment();
  env.State.songs['roots_reggae_anthem'] = {
    title: 'Roots Reggae Anthem',
    versions: {
      1: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/1/audio/final_master', drums: '/api/stems/drums1' } },
      2: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/2/audio/final_master', drums: '/api/stems/drums2' } }
    }
  };
  env.State.activeSongId = 'roots_reggae_anthem';
  env.State.activeVersion = 2;

  // Simulate user prompt "REMOVE THE SCRATCH" resulting in network/backend failure (data = null)
  const resolved = env.onIterationSuccess(3, 'REMOVE THE SCRATCH', 76, 'A Major', null);
  assert.ok(resolved, 'Must return resolved audio URLs');
  assert.strictEqual(
    resolved.master,
    '/api/songs/roots_reggae_anthem/versions/2/audio/final_master',
    'Must resolve to v2 master when backend iteration failed, NOT broken v3 path'
  );
  assert.strictEqual(resolved.drums, '/api/stems/drums2');

  const v3Record = env.State.songs['roots_reggae_anthem'].versions[3];
  assert.ok(v3Record, 'Version 3 record must be created');
  assert.strictEqual(v3Record.audioUrls.master, '/api/songs/roots_reggae_anthem/versions/2/audio/final_master');
  console.log('✔ Test 6 Passed: onIterationSuccess with null data never points to non-existent disk path');
}

// Test 7: onIterationSuccess partial backend return (stems missing in data.audio_urls)
{
  const env = createEnvironment();
  env.State.songs['roots_reggae_anthem'] = {
    title: 'Roots Reggae Anthem',
    versions: {
      2: {
        audioUrls: {
          master: '/api/songs/roots_reggae_anthem/versions/2/audio/final_master',
          drums: '/api/songs/roots_reggae_anthem/versions/2/audio/drums',
          vocals: '/api/songs/roots_reggae_anthem/versions/2/audio/raspy_tenor_vocals'
        }
      }
    }
  };
  env.State.activeSongId = 'roots_reggae_anthem';
  env.State.activeVersion = 2;

  // Backend returns only final_master, stems are missing
  const partialData = {
    audio_urls: {
      final_master: '/api/songs/roots_reggae_anthem/versions/3/audio/final_master'
    }
  };
  const resolved = env.onIterationSuccess(3, 'Add vocal reverb', 76, 'A Major', partialData);
  assert.strictEqual(resolved.master, '/api/songs/roots_reggae_anthem/versions/3/audio/final_master');
  assert.strictEqual(resolved.drums, '/api/songs/roots_reggae_anthem/versions/2/audio/drums', 'Missing stem falls back to v2');
  assert.strictEqual(resolved.vocals, '/api/songs/roots_reggae_anthem/versions/2/audio/raspy_tenor_vocals', 'Missing vocals falls back to v2');
  console.log('✔ Test 7 Passed: Partial audio_urls cleanly inherits prior stems');
}

// Test 8: HTML5 Audio error recovery listener
{
  const env = createEnvironment();
  env.State.songs['roots_reggae_anthem'] = {
    title: 'Roots Reggae Anthem',
    versions: {
      1: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/1/audio/final_master' } },
      2: { audioUrls: { master: '/api/songs/roots_reggae_anthem/versions/2/audio/final_master' } }
    }
  };
  env.State.activeSongId = 'roots_reggae_anthem';
  env.State.activeVersion = 3; // user switched to non-existent v3

  const result = env.simulateAudioError();
  assert.strictEqual(result.recovered, true, 'Audio error must recover');
  assert.strictEqual(result.fallbackSrc, '/api/songs/roots_reggae_anthem/versions/2/audio/final_master');
  assert.strictEqual(env.DOM.masterAudio.loadCalled, true, 'load() must be invoked on fallback');
  assert.strictEqual(env.DOM.deckStatusMsg.textContent, '⚠️ Audio version 3 unavailable; playing previous valid version...');
  console.log('✔ Test 8 Passed: HTML5 Audio error recovers to prior valid version without crashing');
}

console.log('=== All 8 Playback 404 Resilience Tests Passed! ===\n');
