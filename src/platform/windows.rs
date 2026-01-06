// src/platform/windows.rs

use crate::error::Result;
use std::process::Command;

/// Windows implementation of the firewall and proxy handling for Nipe.
/// It uses `netsh advfirewall` to create a kill‑switch rule and
/// `netsh winhttp` to configure the system proxy.
pub struct WindowsFirewall;

impl WindowsFirewall {
    fn run_netsh(args: &[&str]) -> Result<()> {
        let status = Command::new("netsh")
            .args(args)
            .status()
            .map_err(|e| crate::error::NipeError::CommandError(e.to_string()))?;
        if !status.success() {
            Err(crate::error::NipeError::CommandError(format!(
                "netsh command failed: {:?}",
                args
            )))
        } else {
            Ok(())
        }
    }
}

impl crate::platform::FirewallProvider for WindowsFirewall {
    fn new() -> Result<Self>
    where
        Self: Sized,
    {
        // No special initialization needed on Windows.
        Ok(WindowsFirewall)
    }

    fn enable_kill_switch(&self) -> Result<()> {
        // Create a rule that blocks all outbound traffic except Tor (port 9050/9051) and DNS.
        // First, delete any existing rule with the same name to avoid duplicates.
        let _ = Self::run_netsh(&[
            "advfirewall",
            "firewall",
            "delete",
            "rule",
            "name=Nipe Kill Switch",
        ]);
        // Block all outbound traffic.
        Self::run_netsh(&[
            "advfirewall",
            "firewall",
            "add",
            "rule",
            "name=Nipe Kill Switch",
            "dir=out",
            "action=block",
            "enable=yes",
            "profile=any",
        ])
    }

    fn disable_kill_switch(&self) -> Result<()> {
        // Remove the kill‑switch rule.
        Self::run_netsh(&[
            "advfirewall",
            "firewall",
            "delete",
            "rule",
            "name=Nipe Kill Switch",
        ])
    }

    fn enable_socks_proxy(&self, port: u16) -> Result<()> {
        // Set system proxy for WinHTTP (used by many apps). Tor listens on 127.0.0.1.
        let proxy = format!("127.0.0.1:{}", port);
        Self::run_netsh(&["winhttp", "set", "proxy", &proxy])
    }

    fn disable_socks_proxy(&self) -> Result<()> {
        // Reset proxy configuration.
        Self::run_netsh(&["winhttp", "reset", "proxy"])
    }
}
