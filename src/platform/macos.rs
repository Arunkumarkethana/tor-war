use crate::error::{NipeError, Result};
use crate::platform::FirewallProvider;
use std::process::Command;
use tracing::{info, warn};

pub struct MacOSFirewall {
    interface: String,
    service: Option<String>,
}

impl FirewallProvider for MacOSFirewall {
    fn new() -> Result<Self> {
        let interface = Self::detect_interface()?;
        let service = Self::detect_service(&interface).ok();

        Ok(Self { interface, service })
    }

    fn enable_kill_switch(&self) -> Result<()> {
        info!("Enabling macOS kill switch with PF");

        let pf_rules = format!(
            r#"
# Nipe Kill Switch Rules
ext_if = "{}"
tor_user = "root"

# Options
set block-policy drop
set skip on lo0

# Allow DNS for Tor bootstrap
pass out quick on $ext_if proto udp to any port 53 keep state

# Allow all TCP traffic from Tor (running as root)
pass out quick on $ext_if proto tcp user $tor_user keep state

# Block IPv6 entirely (prevent leaks)
block drop quick inet6 all

# Block everything else
block drop out quick on $ext_if all
"#,
            self.interface
        );

        let rules_path = "/tmp/nipe_pf.conf";
        std::fs::write(rules_path, pf_rules)?;

        // Enable PF with rules
        let output = Command::new("pfctl")
            .args(["-ef", rules_path])
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| NipeError::FirewallError(format!("Failed to enable PF: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("PF warning: {}", stderr);
        }

        info!("Kill switch enabled");
        Ok(())
    }

    fn disable_kill_switch(&self) -> Result<()> {
        info!("Disabling macOS kill switch");

        let output = Command::new("pfctl")
            .arg("-d")
            .output()
            .map_err(|e| NipeError::FirewallError(format!("Failed to disable PF: {}", e)))?;

        if !output.status.success() {
            warn!("Failed to disable PF, it may already be disabled");
        }

        // Clean up rules file
        let _ = std::fs::remove_file("/tmp/nipe_pf.conf");

        info!("Kill switch disabled");
        Ok(())
    }

    fn enable_socks_proxy(&self, port: u16) -> Result<()> {
        info!("Enabling system SOCKS proxy on port {}", port);

        let default_service = "Wi-Fi".to_string();
        let service = self.service.as_ref().unwrap_or(&default_service);

        // Set SOCKS proxy
        let status = Command::new("networksetup")
            .args([
                "-setsocksfirewallproxy",
                service,
                "127.0.0.1",
                &port.to_string(),
            ])
            .status()
            .map_err(|e| NipeError::FirewallError(e.to_string()))?;

        if !status.success() {
            return Err(NipeError::FirewallError(
                "Failed to set SOCKS proxy".to_string(),
            ));
        }

        // Enable it
        let status = Command::new("networksetup")
            .args(["-setsocksfirewallproxystate", service, "on"])
            .status()?;

        if !status.success() {
            return Err(NipeError::FirewallError(
                "Failed to enable SOCKS proxy".to_string(),
            ));
        }

        info!("System SOCKS proxy enabled on {}", service);
        Ok(())
    }

    fn disable_socks_proxy(&self) -> Result<()> {
        info!("Disabling system SOCKS proxy");

        let default_service = "Wi-Fi".to_string();
        let service = self.service.as_ref().unwrap_or(&default_service);

        // Disable SOCKS proxy
        let _ = Command::new("networksetup")
            .args(["-setsocksfirewallproxystate", service, "off"])
            .status();

        self.disable_kill_switch()?;

        info!("System SOCKS proxy disabled");
        Ok(())
    }
}

impl MacOSFirewall {
    fn detect_interface() -> Result<String> {
        let output = Command::new("route")
            .args(["get", "default"])
            .output()
            .map_err(|_e| NipeError::InterfaceNotFound)?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        for line in output_str.lines() {
            if line.contains("interface:") {
                if let Some(interface) = line.split_whitespace().last() {
                    info!("Detected network interface: {}", interface);
                    return Ok(interface.to_string());
                }
            }
        }

        Err(NipeError::InterfaceNotFound)
    }

    fn detect_service(interface: &str) -> Result<String> {
        let output = Command::new("networksetup")
            .arg("-listallhardwareports")
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = output_str.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.contains(interface) && i > 0 {
                let service_line = lines[i - 1];
                if let Some(service) = service_line.split(':').nth(1) {
                    let service = service.trim().to_string();
                    info!("Detected network service: {}", service);
                    return Ok(service);
                }
            }
        }

        info!("Could not detect service name, using default 'Wi-Fi'");
        Ok("Wi-Fi".to_string())
    }
}
