//! # CLI Command Handlers & Formatters

use anyhow::Result;
use crate::client::KillSwitchClient;
use crate::verify::SubsystemVerifier;

pub async fn handle_status(client: &KillSwitchClient) -> Result<()> {
    let status = client.get_status().await?;
    println!("\n================================================================================");
    println!("🛡️  KILLSWITCH SECURITY AGENT STATUS");
    println!("================================================================================");
    println!("  Isolated:               {}", if status.isolated { "YES (HOST CONTAINED)" } else { "NO (Normal Operation)" });
    println!("  Containment Level:      {}", status.containment_level);
    println!("  Active Mode:            {}", status.active_mode);
    println!("  Uptime:                 {} seconds", status.uptime_secs);
    println!("  Version:                {}", status.version);
    println!("  State Hash (BLAKE3):    {}", status.state_hash_blake3);
    println!("  Network Isolated:       {}", status.network_isolated);
    println!("  Processes Frozen:       {}", status.processes_frozen_count);
    println!("  Processes Tarpitted:    {}", status.processes_tarpitted_count);
    println!("  Kernel Lockdown Active: {}", status.kernel_lockdown_active);
    if let Some(iso_at) = status.isolated_at {
        println!("  Isolated At:            {}", iso_at);
    }
    if let Some(iso_by) = status.isolated_by {
        println!("  Isolated By:            {}", iso_by);
    }
    if let Some(reason) = status.reason {
        println!("  Reason:                 {}", reason);
    }
    if !status.allowlisted_ports.is_empty() {
        println!("  Allowlisted Ports:      {:?}", status.allowlisted_ports);
    }
    if !status.frozen_pids.is_empty() {
        println!("  Frozen PIDs:            {:?}", status.frozen_pids);
    }
    println!("================================================================================\n");
    Ok(())
}

pub async fn handle_isolate(
    client: &KillSwitchClient,
    operator: &str,
    reason: &str,
    level: Option<&str>,
    token: Option<&str>,
) -> Result<()> {
    println!("🚨 Initiating Host Containment Isolation...");

    #[cfg(windows)]
    {
        use std::io::IsTerminal;
        let has_wfp_access = killswitch_core::wfp::WfpDynamicSession::is_available();
        let is_interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
        let is_child = std::env::args().any(|a| a == "--elevated-wfp-helper");

        if !has_wfp_access && is_interactive && !is_child {
            println!("⚡ [WFP-KERNEL] Requesting Windows Administrator elevation via native ShellExecuteExW to install kernel WFP filters...");
            let mut args = vec![
                "isolate".to_string(),
                "-o".to_string(),
                operator.to_string(),
                "-r".to_string(),
                reason.to_string(),
            ];
            if let Some(l) = level {
                args.push("-l".to_string());
                args.push(l.to_string());
            }
            if let Some(t) = token {
                args.push("-t".to_string());
                args.push(t.to_string());
            }
            if try_elevate_via_shellexecute(&args) {
                let status = client.get_status().await?;
                println!("✅ Containment Engaged (Elevated Kernel WFP + BFE Firewall Active)");
                println!("   Level:          {}", status.containment_level);
                println!("   State Hash:     {}", status.state_hash_blake3);
                return Ok(());
            } else {
                println!("⚠️  [WFP-KERNEL] Administrator elevation was declined or unavailable.");
                println!("   Tip: Run cmd.exe / PowerShell as Administrator so Windows BFE (fwpuclnt.dll) blocks ICMP ping & TCP packets at the kernel layer.");
            }
        }
    }

    let resp = client.engage(operator, reason, level, token).await?;
    println!("✅ Containment Engaged: {}", resp.message);
    println!("   Level: {:?}", resp.status.containment_level);
    println!("   State Hash: {}", resp.status.state_hash_blake3);
    #[cfg(windows)]
    if !killswitch_core::wfp::WfpDynamicSession::is_available() {
        println!("   ⚠️  Note: Running without Administrator privileges; open an Administrator terminal to enforce kernel-level WFP packet blocking (e.g. ping 8.8.8.8).");
    }
    Ok(())
}

pub async fn handle_restore(
    client: &KillSwitchClient,
    operator: &str,
    reason: &str,
    token: Option<&str>,
) -> Result<()> {
    println!("🛡️  Initiating Host Disarm & Endpoint Restoration...");

    #[cfg(windows)]
    {
        use std::io::IsTerminal;
        let has_wfp_access = killswitch_core::wfp::WfpDynamicSession::is_available();
        let is_interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
        let is_child = std::env::args().any(|a| a == "--elevated-wfp-helper");

        if !has_wfp_access && is_interactive && !is_child {
            println!("⚡ [WFP-KERNEL] Requesting Windows Administrator elevation via native ShellExecuteExW to purge kernel WFP filters...");
            let mut args = vec![
                "restore".to_string(),
                "-o".to_string(),
                operator.to_string(),
                "-r".to_string(),
                reason.to_string(),
            ];
            if let Some(t) = token {
                args.push("-t".to_string());
                args.push(t.to_string());
            }
            if try_elevate_via_shellexecute(&args) {
                let _ = client.disarm(operator, reason, token).await;
                let status = client.get_status().await?;
                println!("✅ Disarm Successful (Elevated Kernel WFP + BFE Firewall Restored)");
                println!("   Current Level:  {}", status.containment_level);
                println!("   State Hash:     {}", status.state_hash_blake3);
                return Ok(());
            }
        }
    }

    let resp = client.disarm(operator, reason, token).await?;
    println!("✅ Disarm Successful: {}", resp.message);
    println!("   Current Level: {:?}", resp.status.containment_level);
    println!("   State Hash: {}", resp.status.state_hash_blake3);
    Ok(())
}

#[cfg(windows)]
fn try_elevate_via_shellexecute(args: &[String]) -> bool {
    #[repr(C)]
    struct ShellExecuteInfoW {
        cb_size: u32,
        f_mask: u32,
        hwnd: usize,
        lp_verb: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show: i32,
        h_inst_app: usize,
        lp_id_list: usize,
        lp_class: *const u16,
        hkey_class: usize,
        dw_hot_key: u32,
        h_icon_or_monitor: usize,
        h_process: usize,
    }

    extern "system" {
        fn LoadLibraryW(lp_lib_file_name: *const u16) -> usize;
        fn GetProcAddress(h_module: usize, lp_proc_name: *const u8) -> usize;
        fn WaitForSingleObject(h_handle: usize, dw_milliseconds: u32) -> u32;
        fn GetExitCodeProcess(h_process: usize, lp_exit_code: *mut u32) -> i32;
        fn CloseHandle(h_object: usize) -> i32;
    }

    type FnShellExecuteExW = unsafe extern "system" fn(*mut ShellExecuteInfoW) -> i32;

    let Ok(exe_path) = std::env::current_exe() else {
        return false;
    };
    let Ok(cwd) = std::env::current_dir() else {
        return false;
    };

    let quoted_args: Vec<String> = args
        .iter()
        .map(|a| {
            if a.contains(' ') {
                format!("\"{}\"", a.replace('"', "\\\""))
            } else {
                a.clone()
            }
        })
        .collect();
    let param_str = quoted_args.join(" ");

    let verb_w: Vec<u16> = "runas\0".encode_utf16().collect();
    let file_w: Vec<u16> = format!("{}\0", exe_path.display()).encode_utf16().collect();
    let params_w: Vec<u16> = format!("{}\0", param_str).encode_utf16().collect();
    let dir_w: Vec<u16> = format!("{}\0", cwd.display()).encode_utf16().collect();
    let dll_w: Vec<u16> = "shell32.dll\0".encode_utf16().collect();

    unsafe {
        let h_shell32 = LoadLibraryW(dll_w.as_ptr());
        if h_shell32 == 0 {
            return false;
        }
        let p_func = GetProcAddress(h_shell32, b"ShellExecuteExW\0".as_ptr());
        if p_func == 0 {
            return false;
        }
        let shell_execute_ex: FnShellExecuteExW = std::mem::transmute(p_func);

        const SEE_MASK_NOCLOSEPROCESS: u32 = 0x00000040;
        const SW_HIDE: i32 = 0;

        let mut sei = ShellExecuteInfoW {
            cb_size: std::mem::size_of::<ShellExecuteInfoW>() as u32,
            f_mask: SEE_MASK_NOCLOSEPROCESS,
            hwnd: 0,
            lp_verb: verb_w.as_ptr(),
            lp_file: file_w.as_ptr(),
            lp_parameters: params_w.as_ptr(),
            lp_directory: dir_w.as_ptr(),
            n_show: SW_HIDE,
            h_inst_app: 0,
            lp_id_list: 0,
            lp_class: std::ptr::null(),
            hkey_class: 0,
            dw_hot_key: 0,
            h_icon_or_monitor: 0,
            h_process: 0,
        };

        let ok = shell_execute_ex(&mut sei);
        if ok == 0 || sei.h_process == 0 {
            return false;
        }

        WaitForSingleObject(sei.h_process, 30_000);
        let mut exit_code: u32 = 1;
        GetExitCodeProcess(sei.h_process, &mut exit_code);
        CloseHandle(sei.h_process);
        exit_code == 0
    }
}

pub async fn handle_service(
    client: &KillSwitchClient,
    action: Option<&str>,
) -> Result<()> {
    match action {
        None | Some("status") => {
            let status = client.get_service_status().await?;
            println!("\n================================================================================");
            println!("🔧 NATIVE SERVICE STATUS: {}", status.name);
            println!("================================================================================");
            println!("  Display Name: {}", status.display_name);
            println!("  State:        {}", status.state);
            println!("  Installed:    {}", status.is_installed);
            println!("  PID:          {:?}", status.pid);
            println!("  Binary Path:  {}", status.binary_path);
            println!("================================================================================\n");
        }
        Some(act) => {
            println!("🔧 Commanding service action: '{}'...", act);
            let resp = client.control_service(act).await?;
            println!("✅ Service Action Result: {}", resp.action);
            println!("   Service Name: {}", resp.service_name);
            println!("   New State:    {}", resp.resulting_status.state);
        }
    }
    Ok(())
}

pub async fn handle_lineage(
    client: &KillSwitchClient,
    action: &str,
    path_or_url: Option<&str>,
    sha256: Option<&str>,
    classification: Option<&str>,
    policy: Option<&str>,
    pid: Option<u32>,
) -> Result<()> {
    match action {
        "status" => {
            let status = client.get_lineage_status().await?;
            println!("\n================================================================================");
            println!("🧬 DLP OPENLINEAGE STATUS");
            println!("================================================================================");
            println!("  Total Processes Tracked:   {}", status.total_processes_tracked);
            println!("  Tainted Processes Count:   {}", status.tainted_processes_count);
            println!("  Total Sensitive Files:     {}", status.total_files_tracked);
            println!("  Tainted Files Count:       {}", status.tainted_files_count);
            println!("================================================================================\n");
        }
        "seed" => {
            let p = path_or_url.unwrap_or("C:\\confidential\\doc.docx");
            let s = sha256.unwrap_or("0000000000000000000000000000000000000000000000000000000000000000");
            let c = classification.unwrap_or("CrownJewel");
            let pol = policy.unwrap_or("Quarantine");

            let res = client.seed_lineage(p, s, c, pol).await?;
            println!("✅ Seeded Sensitive File: {}", res.path);
            println!("   Classification: {}", res.classification);
            println!("   Policy:         {}", res.policy);
        }
        "eval" => {
            let p = pid.unwrap_or(std::process::id());
            let dest = path_or_url.unwrap_or("https://external-exfil.com");

            let res = client.evaluate_lineage(p, dest).await?;
            println!("🔍 Egress Verdict for PID {}: {}", p, res.verdict);
            println!("   Is Blocked:      {}", res.is_block);
            if let Some(r) = res.reason {
                println!("   Reason:          {}", r);
            }
            if let Some(c) = res.classification {
                println!("   Classification:  {}", c);
            }
        }
        _ => anyhow::bail!("Unknown lineage subcommand: {}", action),
    }
    Ok(())
}

pub async fn handle_graph(
    client: &KillSwitchClient,
    action: &str,
    force_node: Option<&str>,
) -> Result<()> {
    match action {
        "state" => {
            let state = client.get_graph_state().await?;
            println!("\n================================================================================");
            println!("🕸️  LANGGRAPH ORCHESTRATOR STATE");
            println!("================================================================================");
            println!("  Incident ID:           {}", state.incident_id);
            println!("  Current Node:          {}", state.current_node);
            println!("  Mode:                  {}", state.mode);
            println!("  Aggregate Threat Score: {:.2}", state.aggregate_threat_score);
            println!("  Active Threats:        {}", state.active_threats_count);
            println!("  Execution Trace Len:   {}", state.execution_trace_len);
            println!("  Trace Chain Valid:     {}", state.trace_chain_valid);
            println!("================================================================================\n");
        }
        "step" => {
            let res = client.step_graph(force_node).await?;
            println!("✅ Graph Step Completed:");
            println!("   Executed Node: {}", res.executed_node);
            println!("   Next Node:     {}", res.next_node);
            println!("   Summary:       {}", res.summary);
            println!("   Threat Score:  {:.2}", res.threat_score);
            println!("   Is Isolated:   {}", res.is_isolated);
        }
        _ => anyhow::bail!("Unknown graph subcommand: {}", action),
    }
    Ok(())
}

pub async fn handle_verify() -> Result<()> {
    let report = SubsystemVerifier::run_all().await?;
    report.print_summary();
    Ok(())
}
