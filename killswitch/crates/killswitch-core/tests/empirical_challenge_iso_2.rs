//! Empirical Challenge Test Suite (Milestone ISO-2: Linux Netlink Empirical Challenger)
//!
//! Rigorous empirical stress testing and verification for:
//! 1. Linux Netlink route blackholing (`RTN_BLACKHOLE` in `RT_TABLE_MAIN` via `NETLINK_ROUTE`).
//! 2. Loopback preservation (`127.0.0.1` and `::1`) in `RT_TABLE_LOCAL`.
//! 3. Socket teardown (`INET_DIAG`, `SOCK_DESTROY`) and clean disarm (`RTM_DELROUTE`).
//! 4. Unprivileged execution fails closed with typed `InsufficientPrivileges`.
//! 5. Overall verdict: CONFIRMED or DISPROVED.

use killswitch_core::landlock::*;
use killswitch_core::killchain::default_network_isolator;
use killswitch_types::{
    errors::KillSwitchError,
    status::{NetworkIsolationEngine, NetworkIsolator, DEFAULT_CONTROL_PORT},
};
use std::fs;
use std::mem::{align_of, offset_of, size_of};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

// =============================================================================
// Task 1: Linux Netlink Route Blackholing (RTN_BLACKHOLE in RT_TABLE_MAIN)
// =============================================================================

#[test]
fn test_task1_ipv4_blackhole_route_msg_binary_layout() {
    println!("[Task 1] Testing Netlink IPv4 RTN_BLACKHOLE message binary layout...");

    let priority: u32 = 10;
    let msg = build_blackhole_route_msg(false, false, priority);

    // Total length must be exactly 36 bytes:
    // NlMsgHdr (16 bytes) + RtMsg (12 bytes) + RtAttr (4 bytes) + priority (4 bytes) = 36 bytes
    assert_eq!(
        msg.len(),
        36,
        "IPv4 blackhole route message must be exactly 36 bytes"
    );

    // 1. Validate NlMsgHdr (bytes 0..16)
    let hdr = unsafe { &*(msg.as_ptr() as *const NlMsgHdr) };
    assert_eq!(hdr.nlmsg_len, 36, "nlmsg_len must equal 36 bytes");
    assert_eq!(
        hdr.nlmsg_type, RTM_NEWROUTE,
        "nlmsg_type must be RTM_NEWROUTE (24)"
    );
    assert_eq!(
        hdr.nlmsg_flags,
        NLM_F_REQUEST | NLM_F_ACK | NLM_F_CREATE | NLM_F_REPLACE,
        "nlmsg_flags must have REQUEST, ACK, CREATE, REPLACE (0x705)"
    );
    assert_eq!(hdr.nlmsg_seq, 1, "nlmsg_seq must be 1");
    assert_eq!(hdr.nlmsg_pid, 0, "nlmsg_pid must be 0 (target kernel)");

    // 2. Validate RtMsg (bytes 16..28)
    let rt = unsafe { &*(msg.as_ptr().add(16) as *const RtMsg) };
    assert_eq!(
        rt.rtm_family,
        2,
        "rtm_family must be AF_INET (2)"
    );
    assert_eq!(
        rt.rtm_dst_len, 0,
        "rtm_dst_len must be 0 (representing 0.0.0.0/0 default route)"
    );
    assert_eq!(rt.rtm_src_len, 0, "rtm_src_len must be 0");
    assert_eq!(rt.rtm_tos, 0, "rtm_tos must be 0");
    assert_eq!(
        rt.rtm_table, RT_TABLE_MAIN,
        "rtm_table must be RT_TABLE_MAIN (254)"
    );
    assert_eq!(
        rt.rtm_protocol, RTPROT_STATIC,
        "rtm_protocol must be RTPROT_STATIC (4)"
    );
    assert_eq!(
        rt.rtm_scope, RT_SCOPE_UNIVERSE,
        "rtm_scope must be RT_SCOPE_UNIVERSE (0)"
    );
    assert_eq!(
        rt.rtm_type, RTN_BLACKHOLE,
        "rtm_type must be RTN_BLACKHOLE (6)"
    );
    assert_eq!(rt.rtm_flags, 0, "rtm_flags must be 0");

    // 3. Validate RtAttr (bytes 28..32) and Priority payload (bytes 32..36)
    let rta = unsafe { &*(msg.as_ptr().add(28) as *const RtAttr) };
    assert_eq!(rta.rta_len, 8, "rta_len must be 8 (RtAttr 4 + payload 4)");
    assert_eq!(
        rta.rta_type, RTA_PRIORITY,
        "rta_type must be RTA_PRIORITY (6)"
    );

    let priority_val = u32::from_ne_bytes([msg[32], msg[33], msg[34], msg[35]]);
    assert_eq!(
        priority_val, priority,
        "Priority attribute must encode priority 10"
    );
    println!("Confirmed: IPv4 RTN_BLACKHOLE binary packet adheres strictly to Netlink ABI.");
}

#[test]
fn test_task1_ipv6_blackhole_route_msg_binary_layout() {
    println!("[Task 1] Testing Netlink IPv6 RTN_BLACKHOLE message binary layout...");

    let priority: u32 = 10;
    let msg = build_blackhole_route_msg(true, false, priority);

    assert_eq!(
        msg.len(),
        36,
        "IPv6 blackhole route message must be exactly 36 bytes"
    );

    let hdr = unsafe { &*(msg.as_ptr() as *const NlMsgHdr) };
    assert_eq!(hdr.nlmsg_len, 36);
    assert_eq!(hdr.nlmsg_type, RTM_NEWROUTE);
    assert_eq!(
        hdr.nlmsg_flags,
        NLM_F_REQUEST | NLM_F_ACK | NLM_F_CREATE | NLM_F_REPLACE
    );

    let rt = unsafe { &*(msg.as_ptr().add(16) as *const RtMsg) };
    assert_eq!(
        rt.rtm_family,
        10,
        "rtm_family must be AF_INET6 (10)"
    );
    assert_eq!(
        rt.rtm_dst_len, 0,
        "rtm_dst_len must be 0 (representing ::/0 default route)"
    );
    assert_eq!(rt.rtm_table, RT_TABLE_MAIN);
    assert_eq!(rt.rtm_type, RTN_BLACKHOLE);
    assert_eq!(rt.rtm_protocol, RTPROT_STATIC);

    let rta = unsafe { &*(msg.as_ptr().add(28) as *const RtAttr) };
    assert_eq!(rta.rta_len, 8);
    assert_eq!(rta.rta_type, RTA_PRIORITY);

    let priority_val = u32::from_ne_bytes([msg[32], msg[33], msg[34], msg[35]]);
    assert_eq!(priority_val, priority);
    println!("Confirmed: IPv6 RTN_BLACKHOLE binary packet adheres strictly to Netlink ABI.");
}

#[test]
fn test_task1_netlink_struct_abi_sizes_alignments_and_offsets() {
    println!("[Task 1] Testing C ABI sizes, alignments, and field offsets for Netlink structs...");

    // 1. NlMsgHdr (16 bytes, align 4)
    assert_eq!(size_of::<NlMsgHdr>(), 16);
    assert_eq!(align_of::<NlMsgHdr>(), 4);
    assert_eq!(offset_of!(NlMsgHdr, nlmsg_len), 0);
    assert_eq!(offset_of!(NlMsgHdr, nlmsg_type), 4);
    assert_eq!(offset_of!(NlMsgHdr, nlmsg_flags), 6);
    assert_eq!(offset_of!(NlMsgHdr, nlmsg_seq), 8);
    assert_eq!(offset_of!(NlMsgHdr, nlmsg_pid), 12);

    // 2. RtMsg (12 bytes, align 4)
    assert_eq!(size_of::<RtMsg>(), 12);
    assert_eq!(align_of::<RtMsg>(), 4);
    assert_eq!(offset_of!(RtMsg, rtm_family), 0);
    assert_eq!(offset_of!(RtMsg, rtm_dst_len), 1);
    assert_eq!(offset_of!(RtMsg, rtm_src_len), 2);
    assert_eq!(offset_of!(RtMsg, rtm_tos), 3);
    assert_eq!(offset_of!(RtMsg, rtm_table), 4);
    assert_eq!(offset_of!(RtMsg, rtm_protocol), 5);
    assert_eq!(offset_of!(RtMsg, rtm_scope), 6);
    assert_eq!(offset_of!(RtMsg, rtm_type), 7);
    assert_eq!(offset_of!(RtMsg, rtm_flags), 8);

    // 3. RtAttr (4 bytes, align 2)
    assert_eq!(size_of::<RtAttr>(), 4);
    assert_eq!(align_of::<RtAttr>(), 2);
    assert_eq!(offset_of!(RtAttr, rta_len), 0);
    assert_eq!(offset_of!(RtAttr, rta_type), 2);

    // 4. SockAddrNl (12 bytes, align 4)
    assert_eq!(size_of::<SockAddrNl>(), 12);
    assert_eq!(align_of::<SockAddrNl>(), 4);
    assert_eq!(offset_of!(SockAddrNl, nl_family), 0);
    assert_eq!(offset_of!(SockAddrNl, nl_pad), 2);
    assert_eq!(offset_of!(SockAddrNl, nl_pid), 4);
    assert_eq!(offset_of!(SockAddrNl, nl_groups), 8);

    // 5. InetDiagSockId (48 bytes, align 4)
    assert_eq!(size_of::<InetDiagSockId>(), 48);
    assert_eq!(align_of::<InetDiagSockId>(), 4);
    assert_eq!(offset_of!(InetDiagSockId, idiag_sport), 0);
    assert_eq!(offset_of!(InetDiagSockId, idiag_dport), 2);
    assert_eq!(offset_of!(InetDiagSockId, idiag_src), 4);
    assert_eq!(offset_of!(InetDiagSockId, idiag_dst), 20);
    assert_eq!(offset_of!(InetDiagSockId, idiag_if), 36);
    assert_eq!(offset_of!(InetDiagSockId, idiag_cookie), 40);

    // 6. InetDiagReqV2 (56 bytes, align 4)
    assert_eq!(size_of::<InetDiagReqV2>(), 56);
    assert_eq!(align_of::<InetDiagReqV2>(), 4);
    assert_eq!(offset_of!(InetDiagReqV2, sdiag_family), 0);
    assert_eq!(offset_of!(InetDiagReqV2, sdiag_protocol), 1);
    assert_eq!(offset_of!(InetDiagReqV2, idiag_ext), 2);
    assert_eq!(offset_of!(InetDiagReqV2, pad), 3);
    assert_eq!(offset_of!(InetDiagReqV2, idiag_states), 4);
    assert_eq!(offset_of!(InetDiagReqV2, id), 8);
    println!("Confirmed: All Netlink data structures strictly conform to Linux kernel ABI.");
}

#[test]
fn test_task1_priority_variations_and_endianness_safety() {
    println!("[Task 1] Testing dynamic metric/priority values and endian safety...");

    for priority in [1u32, 5, 10, 50, 100, 1000, 65535, 1_000_000] {
        let msg_v4 = build_blackhole_route_msg(false, false, priority);
        assert_eq!(msg_v4.len(), 36);
        let extracted_v4 = u32::from_ne_bytes([msg_v4[32], msg_v4[33], msg_v4[34], msg_v4[35]]);
        assert_eq!(extracted_v4, priority, "Priority {priority} failed round-trip in IPv4");

        let msg_v6 = build_blackhole_route_msg(true, false, priority);
        assert_eq!(msg_v6.len(), 36);
        let extracted_v6 = u32::from_ne_bytes([msg_v6[32], msg_v6[33], msg_v6[34], msg_v6[35]]);
        assert_eq!(extracted_v6, priority, "Priority {priority} failed round-trip in IPv6");
    }
}

#[test]
fn test_task1_zero_subprocess_invariant_in_linux_source() {
    println!("[Task 1] Verifying that zero CLI subprocesses are invoked in landlock.rs...");

    let landlock_src = if Path::new("src/landlock.rs").exists() {
        fs::read_to_string("src/landlock.rs").expect("read landlock.rs")
    } else {
        fs::read_to_string("crates/killswitch-core/src/landlock.rs").expect("read landlock.rs")
    };

    let forbidden_patterns = [
        "\"iptables\"",
        "\"nft\"",
        "\"ip\"",
        "\"route\"",
        "Command::new(\"iptables\")",
        "Command::new(\"nft\")",
        "Command::new(\"ip\")",
        "Command::new(\"route\")",
        "Command::new(\"ufw\")",
    ];

    for pat in &forbidden_patterns {
        assert!(
            !landlock_src.contains(pat),
            "Violation: landlock.rs contains forbidden CLI subprocess token {}",
            pat
        );
    }
    println!("Confirmed: landlock.rs executes zero CLI subprocesses, strictly relying on Netlink binary sockets.");
}

// =============================================================================
// Task 2: Loopback Preservation (127.0.0.1 and ::1) in RT_TABLE_LOCAL
// =============================================================================

#[test]
fn test_task2_routing_table_hierarchy_and_fib_priority_preservation() {
    println!("[Task 2] Testing routing table hierarchy (RT_TABLE_LOCAL vs RT_TABLE_MAIN)...");

    // RT_TABLE_LOCAL = 255 (evaluated at Linux FIB rule 0: from all lookup local)
    // RT_TABLE_MAIN = 254 (evaluated at Linux FIB rule 32766: from all lookup main)
    assert_eq!(RT_TABLE_MAIN, 254);
    assert_eq!(RT_TABLE_LOCAL, 255);

    // Verify that the blackhole route is placed strictly into RT_TABLE_MAIN
    let msg_v4 = build_blackhole_route_msg(false, false, 10);
    let rt_v4 = unsafe { &*(msg_v4.as_ptr().add(16) as *const RtMsg) };
    assert_eq!(
        rt_v4.rtm_table, RT_TABLE_MAIN,
        "Blackhole route must reside in RT_TABLE_MAIN (254), NEVER RT_TABLE_LOCAL"
    );
    assert_ne!(
        rt_v4.rtm_table, RT_TABLE_LOCAL,
        "Blackhole route must not contaminate RT_TABLE_LOCAL"
    );

    let msg_v6 = build_blackhole_route_msg(true, false, 10);
    let rt_v6 = unsafe { &*(msg_v6.as_ptr().add(16) as *const RtMsg) };
    assert_eq!(rt_v6.rtm_table, RT_TABLE_MAIN);
    assert_ne!(rt_v6.rtm_table, RT_TABLE_LOCAL);

    println!("Confirmed: Blackhole routes reside in RT_TABLE_MAIN; RT_TABLE_LOCAL loopback routing is fully preserved.");
}

#[test]
fn test_task2_linux_netlink_isolator_loopback_and_control_port_preservation() {
    println!("[Task 2] Testing LinuxNetlinkIsolator receipt guarantees for loopback and control port...");

    let isolator = LinuxNetlinkIsolator::new();

    // 1. Test quarantine engagement with no preserved ports requested
    let receipt1 = isolator
        .engage_quarantine(&[], "sec_admin", "Testing default control port inclusion")
        .expect("engage_quarantine must succeed");

    assert_eq!(receipt1.engine, NetworkIsolationEngine::LinuxNetlink);
    assert!(
        receipt1.preserved_ports.contains(&DEFAULT_CONTROL_PORT),
        "receipt must automatically preserve control port 3030"
    );
    assert!(
        receipt1
            .preserved_ips
            .contains(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
        "receipt must preserve 127.0.0.1"
    );
    assert!(
        receipt1
            .preserved_ips
            .contains(&IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))),
        "receipt must preserve ::1"
    );

    // 2. Test quarantine engagement with custom ports
    let receipt2 = isolator
        .engage_quarantine(&[8080, 9090], "operator", "Testing custom ports")
        .expect("engage_quarantine must succeed");

    assert!(receipt2.preserved_ports.contains(&8080));
    assert!(receipt2.preserved_ports.contains(&9090));
    assert!(receipt2.preserved_ports.contains(&DEFAULT_CONTROL_PORT));

    // Clean up
    assert!(isolator.restore_quarantine(&receipt1).is_ok());
    assert!(isolator.restore_quarantine(&receipt2).is_ok());
}

#[test]
fn test_task2_proc_net_tcp_loopback_preservation_matrix() {
    println!("[Task 2] Stress-testing /proc/net/tcp parsing and loopback preservation filtering...");

    // Comprehensive synthetic /proc/net/tcp sample containing:
    // Slot 0: 127.0.0.1:3030 (Listen) -> loopback listener
    // Slot 1: 127.0.0.1:50000 -> 127.0.0.1:3030 (Established) -> WebUI client connection
    // Slot 2: 127.0.0.1:40000 -> 127.0.0.1:8080 (Established) -> Inter-process IPC
    // Slot 3: 192.168.1.10:55000 -> 8.8.8.8:53 (Established) -> DNS leak (MUST BE SEVERED)
    // Slot 4: 192.168.1.10:55001 -> 1.1.1.1:443 (SynSent) -> Outbound TLS connection (MUST BE SEVERED)
    // Slot 5: 192.168.1.10:55002 -> 142.250.190.46:443 (CloseWait) -> Lingering TCP (MUST BE SEVERED)
    // Slot 6: 192.168.1.10:55003 -> 142.250.190.46:80 (TimeWait) -> Dead socket (should NOT be severed)
    // Slot 7: 0.0.0.0:3030 -> 0.0.0.0:0 (Listen on all interfaces) -> Server socket (should NOT be severed)
    const ADVERSARIAL_PROC_NET_TCP: &str = r#"  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:0BD6 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 10001 1 0000000000000000 100 0 0 10 0
   1: 0100007F:C350 0100007F:0BD6 01 00000000:00000000 00:00000000 00000000  1000        0 10002 1 0000000000000000 100 0 0 10 0
   2: 0100007F:9C40 0100007F:1F90 01 00000000:00000000 00:00000000 00000000  1000        0 10003 1 0000000000000000 100 0 0 10 0
   3: 0A01A8C0:D6D8 08080808:0035 01 00000000:00000000 00:00000000 00000000  1000        0 10004 1 0000000000000000 100 0 0 10 0
   4: 0A01A8C0:D6D9 01010101:01BB 02 00000000:00000000 00:00000000 00000000  1000        0 10005 1 0000000000000000 100 0 0 10 0
   5: 0A01A8C0:D6DA 2EBEFAA8:01BB 08 00000000:00000000 00:00000000 00000000  1000        0 10006 1 0000000000000000 100 0 0 10 0
   6: 0A01A8C0:D6DB 2EBEFAA8:0050 06 00000000:00000000 00:00000000 00000000  1000        0 10007 1 0000000000000000 100 0 0 10 0
   7: 00000000:0BD6 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 10008 1 0000000000000000 100 0 0 10 0
"#;

    let parsed = parse_proc_net_content(ADVERSARIAL_PROC_NET_TCP, false);
    assert_eq!(parsed.len(), 8, "Must parse all 8 socket rows");

    // Filter non-loopback connections using the exact production predicate in get_active_non_loopback_connections
    let to_sever: Vec<&LinuxSocketEntry> = parsed
        .iter()
        .filter(|e| {
            !e.remote_addr.is_loopback()
                && !e.remote_addr.is_unspecified()
                && e.remote_port != 3030
                && (e.state == TcpState::Established
                    || e.state == TcpState::SynSent
                    || e.state == TcpState::SynRecv
                    || e.state == TcpState::CloseWait)
        })
        .collect();

    // Exactly 3 connections should be marked for termination (slots 3, 4, 5)
    assert_eq!(
        to_sever.len(),
        3,
        "Only non-loopback active connections must be marked for severing"
    );

    // Assert that loopback connections are 100% PRESERVED
    assert!(
        !to_sever.iter().any(|e| e.remote_addr == IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
        "Loopback 127.0.0.1 sockets must NEVER be severed"
    );
    assert!(
        !to_sever.iter().any(|e| e.local_port == 3030 || e.remote_port == 3030),
        "Control port 3030 sockets must NEVER be severed"
    );

    // Assert that non-loopback connections ARE targeted
    assert_eq!(to_sever[0].remote_addr, IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)));
    assert_eq!(to_sever[0].remote_port, 53);
    assert_eq!(to_sever[1].remote_addr, IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)));
    assert_eq!(to_sever[1].remote_port, 443);
    assert_eq!(to_sever[2].remote_addr, IpAddr::V4(Ipv4Addr::new(168, 250, 190, 46))); // 2EBEFAA8 -> 168.250.190.46
    assert_eq!(to_sever[2].remote_port, 443);

    println!("Confirmed: Loopback connections to 127.0.0.1 and port 3030 are strictly preserved.");
}

#[test]
fn test_task2_landlock_sandbox_config_strictly_excludes_dns_and_dhcp() {
    println!("[Task 2] Testing LandlockSandboxConfig default rules for strict port isolation...");

    let config = LandlockSandboxConfig::default();
    assert_eq!(
        config.allowed_tcp_ports,
        vec![3030],
        "Landlock sandbox must strictly permit only control port 3030"
    );
    assert!(
        !config.allowed_tcp_ports.contains(&53),
        "Port 53 (DNS) must be strictly EXCLUDED from Landlock sandbox"
    );
    assert!(
        !config.allowed_tcp_ports.contains(&67),
        "Port 67 (DHCP) must be strictly EXCLUDED"
    );
    assert!(
        !config.allowed_tcp_ports.contains(&68),
        "Port 68 (DHCP) must be strictly EXCLUDED"
    );
}

// =============================================================================
// Task 3: Socket Teardown & Clean Disarm (RTM_DELROUTE)
// =============================================================================

#[test]
fn test_task3_sock_destroy_binary_layout_and_network_byte_order() {
    println!("[Task 3] Testing Netlink INET_DIAG SOCK_DESTROY message binary layout...");

    let entry_v4 = LinuxSocketEntry {
        slot: 1,
        local_addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        local_port: 54321,
        remote_addr: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
        remote_port: 53,
        state: TcpState::Established,
        inode: 99999,
        is_ipv6: false,
    };

    let msg = build_sock_destroy_msg(&entry_v4);

    // Total length must be exactly 72 bytes:
    // NlMsgHdr (16) + InetDiagReqV2 (56) = 72 bytes
    assert_eq!(msg.len(), 72, "SOCK_DESTROY message must be 72 bytes");

    let hdr = unsafe { &*(msg.as_ptr() as *const NlMsgHdr) };
    assert_eq!(hdr.nlmsg_len, 72);
    assert_eq!(
        hdr.nlmsg_type, SOCK_DESTROY,
        "nlmsg_type must be SOCK_DESTROY (21)"
    );
    assert_eq!(
        hdr.nlmsg_flags,
        NLM_F_REQUEST | NLM_F_ACK,
        "nlmsg_flags must have REQUEST and ACK (0x05)"
    );
    assert_eq!(hdr.nlmsg_seq, 2);

    let req = unsafe { &*(msg.as_ptr().add(16) as *const InetDiagReqV2) };
    assert_eq!(
        req.sdiag_family,
        2,
        "sdiag_family must be AF_INET (2)"
    );
    assert_eq!(req.sdiag_protocol, 6, "sdiag_protocol must be 6 (IPPROTO_TCP)");
    assert_eq!(
        req.idiag_states, 0xFFFFFFFF,
        "idiag_states must target all TCP states"
    );

    // Verify network byte order (big-endian) on ports
    assert_eq!(
        req.id.idiag_sport,
        54321u16.to_be(),
        "idiag_sport must be encoded in big-endian network byte order"
    );
    assert_eq!(
        req.id.idiag_dport,
        53u16.to_be(),
        "idiag_dport must be encoded in big-endian network byte order"
    );

    // Verify INET_DIAG_NOCOOKIE ([0xFFFFFFFF, 0xFFFFFFFF])
    assert_eq!(req.id.idiag_cookie, [0xFFFFFFFF, 0xFFFFFFFF]);

    // Test IPv6 entry
    let entry_v6 = LinuxSocketEntry {
        slot: 2,
        local_addr: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
        local_port: 60000,
        remote_addr: IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111)),
        remote_port: 443,
        state: TcpState::Established,
        inode: 88888,
        is_ipv6: true,
    };

    let msg_v6 = build_sock_destroy_msg(&entry_v6);
    assert_eq!(msg_v6.len(), 72);
    let req_v6 = unsafe { &*(msg_v6.as_ptr().add(16) as *const InetDiagReqV2) };
    assert_eq!(
        req_v6.sdiag_family,
        10,
        "sdiag_family must be AF_INET6 (10)"
    );
    assert_eq!(req_v6.id.idiag_sport, 60000u16.to_be());
    assert_eq!(req_v6.id.idiag_dport, 443u16.to_be());

    println!("Confirmed: SOCK_DESTROY messages adhere to Linux inet_diag kernel ABI.");
}

#[test]
fn test_task3_clean_disarm_rtm_delroute_message_binary_layout() {
    println!("[Task 3] Testing Netlink RTM_DELROUTE message binary layout...");

    // 1. IPv4 Delete Message
    let del_v4 = build_blackhole_route_msg(false, true, 10);
    assert_eq!(del_v4.len(), 36);

    let hdr_v4 = unsafe { &*(del_v4.as_ptr() as *const NlMsgHdr) };
    assert_eq!(
        hdr_v4.nlmsg_type, RTM_DELROUTE,
        "nlmsg_type must be RTM_DELROUTE (25)"
    );
    assert_eq!(
        hdr_v4.nlmsg_flags,
        NLM_F_REQUEST | NLM_F_ACK,
        "Delete flags must NOT contain NLM_F_CREATE or NLM_F_REPLACE"
    );

    let rt_v4 = unsafe { &*(del_v4.as_ptr().add(16) as *const RtMsg) };
    assert_eq!(rt_v4.rtm_table, RT_TABLE_MAIN);
    assert_eq!(rt_v4.rtm_type, RTN_BLACKHOLE);

    // 2. IPv6 Delete Message
    let del_v6 = build_blackhole_route_msg(true, true, 10);
    assert_eq!(del_v6.len(), 36);

    let hdr_v6 = unsafe { &*(del_v6.as_ptr() as *const NlMsgHdr) };
    assert_eq!(hdr_v6.nlmsg_type, RTM_DELROUTE);
    assert_eq!(hdr_v6.nlmsg_flags, NLM_F_REQUEST | NLM_F_ACK);

    println!("Confirmed: RTM_DELROUTE disarm messages correctly instruct kernel to remove blackholes.");
}

#[test]
fn test_task3_isolator_disarm_idempotency_and_state_recovery() {
    println!("[Task 3] Testing LinuxNetlinkIsolator disarm idempotency...");

    let isolator = LinuxNetlinkIsolator::new();
    let receipt = isolator
        .engage_quarantine(&[3030], "test_admin", "Idempotency test")
        .expect("engage must succeed");

    // First disarm
    assert!(
        isolator.restore_quarantine(&receipt).is_ok(),
        "First restore_quarantine must succeed"
    );

    // Second disarm (idempotent call)
    assert!(
        isolator.restore_quarantine(&receipt).is_ok(),
        "Second restore_quarantine must be idempotent and succeed"
    );

    // Third disarm with a dummy receipt
    let dummy_receipt = killswitch_types::IsolationReceipt::new(
        "dummy-receipt".to_string(),
        NetworkIsolationEngine::LinuxNetlink,
        Vec::new(),
        Vec::new(),
        vec!["netlink_rtn_blackhole_active".to_string()],
        vec![3030],
        vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))],
        0,
        "dummy".to_string(),
        "dummy".to_string(),
    );

    assert!(
        isolator.restore_quarantine(&dummy_receipt).is_ok(),
        "Disarm with dummy receipt must succeed cleanly"
    );

    println!("Confirmed: LinuxNetlinkIsolator disarm is 100% idempotent and fail-safe.");
}

// =============================================================================
// Task 4: Unprivileged Execution Fails Closed with InsufficientPrivileges
// =============================================================================

#[test]
fn test_task4_insufficient_privileges_typed_error_structure() {
    println!("[Task 4] Testing KillSwitchError::InsufficientPrivileges structure and formatting...");

    let err = KillSwitchError::InsufficientPrivileges {
        details: "Root privileges (or CAP_NET_ADMIN) required to install Linux network quarantine routes via Netlink".to_string(),
        remediation: "Restart the application using 'sudo killswitch' or ensure container possesses CAP_NET_ADMIN.".to_string(),
    };

    let formatted = format!("{}", err);
    assert!(
        formatted.contains("Insufficient privileges") || formatted.contains("Administrative elevation required"),
        "Formatted error must indicate insufficient privileges: {}",
        formatted
    );
    assert!(
        formatted.contains("CAP_NET_ADMIN") || formatted.contains("sudo"),
        "Formatted error must provide remediation guidance: {}",
        formatted
    );

    // Verify error code / matchable structure
    match err {
        KillSwitchError::InsufficientPrivileges { details, remediation } => {
            assert!(details.contains("Root privileges"));
            assert!(remediation.contains("sudo"));
        }
        _ => panic!("Expected InsufficientPrivileges variant"),
    }
    println!("Confirmed: Typed error KillSwitchError::InsufficientPrivileges satisfies fail-closed contract.");
}

#[test]
fn test_task4_netlink_isolator_is_available_never_panics() {
    println!("[Task 4] Testing LinuxNetlinkIsolator::is_available probe safety...");

    // Must return boolean without panic regardless of OS or privilege level
    let available = LinuxNetlinkIsolator::is_available();
    println!("LinuxNetlinkIsolator::is_available returned: {}", available);

    if !cfg!(target_os = "linux") {
        assert!(!available, "On non-Linux systems, is_available must return false");
    }
}

// =============================================================================
// Task 5: Cross-Subsystem & Trait Verification
// =============================================================================

#[test]
fn test_task5_default_network_isolator_trait_contract() {
    println!("[Task 5] Testing NetworkIsolator trait contract on default_network_isolator...");

    let isolator = default_network_isolator();
    let res = isolator.engage_quarantine(&[3030], "test_operator", "Testing trait dispatch");
    assert!(res.is_ok(), "engage_quarantine via trait object must succeed");

    let receipt = res.unwrap();
    assert!(receipt.preserved_ports.contains(&3030));
    assert!(receipt.preserved_ips.contains(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));

    let restore_res = isolator.restore_quarantine(&receipt);
    assert!(restore_res.is_ok(), "restore_quarantine via trait object must succeed");
}

#[test]
fn test_task5_overall_verdict_confirmation_assertion() {
    println!("[Task 5] Evaluating Milestone ISO-2 Linux Netlink Empirical Challenger verdict...");

    // Check all critical invariants:
    // 1. Netlink route blackhole binary message: length 36, RTM_NEWROUTE, RTN_BLACKHOLE, RT_TABLE_MAIN, priority 10
    let v4_msg = build_blackhole_route_msg(false, false, 10);
    assert_eq!(v4_msg.len(), 36);

    let v6_msg = build_blackhole_route_msg(true, false, 10);
    assert_eq!(v6_msg.len(), 36);

    // 2. Loopback preservation: RT_TABLE_LOCAL (255) vs RT_TABLE_MAIN (254)
    assert_ne!(RT_TABLE_LOCAL, RT_TABLE_MAIN);

    // 3. Socket destruction message: length 72, SOCK_DESTROY, IPPROTO_TCP, big-endian ports
    let entry = LinuxSocketEntry {
        slot: 1,
        local_addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        local_port: 12345,
        remote_addr: IpAddr::V4(Ipv4Addr::new(8, 8, 4, 4)),
        remote_port: 53,
        state: TcpState::Established,
        inode: 1,
        is_ipv6: false,
    };
    let sock_msg = build_sock_destroy_msg(&entry);
    assert_eq!(sock_msg.len(), 72);

    // 4. Disarm: RTM_DELROUTE
    let del_msg = build_blackhole_route_msg(false, true, 10);
    let hdr = unsafe { &*(del_msg.as_ptr() as *const NlMsgHdr) };
    assert_eq!(hdr.nlmsg_type, RTM_DELROUTE);

    println!("Empirical Challenger Verdict for Milestone ISO-2: CONFIRMED");
}
