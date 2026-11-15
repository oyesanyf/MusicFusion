use music_dsp::{BiquadFilter, ReferenceRlSynthesizer, SAMPLE_RATE};

#[test]
fn test_biquad_low_and_high_shelf_response() {
    let mut unity_lo = BiquadFilter::low_shelf(100.0, 0.0, 2);
    let mut boost_lo = BiquadFilter::low_shelf(100.0, 6.0, 2);
    let mut cut_hi = BiquadFilter::high_shelf(10000.0, -6.0, 2);

    let mut rms_unity = 0.0f32;
    let mut rms_boost = 0.0f32;
    let mut rms_hi_cut = 0.0f32;

    for i in 0..4800 {
        let t = i as f32 / SAMPLE_RATE as f32;
        let low_sig = (2.0 * std::f32::consts::PI * 60.0 * t).sin() * 0.4;
        let high_sig = (2.0 * std::f32::consts::PI * 14000.0 * t).sin() * 0.4;

        let y_u = unity_lo.process_sample(0, low_sig);
        let y_b = boost_lo.process_sample(0, low_sig);
        let y_c = cut_hi.process_sample(0, high_sig);

        rms_unity += y_u * y_u;
        rms_boost += y_b * y_b;
        rms_hi_cut += y_c * y_c;
    }
    assert!(rms_boost > rms_unity * 2.5);
    assert!(rms_hi_cut < rms_unity * 0.6);
}

#[test]
fn test_open_weight_prompt_has_zero_paid_api_references() {
    let prompt = ReferenceRlSynthesizer::build_open_weight_prompt(
        "make folk music with acoustic rhythm",
        Some("[Verse 1]\nNeon rivers under silver stars"),
        80,
        "G Major",
        "[Verse 1]\nNeon rivers under silver stars",
    );
    assert!(!prompt.to_ascii_lowercase().contains("lyria"));
    assert!(prompt.contains("80 BPM"));
    assert!(prompt.contains("G Major"));
}
