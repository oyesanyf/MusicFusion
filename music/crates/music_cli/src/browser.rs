use std::time::Duration;
use log::{info, warn};

/// Launches the default web browser to the specified URL.
/// On Windows, executes `cmd /C start "" "<url>"` with `CREATE_NO_WINDOW (0x08000000)`
/// to eliminate console window flash.
pub fn open_browser_raw(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()?;
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()?;
    }

    Ok(())
}

/// Determines if browser opening should be suppressed based on user flags (`no_browser`, `mcp`)
/// or headless / continuous integration environment variables:
/// `CI`, `GITHUB_ACTIONS`, `TF_BUILD`, and `AGENT_OS`.
pub fn should_suppress_browser(no_browser: bool, mcp: bool) -> bool {
    if no_browser || mcp {
        return true;
    }
    const CI_VARS: &[&str] = &["CI", "GITHUB_ACTIONS", "TF_BUILD", "AGENT_OS"];
    for var in CI_VARS {
        if std::env::var(var).map(|v| !v.trim().is_empty()).unwrap_or(false) {
            return true;
        }
    }
    false
}

/// Spawns a background delayed browser launch task.
/// Delay allows the HTTP listener to complete socket bind before browser connects.
/// Strictly suppressed if `suppressed` is true (e.g. `--no-browser`, `--mcp`, CI).
pub fn spawn_browser_delayed(url: String, delay_ms: u64, suppressed: bool) {
    if suppressed || should_suppress_browser(false, false) {
        info!("Browser launch suppressed by flag or headless/CI environment");
        return;
    }

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        info!("Auto-launching browser to {}", url);
        if let Err(e) = open_browser_raw(&url) {
            warn!("Failed to launch browser: {}", e);
        }
    });
}
