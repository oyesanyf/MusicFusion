//! Autonomous REA (Reverse Engineering Automation) Subsystem.

pub mod engine;
pub mod path_security;
pub mod prompt_injection;

pub use engine::{
    has_active_lotl_or_exploit_indicators, is_rea_benign_system_or_dev_process,
    AutonomousReaEngine,
};
pub use path_security::{
    detect_binary_masquerade_spoof, get_canonical_protected_binary_names,
    is_canonical_protected_binary_name, is_untrusted_writable_or_temp_path,
    validate_rea_target_path,
};
pub use prompt_injection::{
    calculate_entropy, extract_printable_strings, scan_and_sanitize_counter_ai_strings,
    MAX_EXTRACTED_STRINGS,
};
