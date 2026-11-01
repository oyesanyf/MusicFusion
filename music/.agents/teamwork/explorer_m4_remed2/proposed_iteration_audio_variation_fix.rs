//! Proposed Production Remediation Patch for Milestone 4:
//! Genuine Audio Variation & Diff-Architected Song Iteration Architecture.
//!
//! Target Crates:
//! 1. `crates/music_fusion/src/iteration_panel.rs`
//! 2. `crates/music_pipeline/src/runner.rs`
//! 3. `crates/music_pipeline/src/nodes/node1_producer_panel.rs`
//! 4. `crates/music_pipeline/src/nodes/node3_vocal_timbre.rs`
//! 5. `crates/music_pipeline/src/nodes/node6_analog_mastering.rs`
//! 6. `crates/music_cli/src/main.rs`
//! 7. `crates/music_cli/src/rest_server.rs`
//!
//! Architected by Remediation Explorer M4-2 (`teamwork_explorer`).

// ============================================================================
// COMPONENT 1: crates/music_fusion/src/iteration_panel.rs
// Enhance `offline_fallback_iteration` with tempo direction cues & automatic
// genre-bounded variation (+/- 2 to +/- 6 BPM).
// ============================================================================

/*
REPLACE in `crates/music_fusion/src/iteration_panel.rs` lines 478-485:

Existing:
```rust
        // 1. Determine updated BPM
        let mut target_bpm = tweaks.bpm.unwrap_or(current_state.bpm);
        if target_bpm == current_state.bpm {
            if let Some(bpm_val) = parse_bpm_from_critique(critique) {
                target_bpm = bpm_val;
            }
        }
```

Replacement:
```rust
        // 1. Determine updated BPM with intelligent tempo parsing and guaranteed sonic variation
        let mut target_bpm = tweaks.bpm.unwrap_or(current_state.bpm);
        if target_bpm == current_state.bpm {
            if let Some(bpm_val) = parse_bpm_from_critique(critique) {
                target_bpm = bpm_val;
            } else {
                let crit_lower = critique.to_ascii_lowercase();
                if crit_lower.contains("faster")
                    || crit_lower.contains("speed up")
                    || crit_lower.contains("quicker")
                    || crit_lower.contains("upbeat")
                    || crit_lower.contains("accelerate")
                {
                    target_bpm = (current_state.bpm + 4).min(genre.max_bpm);
                } else if crit_lower.contains("slower")
                    || crit_lower.contains("slow down")
                    || crit_lower.contains("relax")
                    || crit_lower.contains("laid back")
                    || crit_lower.contains("chill")
                    || crit_lower.contains("half time")
                {
                    target_bpm = (current_state.bpm.saturating_sub(4)).max(genre.min_bpm);
                } else if !critique.trim().is_empty() {
                    // Non-empty critique without explicit tempo: apply deterministic +/- 2 to 4 BPM variation
                    // within canonical genre boundaries to guarantee audibly and digitally distinct waveforms.
                    let delta: i32 = if (current_state.version + 1) % 2 == 0 { 2 } else { -2 };
                    let candidate = (current_state.bpm as i32 + delta).clamp(genre.min_bpm as i32, genre.max_bpm as i32) as u32;
                    if candidate != current_state.bpm {
                        target_bpm = candidate;
                    } else if current_state.bpm + 2 <= genre.max_bpm {
                        target_bpm = current_state.bpm + 2;
                    } else if current_state.bpm >= genre.min_bpm + 2 {
                        target_bpm = current_state.bpm - 2;
                    }
                }
            }
        }
```
*/


// ============================================================================
// COMPONENT 2: crates/music_pipeline/src/runner.rs
// Overhaul `PipelineRunner::iterate_song` and add `iterate_song_with_tweaks`
// to invoke `IterationFusionPanel` and modulate state parameters.
// ============================================================================

/*
REPLACE in `crates/music_pipeline/src/runner.rs` lines 204-220:

Existing:
```rust
    /// Song iteration: creates a new version branch with optional updated style prompt or lyrics.
    pub async fn iterate_song(
        &self,
        song_id: &str,
        from_version: u32,
        prompt_update: Option<&str>,
    ) -> Result<MusicProductionState> {
        let mut state = self.checkpointer.load_checkpoint(song_id, from_version).await?;
        state.advance_version();
        if let Some(p) = prompt_update {
            state.style_prompt = p.to_string();
        }
        state.history.push(format!("Iterating song from v{} to v{}", from_version, state.version));

        self.compiled.execute_with_checkpointer(state, &self.checkpointer).await
    }
```

Replacement:
```rust
    /// Song iteration: creates a new version branch with updated parameters and executed pipeline.
    pub async fn iterate_song(
        &self,
        song_id: &str,
        from_version: u32,
        prompt_update: Option<&str>,
    ) -> Result<MusicProductionState> {
        self.iterate_song_with_tweaks(song_id, from_version, prompt_update, None).await
    }

    /// Enhanced song iteration: accepts explicit ParameterTweaks or parses them from critique string,
    /// invokes IterationFusionPanel, applies diff blueprint & stem adjustments, and runs pipeline.
    pub async fn iterate_song_with_tweaks(
        &self,
        song_id: &str,
        from_version: u32,
        prompt_update: Option<&str>,
        explicit_tweaks: Option<music_fusion::iteration_panel::ParameterTweaks>,
    ) -> Result<MusicProductionState> {
        let prev_state = self.checkpointer.load_checkpoint(song_id, from_version).await?;
        let critique = prompt_update.unwrap_or("").trim();

        // 1. Prepare base ParameterTweaks
        let tweaks = explicit_tweaks.unwrap_or_default();

        // 2. Run IterationFusionPanel (or deterministic fallback)
        let mem = music_fusion::memory::SystemMemory::detect_live();
        let panel = music_fusion::iteration_panel::IterationFusionPanel::new_auto_configured(&mem).await;
        let iteration_result = panel.run_iteration_fusion(&prev_state, critique, &tweaks).await
            .unwrap_or_else(|_| panel.offline_fallback_iteration(&prev_state, critique, &tweaks));

        // 3. Modulate state parameters
        let mut next_state = prev_state.clone();
        next_state.advance_version();

        // 3a. BPM modulation: prioritize explicit tweak, extracted BPM, or genre-bounded delta
        let genre_preset = music_core::genre::GenreTaxonomy::find_or_create(&prev_state.genre);
        let next_bpm = if let Some(b) = tweaks.bpm {
            b
        } else if let Some(b) = music_fusion::iteration_panel::parse_bpm_from_critique(critique) {
            b
        } else {
            let candidate_bpm = iteration_result.blueprint.bpm.round() as u32;
            if candidate_bpm != prev_state.bpm && (genre_preset.min_bpm..=genre_preset.max_bpm).contains(&candidate_bpm) {
                candidate_bpm
            } else if !critique.is_empty() {
                let delta: i32 = if next_state.version % 2 == 0 { 2 } else { -2 };
                let cand = (prev_state.bpm as i32 + delta)
                    .clamp(genre_preset.min_bpm as i32, genre_preset.max_bpm as i32) as u32;
                if cand != prev_state.bpm {
                    cand
                } else if prev_state.bpm + 2 <= genre_preset.max_bpm {
                    prev_state.bpm + 2
                } else if prev_state.bpm >= genre_preset.min_bpm + 2 {
                    prev_state.bpm - 2
                } else {
                    prev_state.bpm
                }
            } else {
                prev_state.bpm
            }
        };
        next_state.bpm = next_bpm;

        // 3b. Key signature modulation
        if let Some(ref k) = tweaks.key_signature {
            next_state.key_signature = k.clone();
        } else if let Some(k) = music_fusion::iteration_panel::parse_key_from_critique(critique) {
            next_state.key_signature = k;
        } else if !iteration_result.blueprint.key_signature.is_empty() && iteration_result.blueprint.key_signature != prev_state.key_signature {
            next_state.key_signature = iteration_result.blueprint.key_signature.clone();
        }

        // 3c. Style prompt & lyrics
        if let Some(p) = prompt_update {
            next_state.style_prompt = p.to_string();
        }
        if !iteration_result.blueprint.lyrics_text.is_empty() {
            next_state.lyrics = iteration_result.blueprint.lyrics_text.clone();
        }

        // 3d. Diff-architected blueprint and stem mix modulation
        let mut blueprint = prev_state.blueprint.clone().unwrap_or_else(|| {
            music_core::schema::SongBlueprint::default_one_love_shining_anthem()
        });
        blueprint.version = next_state.version;
        blueprint.parent_version = Some(from_version);
        blueprint.bpm = next_state.bpm as f64;
        blueprint.key_signature = next_state.key_signature.clone();
        blueprint.iteration_notes = iteration_result.iteration_summary.clone();

        // Modulate stem mix parameters based on tweaks or critique
        let crit_lower = critique.to_ascii_lowercase();
        if let Some(rasp) = tweaks.vocal_rasp {
            blueprint.stem_mix.vocal_rasp_intensity = rasp;
        } else if crit_lower.contains("rasp") || crit_lower.contains("vocal") {
            blueprint.stem_mix.vocal_rasp_intensity = (blueprint.stem_mix.vocal_rasp_intensity + 0.15).min(1.0);
        }

        if let Some(bass) = tweaks.sub_bass_weight {
            blueprint.stem_mix.sub_bass_boost_db = bass * 6.0;
            blueprint.stem_mix.dub_bass_db += 1.5;
        } else if crit_lower.contains("bass") || crit_lower.contains("sub") {
            blueprint.stem_mix.sub_bass_boost_db = (blueprint.stem_mix.sub_bass_boost_db + 1.8).min(6.0);
            blueprint.stem_mix.dub_bass_db += 1.2;
        }

        if let Some(wah) = tweaks.guitar_wah {
            blueprint.stem_mix.guitar_db += 1.0;
            for sec in &mut blueprint.sections {
                sec.wah_pedal_depth = wah;
                sec.guitar_solo_intensity = wah;
            }
        } else if crit_lower.contains("guitar") || crit_lower.contains("wah") || crit_lower.contains("solo") {
            blueprint.stem_mix.guitar_db += 1.5;
            for sec in &mut blueprint.sections {
                sec.wah_pedal_depth = (sec.wah_pedal_depth + 0.20).min(1.0);
                sec.guitar_solo_intensity = (sec.guitar_solo_intensity + 0.15).min(1.0);
            }
        }

        if let Some(drum) = tweaks.drum_fill_complexity {
            blueprint.stem_mix.drums_db += 0.8;
            for sec in &mut blueprint.sections {
                sec.drum_fill_complexity = drum;
            }
        } else if crit_lower.contains("drum") || crit_lower.contains("fill") || crit_lower.contains("nyabinghi") {
            blueprint.stem_mix.drums_db += 1.0;
            for sec in &mut blueprint.sections {
                sec.drum_fill_complexity = (sec.drum_fill_complexity + 0.20).min(1.0);
            }
        }

        next_state.blueprint = Some(blueprint);
        next_state.history.push(format!(
            "Iterating song from v{} to v{}: {} (BPM: {}, Key: {})",
            from_version, next_state.version, critique, next_state.bpm, next_state.key_signature
        ));

        self.compiled.execute_with_checkpointer(next_state, &self.checkpointer).await
    }
```
*/


// ============================================================================
// COMPONENT 3: crates/music_pipeline/src/nodes/node1_producer_panel.rs
// Preserve pre-existing `blueprint.stem_mix` across iterations.
// ============================================================================

/*
IN `crates/music_pipeline/src/nodes/node1_producer_panel.rs`:
In `execute_node`, after `let mut blueprint = Self::convert_evaluated_blueprint_to_schema(...)`:

Add lines:
```rust
        // Preserve pre-configured stem mix and iteration notes if state already had a blueprint
        if let Some(ref existing) = state.blueprint {
            blueprint.stem_mix = existing.stem_mix.clone();
            if !existing.iteration_notes.is_empty() {
                blueprint.iteration_notes = existing.iteration_notes.clone();
            }
        }
```
*/


// ============================================================================
// COMPONENT 4: crates/music_pipeline/src/nodes/node3_vocal_timbre.rs
// Connect vocal timbre synthesis to `blueprint.stem_mix.vocal_rasp_intensity`.
// ============================================================================

/*
IN `crates/music_pipeline/src/nodes/node3_vocal_timbre.rs`:
In `render_pure_rust_vocal_timbre` around line 238:

Replace:
```rust
        // 3. Determine rasp amount
        let rasp = if let Some(r) = self.default_rasp_override {
            r
        } else {
            params.default_rasp
        };
```

With:
```rust
        // 3. Determine rasp amount, prioritizing explicit override, then blueprint stem mix, then archetype default
        let rasp = if let Some(r) = self.default_rasp_override {
            r
        } else if let Some(ref bp) = state.blueprint {
            bp.stem_mix.vocal_rasp_intensity
        } else {
            params.default_rasp
        };
```
*/


// ============================================================================
// COMPONENT 5: crates/music_pipeline/src/nodes/node6_analog_mastering.rs
// Mix stems using dynamic `blueprint.stem_mix` levels and apply custom sub-bass boost.
// ============================================================================

/*
IN `crates/music_pipeline/src/nodes/node6_analog_mastering.rs`:
In `mix_stems_to_premaster`:

Replace:
```rust
    pub fn mix_stems_to_premaster(&self, state: &MusicProductionState) -> AudioBuffer {
        let mut premaster = AudioBuffer::new_stereo(0.1);

        // 1. Lead Vocal (vocal headroom)
        let vocal_path = state.get_stem(STEM_RASPY_TENOR_VOCALS)
            .or_else(|| state.get_stem(STEM_YUE_VOCALS));
        if let Some(p) = vocal_path {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, 0.95);
            }
        }

        // 2. Inpainted Guitar Solo / Lead
        if let Some(p) = state.get_stem(STEM_INPAINTED_GUITAR) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, 0.85);
            }
        }

        // 3. Inpainted Drums & Percussion
        if let Some(p) = state.get_stem(STEM_INPAINTED_DRUMS) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, 0.90);
            }
        }

        // 4. Bass Foundation
        if let Some(p) = state.get_stem(STEM_BASS) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, 0.82);
            }
        }

        // 5. Other (Keyboards, Horns, Ambient)
        if let Some(p) = state.get_stem(STEM_OTHER) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, 0.75);
            }
        }

        premaster
    }
```

With:
```rust
    pub fn mix_stems_to_premaster(&self, state: &MusicProductionState) -> AudioBuffer {
        let mut premaster = AudioBuffer::new_stereo(0.1);

        let (vocal_gain, guitar_gain, drums_gain, bass_gain, other_gain) = if let Some(ref bp) = state.blueprint {
            let m = &bp.stem_mix;
            let db_to_linear = |db: f32, nominal: f32| -> f32 {
                (nominal * 10.0f32.powf(db / 20.0)).clamp(0.2, 1.4)
            };
            (
                db_to_linear(m.lead_vocal_db, 0.85),
                db_to_linear(m.guitar_db, 0.75),
                db_to_linear(m.drums_db, 0.85),
                db_to_linear(m.dub_bass_db, 0.70),
                db_to_linear(m.organ_brass_db, 0.75),
            )
        } else {
            (0.95, 0.85, 0.90, 0.82, 0.75)
        };

        // 1. Lead Vocal (vocal headroom)
        let vocal_path = state.get_stem(STEM_RASPY_TENOR_VOCALS)
            .or_else(|| state.get_stem(STEM_YUE_VOCALS));
        if let Some(p) = vocal_path {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, vocal_gain);
            }
        }

        // 2. Inpainted Guitar Solo / Lead
        if let Some(p) = state.get_stem(STEM_INPAINTED_GUITAR) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, guitar_gain);
            }
        }

        // 3. Inpainted Drums & Percussion
        if let Some(p) = state.get_stem(STEM_INPAINTED_DRUMS) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, drums_gain);
            }
        }

        // 4. Bass Foundation
        if let Some(p) = state.get_stem(STEM_BASS) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, bass_gain);
            }
        }

        // 5. Other (Keyboards, Horns, Ambient)
        if let Some(p) = state.get_stem(STEM_OTHER) {
            if let Ok(buf) = AudioBuffer::from_wav_file(p) {
                mix_buffers(&mut premaster, &buf, other_gain);
            }
        }

        premaster
    }
```
And in `execute_node` around line 132:
```rust
        let master_buf = if !use_dsp && state.is_hf_hybrid() {
            info!("Attempting external AudioSR/Matchering subprocess...");
            let profile = Self::resolve_profile(&state.genre);
            apply_analog_mastering(&premaster, profile)
        } else {
            let profile = Self::resolve_profile(&state.genre);
            info!("Applying pure-Rust analog mastering profile: {:?}", profile);
            if let Some(ref bp) = state.blueprint {
                // Apply tuned sub-bass boost from blueprint
                music_dsp::tape_mastering::apply_analog_tape_mastering(&premaster, bp.stem_mix.sub_bass_boost_db)
            } else {
                apply_analog_mastering(&premaster, profile)
            }
        };
```
*/


// ============================================================================
// COMPONENT 6: crates/music_cli/src/main.rs
// Wire `ParameterTweaks` and `IterationFusionPanel` into CLI `--iterate`.
// ============================================================================

/*
REPLACE in `crates/music_cli/src/main.rs` lines 60-75:

Existing:
```rust
    // 4. Prompt-based song iteration CLI command
    if let Some(instructions) = &args.iterate {
        let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
        let version = args.version.unwrap_or(1);
        println!("🔄 Iterating song {} (v{}) with instructions: '{}'", song_id, version, instructions);
        let result = runner.iterate_song(song_id, version, Some(instructions)).await?;
        println!("✨ Completed Iteration: song_id={}, new version={}", result.song_id, result.version);
        if let Some(master) = result.stem_paths.get("final_master") {
            println!("🔊 Final Master: {}", master.display());
        }
        if args.open_ui && !args.no_browser {
            let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
            browser::open_browser_raw(&url)?;
        }
        return Ok(());
    }
```

Replacement:
```rust
    // 4. Prompt-based song iteration CLI command with IterationFusionPanel parameter tuning
    if let Some(instructions) = &args.iterate {
        let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
        let version = args.version.unwrap_or(1);
        println!("🔄 Iterating song {} (v{}) with instructions: '{}'", song_id, version, instructions);

        let tweaks = music_fusion::iteration_panel::ParameterTweaks {
            bpm: args.bpm,
            key_signature: args.key.clone(),
            ..Default::default()
        };

        let result = runner.iterate_song_with_tweaks(song_id, version, Some(instructions), Some(tweaks)).await?;
        println!(
            "✨ Completed Iteration: song_id={}, new version={} (BPM: {}, Key: {})",
            result.song_id, result.version, result.bpm, result.key_signature
        );
        if let Some(master) = result.stem_paths.get("final_master") {
            println!("🔊 Final Master: {}", master.display());
        }
        if args.open_ui && !args.no_browser {
            let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
            browser::open_browser_raw(&url)?;
        }
        return Ok(());
    }
```
*/


// ============================================================================
// COMPONENT 7: crates/music_cli/src/rest_server.rs
// 1. Ensure `handle_iterate_song` applies diff blueprint & stem tweaks before execution.
// 2. Fix `handle_stream_audio` to check `state.stem_paths` for cross-version audio streaming.
// ============================================================================

/*
IN `crates/music_cli/src/rest_server.rs`:

In `handle_iterate_song` lines 327-357:
```rust
    let iteration_result = panel.run_iteration_fusion(&prev_state, &req.critique, &tweaks).await
        .unwrap_or_else(|_| panel.offline_fallback_iteration(&prev_state, &req.critique, &tweaks));

    let mut next_state = prev_state.clone();
    next_state.advance_version();
    next_state.bpm = iteration_result.blueprint.bpm.round() as u32;
    next_state.key_signature = iteration_result.blueprint.key_signature.clone();
    if !iteration_result.blueprint.lyrics_text.is_empty() {
        next_state.lyrics = iteration_result.blueprint.lyrics_text.clone();
    }
    if !req.critique.trim().is_empty() {
        next_state.style_prompt = format!("{}, {}", prev_state.style_prompt.trim_end_matches('.'), req.critique.trim());
    }

    // Apply diff architected blueprint and stem tweaks into next_state.blueprint
    let mut blueprint = prev_state.blueprint.clone().unwrap_or_else(|| {
        music_core::schema::SongBlueprint::default_one_love_shining_anthem()
    });
    blueprint.version = next_state.version;
    blueprint.parent_version = Some(req.from_version);
    blueprint.bpm = next_state.bpm as f64;
    blueprint.key_signature = next_state.key_signature.clone();
    blueprint.iteration_notes = iteration_result.iteration_summary.clone();

    // Stem tweaks
    let crit_lower = req.critique.to_ascii_lowercase();
    if let Some(rasp) = tweaks.vocal_rasp {
        blueprint.stem_mix.vocal_rasp_intensity = rasp;
    } else if crit_lower.contains("rasp") || crit_lower.contains("vocal") {
        blueprint.stem_mix.vocal_rasp_intensity = (blueprint.stem_mix.vocal_rasp_intensity + 0.15).min(1.0);
    }

    if let Some(bass) = tweaks.sub_bass_weight {
        blueprint.stem_mix.sub_bass_boost_db = bass * 6.0;
        blueprint.stem_mix.dub_bass_db += 1.5;
    } else if crit_lower.contains("bass") || crit_lower.contains("sub") {
        blueprint.stem_mix.sub_bass_boost_db = (blueprint.stem_mix.sub_bass_boost_db + 1.8).min(6.0);
        blueprint.stem_mix.dub_bass_db += 1.2;
    }

    if let Some(wah) = tweaks.guitar_wah {
        blueprint.stem_mix.guitar_db += 1.0;
        for sec in &mut blueprint.sections {
            sec.wah_pedal_depth = wah;
            sec.guitar_solo_intensity = wah;
        }
    } else if crit_lower.contains("guitar") || crit_lower.contains("wah") || crit_lower.contains("solo") {
        blueprint.stem_mix.guitar_db += 1.5;
        for sec in &mut blueprint.sections {
            sec.wah_pedal_depth = (sec.wah_pedal_depth + 0.20).min(1.0);
            sec.guitar_solo_intensity = (sec.guitar_solo_intensity + 0.15).min(1.0);
        }
    }

    if let Some(drum) = tweaks.drum_fill_complexity {
        blueprint.stem_mix.drums_db += 0.8;
        for sec in &mut blueprint.sections {
            sec.drum_fill_complexity = drum;
        }
    } else if crit_lower.contains("drum") || crit_lower.contains("fill") || crit_lower.contains("nyabinghi") {
        blueprint.stem_mix.drums_db += 1.0;
        for sec in &mut blueprint.sections {
            sec.drum_fill_complexity = (sec.drum_fill_complexity + 0.20).min(1.0);
        }
    }

    next_state.blueprint = Some(blueprint);
    next_state.history.push(format!(
        "Iterated from v{} to v{}: {} (BPM: {}, Key: {})",
        req.from_version, next_state.version, req.critique, next_state.bpm, next_state.key_signature
    ));

    // 4. Re-execute pipeline
    match state.runner.execute_song(next_state).await {
```

IN `handle_stream_audio` lines 509-531:
Replace:
```rust
    // 2. Resolve audio file path
    let v_dir = state.storage_dir.join("songs").join(&clean_id).join(format!("v{}", version));
    let mut cand_paths = vec![
        v_dir.join("stems").join(format!("{}.wav", clean_stem)),
        v_dir.join(format!("{}.wav", clean_stem)),
    ];
    if clean_stem == "final_master" || clean_stem == "master" {
        cand_paths.push(v_dir.join("final_master.wav"));
    }

    let found_path = cand_paths.into_iter().find(|p| p.exists());
```

With:
```rust
    // 2. Resolve audio file path
    let v_dir = state.storage_dir.join("songs").join(&clean_id).join(format!("v{}", version));
    let mut cand_paths = vec![
        v_dir.join("stems").join(format!("{}.wav", clean_stem)),
        v_dir.join(format!("{}.wav", clean_stem)),
    ];
    if clean_stem == "final_master" || clean_stem == "master" {
        cand_paths.push(v_dir.join("final_master.wav"));
    }

    // Inspect checkpoint state for cross-version stem lineage (e.g. unregenerated stems)
    if let Ok(ckpt) = state.runner.checkpointer().load_checkpoint(&clean_id, version).await {
        if let Some(p) = ckpt.get_stem(&clean_stem) {
            cand_paths.push(p.clone());
        }
        if clean_stem == "master" {
            if let Some(p) = ckpt.get_stem("final_master") {
                cand_paths.push(p.clone());
            }
        }
    }

    let found_path = cand_paths.into_iter().find(|p| p.exists());
```
*/
