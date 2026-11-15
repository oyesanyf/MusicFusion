//! Dual-Mode Studio Lyricist Engine:
//! 1. Mode A (`UserProvided`): Strict verbatim preservation & structural section mapping (zero 6x expansion, zero hardcoded Reggae injection).
//! 2. Mode B (`AiGenerated`): Local Ollama lyric composition (`qwen2.5` / `llama3` / `gemma`) with genre-matched multi-genre offline composer fallback.

use serde::{Deserialize, Serialize};

use crate::memory::SystemMemory;
use crate::ollama_client::OllamaClient;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedLyricSection {
    pub tag: String,
    pub display_label: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuredLyricsResult {
    pub mode_used: String,
    pub model_used: String,
    pub title: String,
    pub formatted_lyrics: String,
    pub sections: Vec<ParsedLyricSection>,
    pub preserved_verbatim: bool,
}

fn sanitize_tag(raw: &str) -> (String, String) {
    let trimmed = raw
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(':')
        .next()
        .unwrap_or(raw)
        .trim();
    let label = if trimmed.is_empty() {
        "Verse".to_string()
    } else {
        trimmed.to_string()
    };
    let lower = format!("[{}]", label.to_ascii_lowercase());
    (lower, label)
}

fn is_section_header(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('[') && t.ends_with(']') && t.len() >= 3 && t.len() <= 64
}

/// Parses structured lyrics (with or without `[Section]` tags) into clean sections without modifying lyric words.
pub fn parse_lyrics_sections(text: &str) -> Vec<ParsedLyricSection> {
    let normalized = text.replace("\r\n", "\n");
    let raw_lines: Vec<&str> = normalized.lines().collect();
    let has_tags = raw_lines.iter().any(|l| is_section_header(l));

    if has_tags {
        let mut sections: Vec<ParsedLyricSection> = Vec::new();
        let mut cur_tag = "[intro]".to_string();
        let mut cur_label = "Intro".to_string();
        let mut cur_lines: Vec<String> = Vec::new();
        let mut seen_any_header = false;

        for line in raw_lines {
            let trimmed = line.trim();
            if is_section_header(trimmed) {
                if seen_any_header || !cur_lines.is_empty() {
                    sections.push(ParsedLyricSection {
                        tag: cur_tag.clone(),
                        display_label: cur_label.clone(),
                        lines: std::mem::take(&mut cur_lines),
                    });
                }
                let (t, l) = sanitize_tag(trimmed);
                cur_tag = t;
                cur_label = l;
                seen_any_header = true;
            } else if !trimmed.is_empty() {
                if !seen_any_header {
                    cur_tag = "[verse 1]".to_string();
                    cur_label = "Verse 1".to_string();
                    seen_any_header = true;
                }
                cur_lines.push(trimmed.to_string());
            }
        }
        if seen_any_header || !cur_lines.is_empty() {
            sections.push(ParsedLyricSection {
                tag: cur_tag,
                display_label: cur_label,
                lines: cur_lines,
            });
        }
        return sections;
    }

    // No explicit [Tag] headers provided: group raw stanzas verbatim into Verse / Chorus / Bridge / Outro
    let mut blocks: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in raw_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(trimmed.to_string());
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    // If user pasted a single solid block of lines (>= 8 lines), gently group into 4-line stanzas without changing any words
    if blocks.len() == 1 && blocks[0].len() >= 8 {
        let all = blocks.remove(0);
        for chunk in all.chunks(4) {
            blocks.push(chunk.to_vec());
        }
    }

    let labels = [
        ("[verse 1]", "Verse 1"),
        ("[chorus]", "Chorus"),
        ("[verse 2]", "Verse 2"),
        ("[bridge]", "Bridge"),
        ("[outro]", "Outro"),
    ];

    blocks
        .into_iter()
        .enumerate()
        .map(|(idx, lines)| {
            let (tag, label) = if idx < labels.len() {
                (labels[idx].0.to_string(), labels[idx].1.to_string())
            } else {
                let n = idx + 1;
                (format!("[verse {}]", n), format!("Verse {}", n))
            };
            ParsedLyricSection {
                tag,
                display_label: label,
                lines,
            }
        })
        .collect()
}

/// Formats parsed sections cleanly for ACE-Step 1.5, YuE2, DiffRhythm2, and WebUI editing.
pub fn format_sections(sections: &[ParsedLyricSection]) -> String {
    let mut blocks = Vec::with_capacity(sections.len());
    for sec in sections {
        let header = format!("[{}]", sec.display_label.trim());
        if sec.lines.is_empty() {
            blocks.push(header);
        } else {
            blocks.push(format!("{}\n{}", header, sec.lines.join("\n")));
        }
    }
    if blocks.is_empty() {
        String::new()
    } else {
        format!("{}\n", blocks.join("\n\n"))
    }
}

/// Mode A: Accepts user-supplied lyrics cleanly and preserves EVERY line 100% verbatim.
/// Never multiplies lines 6x and never injects hardcoded genre filler.
pub fn structure_user_lyrics_verbatim(raw_lyrics: &str) -> StructuredLyricsResult {
    let cleaned = raw_lyrics.replace("\r\n", "\n").trim().to_string();
    let sections = parse_lyrics_sections(&cleaned);
    let formatted_lyrics = if cleaned.is_empty() {
        String::new()
    } else {
        format_sections(&sections)
    };
    let first_line_title = sections
        .iter()
        .flat_map(|s| s.lines.iter())
        .next()
        .map(|l| {
            let words: Vec<&str> = l.split_whitespace().take(5).collect();
            let raw_t = words.join(" ").trim_matches(|c: char| !c.is_alphanumeric() && c != ' ').to_string();
            if raw_t.is_empty() { "Custom Vocal Song".to_string() } else { raw_t }
        })
        .unwrap_or_else(|| "Custom Vocal Song".to_string());

    StructuredLyricsResult {
        mode_used: "user_verbatim".to_string(),
        model_used: "VerbatimPassThrough".to_string(),
        title: first_line_title,
        formatted_lyrics,
        sections,
        preserved_verbatim: true,
    }
}

fn sanitize_generated_lyrics(raw: &str) -> String {
    let mut clean = if let Some(idx) = raw.find("</think>") {
        raw[idx + 8..].to_string()
    } else {
        raw.to_string()
    };
    clean = clean.replace("\r\n", "\n").trim().to_string();
    for fence in ["```text", "```lyrics", "```markdown", "```"] {
        if clean.starts_with(fence) {
            clean = clean.trim_start_matches(fence).trim().to_string();
        }
    }
    clean = clean.trim_end_matches("```").trim().to_string();

    // Strip meta commentary lines like "Here are the lyrics..." before the first [Section]
    if let Some(first_bracket) = clean.find('[') {
        clean = clean[first_bracket..].to_string();
    }
    clean
}

/// Mode B: Generates full structured song lyrics from a story/topic + musical style prompt.
/// Uses local Ollama when available, and otherwise falls back cleanly to a genre-aware multi-genre composer.
pub async fn write_lyrics_from_prompt(
    client: &OllamaClient,
    memory: &SystemMemory,
    topic: &str,
    style_prompt: &str,
    preferred_model: Option<&str>,
) -> StructuredLyricsResult {
    let effective_topic = if topic.trim().is_empty() {
        if style_prompt.trim().is_empty() {
            "Chasing horizons and finding renewal"
        } else {
            style_prompt.trim()
        }
    } else {
        topic.trim()
    };

    if client.probe_health().await {
        let installed = client.list_installed_tags().await.unwrap_or_default();
        let chosen_model = preferred_model
            .filter(|m| !m.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| memory.recommend_ollama_model(&installed));

        let system = "You are a Grammy-winning multi-genre songwriter. Write complete, original, singable song lyrics structured strictly with tags: [Intro], [Verse 1], [Chorus], [Verse 2], [Bridge], [Instrumental Break], [Outro]. Output ONLY the song lyrics starting immediately with [Intro] or [Verse 1]. No explanations, no prose commentary.";
        let user = format!(
            "Musical Style / Genre: {}\nSong Concept / Topic / Imagery: {}\nWrite vivid, rhythmic, emotionally resonant lyrics tuned specifically to this genre and tempo.",
            style_prompt.trim(),
            effective_topic
        );

        if let Ok(raw) = client.simple_chat(&chosen_model, system, &user, 0.78).await {
            let cleaned = sanitize_generated_lyrics(&raw);
            let sections = parse_lyrics_sections(&cleaned);
            let has_lyric_lines = sections.iter().any(|s| !s.lines.is_empty());
            if has_lyric_lines && sections.len() >= 2 {
                let title = derive_title_from_topic(effective_topic, style_prompt);
                return StructuredLyricsResult {
                    mode_used: "ai_ollama".to_string(),
                    model_used: chosen_model,
                    title,
                    formatted_lyrics: format_sections(&sections),
                    sections,
                    preserved_verbatim: false,
                };
            }
        }
    }

    compose_offline_structured_lyrics(effective_topic, style_prompt)
}

/// Genre-aware deterministic offline songwriter used when Ollama is stopped or unloaded.
///Adapts motifs, cadence, and instrumentation breaks to the requested genre rather than repeating one preset.
pub fn compose_offline_structured_lyrics(topic: &str, style_prompt: &str) -> StructuredLyricsResult {
    let style_low = style_prompt.to_ascii_lowercase();
    let topic_clean = if topic.trim().is_empty() {
        "midnight skyline and open roads"
    } else {
        topic.trim()
    };

    let solo_label = if style_low.contains("synth") || style_low.contains("edm") || style_low.contains("house") || style_low.contains("cyber") {
        "Synth Solo"
    } else if style_low.contains("jazz") || style_low.contains("soul") || style_low.contains("brass") {
        "Saxophone Solo"
    } else if style_low.contains("piano") || style_low.contains("ballad") || style_low.contains("classical") {
        "Piano Interlude"
    } else {
        "Guitar Solo"
    };

    let hook_focus = topic_clean
        .split(['.', ',', '\n'])
        .next()
        .unwrap_or(topic_clean)
        .trim();

    let sections = if style_low.contains("synth") || style_low.contains("cyber") || style_low.contains("electronic") || style_low.contains("pop") {
        vec![
            ParsedLyricSection { tag: "[intro]".into(), display_label: "Intro".into(), lines: vec![] },
            ParsedLyricSection {
                tag: "[verse 1]".into(),
                display_label: "Verse 1".into(),
                lines: vec![
                    format!("Silver streetlights shimmer on the boulevard tonight,"),
                    format!("Carrying echoes of {} in the neon light,", hook_focus),
                    format!("Radio static fades as the pulse begins to climb,"),
                    format!("Chasing every second on the edge of turning time."),
                ],
            },
            ParsedLyricSection {
                tag: "[chorus]".into(),
                display_label: "Chorus".into(),
                lines: vec![
                    format!("Hold on to the signal cutting clear across the sky,"),
                    format!("Feel the rhythm lift us while the city rushes by,"),
                    format!("Every chord is burning bright for {},", hook_focus),
                    format!("Tonight we own the horizon, never looking down again."),
                ],
            },
            ParsedLyricSection {
                tag: "[verse 2]".into(),
                display_label: "Verse 2".into(),
                lines: vec![
                    format!("Velvet basslines roll beneath the glass and steel skyline,"),
                    format!("Tracing every memory where our open futures shine,"),
                    format!("No static in the frequency, the harmony is true,"),
                    format!("Stepping through the static into something bold and new."),
                ],
            },
            ParsedLyricSection { tag: format!("[{}]", solo_label.to_ascii_lowercase()), display_label: solo_label.into(), lines: vec![] },
            ParsedLyricSection {
                tag: "[bridge]".into(),
                display_label: "Bridge".into(),
                lines: vec![
                    format!("Let the drums drop back until we hear our heartbeat speak,"),
                    format!("Every quiet whisper finds the answer that we seek."),
                ],
            },
            ParsedLyricSection {
                tag: "[outro]".into(),
                display_label: "Outro".into(),
                lines: vec![
                    format!("Glowing in the afterlight of {},", hook_focus),
                    format!("Fading with the melody until the morning breaks."),
                ],
            },
        ]
    } else if style_low.contains("rock") || style_low.contains("metal") || style_low.contains("indie") || style_low.contains("blues") {
        vec![
            ParsedLyricSection { tag: "[intro]".into(), display_label: "Intro".into(), lines: vec![] },
            ParsedLyricSection {
                tag: "[verse 1]".into(),
                display_label: "Verse 1".into(),
                lines: vec![
                    format!("Dust upon the amplifier, thunder in the wire,"),
                    format!("Singing out the story of {} through the fire,", hook_focus),
                    format!("Bootsteps hit the wooden stage before the spotlight lands,"),
                    format!("Holding onto thunder with our scarred and steady hands."),
                ],
            },
            ParsedLyricSection {
                tag: "[chorus]".into(),
                display_label: "Chorus".into(),
                lines: vec![
                    format!("Turn it up and let the heavy chorus shake the floor,"),
                    format!("For {} we are knocking down the door,", hook_focus),
                    format!("Echoes ring out louder than the doubts we left behind,"),
                    format!("Living in the music with an undefeated mind."),
                ],
            },
            ParsedLyricSection {
                tag: "[verse 2]".into(),
                display_label: "Verse 2".into(),
                lines: vec![
                    format!("Highway miles behind us and a storm cloud on the ridge,"),
                    format!("Every broken string becomes another burning bridge,"),
                    format!("Snare crack cuts the midnight like a flare above the bay,"),
                    format!("Nothing stops the anthem once the band begins to play."),
                ],
            },
            ParsedLyricSection { tag: "[guitar solo]".into(), display_label: "Guitar Solo".into(), lines: vec![] },
            ParsedLyricSection {
                tag: "[bridge]".into(),
                display_label: "Bridge".into(),
                lines: vec![
                    format!("Breathe before the final chord crashes through the frame,"),
                    format!("Remember why we started and the fire we came to claim."),
                ],
            },
            ParsedLyricSection {
                tag: "[outro]".into(),
                display_label: "Outro".into(),
                lines: vec![
                    format!("Feedback humming softly at the edge of victory,"),
                    format!("Standing in the echo of where we chose to be."),
                ],
            },
        ]
    } else if style_low.contains("reggae") || style_low.contains("dub") {
        vec![
            ParsedLyricSection { tag: "[intro]".into(), display_label: "Intro".into(), lines: vec![] },
            ParsedLyricSection {
                tag: "[verse 1]".into(),
                display_label: "Verse 1".into(),
                lines: vec![
                    format!("Steady one-drop groove beneath the warm meridian sun,"),
                    format!("Carrying the message of {} to everyone,", hook_focus),
                    format!("Deep sub-bass rolling gentle like a tide across the bay,"),
                    format!("Sweeping heavy shadows and the weariness away."),
                ],
            },
            ParsedLyricSection {
                tag: "[chorus]".into(),
                display_label: "Chorus".into(),
                lines: vec![
                    format!("Stand strong together as the melody flows free,"),
                    format!("{} rising up in natural harmony,", hook_focus),
                    format!("Rhythm in the footsteps and truth inside the song,"),
                    format!("Light is shining brighter when the spirit stays so strong."),
                ],
            },
            ParsedLyricSection {
                tag: "[verse 2]".into(),
                display_label: "Verse 2".into(),
                lines: vec![
                    format!("Skank guitar and organ bubble drifting through the breeze,"),
                    format!("Bringing peace of mind among the tall and swaying trees,"),
                    format!("Every note is grounded in patience and in grace,"),
                    format!("Building up a sanctuary time can not erase."),
                ],
            },
            ParsedLyricSection { tag: format!("[{}]", solo_label.to_ascii_lowercase()), display_label: solo_label.into(), lines: vec![] },
            ParsedLyricSection {
                tag: "[bridge]".into(),
                display_label: "Bridge".into(),
                lines: vec![
                    format!("Hear the horn section lifting high above the town,"),
                    format!("No storm can turn a rooted spirit upside down."),
                ],
            },
            ParsedLyricSection {
                tag: "[outro]".into(),
                display_label: "Outro".into(),
                lines: vec![
                    format!("Let the tape echo carry {} wide and far,", hook_focus),
                    format!("Shining steady till the morning star."),
                ],
            },
        ]
    } else {
        vec![
            ParsedLyricSection { tag: "[intro]".into(), display_label: "Intro".into(), lines: vec![] },
            ParsedLyricSection {
                tag: "[verse 1]".into(),
                display_label: "Verse 1".into(),
                lines: vec![
                    format!("Golden hour settles quiet where the story starts,"),
                    format!("Holding all the weight and wonder of {} inside our hearts,", hook_focus),
                    format!("Every measure opens like a window to the sky,"),
                    format!("Finding rhythm in the moments that were passing by."),
                ],
            },
            ParsedLyricSection {
                tag: "[chorus]".into(),
                display_label: "Chorus".into(),
                lines: vec![
                    format!("Sing it clear and steady till the echoes carry far,"),
                    format!("{} written brightly on every distant star,", hook_focus),
                    format!("Warm harmonies rising above the low refrain,"),
                    format!("Turning every memory into sunlight after rain."),
                ],
            },
            ParsedLyricSection {
                tag: "[verse 2]".into(),
                display_label: "Verse 2".into(),
                lines: vec![
                    format!("Acoustic strings and percussion keeping faithful time,"),
                    format!("Every line unfolding in an honest, measured rhyme,"),
                    format!("From quiet whispered embers to a full and lifted sound,"),
                    format!("Grounded in the rhythm where our truest voice is found."),
                ],
            },
            ParsedLyricSection { tag: format!("[{}]", solo_label.to_ascii_lowercase()), display_label: solo_label.into(), lines: vec![] },
            ParsedLyricSection {
                tag: "[bridge]".into(),
                display_label: "Bridge".into(),
                lines: vec![
                    format!("Hold the note suspended in the open breathing space,"),
                    format!("Everything aligns when we stop chasing every race."),
                ],
            },
            ParsedLyricSection {
                tag: "[outro]".into(),
                display_label: "Outro".into(),
                lines: vec![
                    format!("Gentle cadence drifting softly on the evening air,"),
                    format!("Leaving {} shining clear and everywhere.", hook_focus),
                ],
            },
        ]
    };

    StructuredLyricsResult {
        mode_used: "ai_offline_composer".to_string(),
        model_used: "GenreAdaptiveComposer".to_string(),
        title: derive_title_from_topic(hook_focus, style_prompt),
        formatted_lyrics: format_sections(&sections),
        sections,
        preserved_verbatim: false,
    }
}

fn derive_title_from_topic(topic: &str, style_prompt: &str) -> String {
    let words: Vec<String> = topic
        .split_whitespace()
        .take(5)
        .map(|w| {
            let clean = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'');
            let mut chars = clean.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .filter(|s| !s.is_empty())
        .collect();
    if !words.is_empty() {
        return words.join(" ");
    }
    let genre_word = style_prompt.split(',').next().unwrap_or("Original").trim();
    format!("{} Session", genre_word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structure_user_lyrics_verbatim_never_multiplies_or_injects_reggae() {
        let user_input = "Neon rain falls over Kyoto stations\nMidnight trains carry quiet conversations\n\nHold the skyline close tonight\nTill the silver morning light";
        let res = structure_user_lyrics_verbatim(user_input);

        assert!(res.preserved_verbatim);
        assert!(res.formatted_lyrics.contains("Neon rain falls over Kyoto stations"));
        assert!(res.formatted_lyrics.contains("Till the silver morning light"));
        assert!(!res.formatted_lyrics.to_ascii_lowercase().contains("reggae"));
        assert!(!res.formatted_lyrics.to_ascii_lowercase().contains("trenchtown"));
        assert!(!res.formatted_lyrics.to_ascii_lowercase().contains("one love"));

        let matches = res.formatted_lyrics.matches("Neon rain falls over Kyoto stations").count();
        assert_eq!(matches, 1, "User lyric line must not be duplicated 6x");
    }

    #[test]
    fn test_offline_composer_honors_synthpop_and_topic() {
        let res = compose_offline_structured_lyrics(
            "driving through chrome skylines at 2 AM",
            "80s Synthpop, 118 BPM, C Minor, analog Juno pads",
        );
        assert!(!res.preserved_verbatim);
        assert!(res.formatted_lyrics.contains("[Verse 1]"));
        assert!(res.formatted_lyrics.contains("[Chorus]"));
        assert!(res.formatted_lyrics.contains("[Synth Solo]"));
        assert!(!res.formatted_lyrics.to_ascii_lowercase().contains("reggae"));
    }
}
