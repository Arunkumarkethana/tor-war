use crate::error::{NipeError, Result};
use crate::platform::FirewallProvider;
use std::process::Command;
use tracing::info;

pub struct LinuxFirewall {
    tor_user: String,
}

impl FirewallProvider for LinuxFirewall {
    fn new() -> Result<Self> {
        Ok(Self {
            tor_user: "debian-tor".to_string(), // Default Tor user on Debian/Ubuntu
        })
    }

    fn enable_kill_switch(&self) -> Result<()> {
        info!("Enabling Linux kill switch with iptables");

        // Flush existing rules
        Command::new("iptables")
            .args(&["-t", "nat", "-F", "OUTPUT"])
            .output()?;
        Command::new("iptables")
            .args(&["-t", "filter", "-F", "OUTPUT"])
            .output()?;

        // NAT table rules
        self.setup_nat_rules()?;

        // Filter table rules
        self.setup_filter_rules()?;

        info!("Kill switch enabled");
        Ok(())
    }

    fn disable_kill_switch(&self) -> Result<()> {
        info!("Disabling Linux kill switch");

        Command::new("iptables")
            .args(&["-t", "nat", "-F", "OUTPUT"])
            .output()?;
        Command::new("iptables")
            .args(&["-t", "filter", "-F", "OUTPUT"])
            .output()?;
        Command::new("iptables")
            .args(&["-t", "nat", "-X"])
            .output()?;
        Command::new("iptables")
            .args(&["-t", "filter", "-X"])
            .output()?;

        info!("Kill switch disabled");
        Ok(())
    }

    fn enable_socks_proxy(&self, _port: u16) -> Result<()> {
        // Linux doesn't have system-wide SOCKS proxy like macOS
        // Applications need to use the proxy directly
        info!("SOCKS proxy available at 127.0.0.1:{}", _port);
        Ok(())
    }

    fn disable_socks_proxy(&self) -> Result<()> {
        // No-op on Linux
        Ok(())
    }
}

impl LinuxFirewall {
    fn setup_nat_rules(&self) -> Result<()> {
        let commands = vec![
            vec![
                "-t",
                "nat",
                "-A",
                "OUTPUT",
                "-m",
                "state",
                "--state",
                "ESTABLISHED",
                "-j",
                "RETURN",
            ],
            vec![
                "-t",
                "nat",
                "-A",
                "OUTPUT",
                "-m",
                "owner",
                "--uid-owner",
                &self.tor_user,
                "-j",
                "RETURN",
            ],
            vec![
                "-t",
                "nat",
                "-A",
                "OUTPUT",
                "-p",
                "udp",
                "--dport",
                "53",
                "-j",
                "REDIRECT",
                "--to-ports",
                "9061",
            ],
            vec![
                "-t",
                "nat",
                "-A",
                "OUTPUT",
                "-p",
                "tcp",
                "--dport",
                "53",
                "-j",
                "REDIRECT",
                "--to-ports",
                "9061",
            ],
            vec![
                "-t",
                "nat",
                "-A",
                "OUTPUT",
                "-p",
                "tcp",
                "-j",
                "REDIRECT",
                "--to-ports",
                "9051",
            ],
        ];

        for args in commands {
            Command::new("iptables").args(&args).output()?;
        }

        Ok(())
    }

    fn setup_filter_rules(&self) -> Result<()> {
        let commands = vec![
            vec![
                "-t",
                "filter",
                "-A",
                "OUTPUT",
                "-m",
                "state",
                "--state",
                "ESTABLISHED",
                "-j",
                "ACCEPT",
            ],
            vec![
                "-t",
                "filter",
                "-A",
                "OUTPUT",
                "-m",
                "owner",
                "--uid-owner",
                &self.tor_user,
                "-j",
                "ACCEPT",
            ],
            vec!["-t", "filter", "-A", "OUTPUT", "-p", "udp", "-j", "REJECT"],
            vec!["-t", "filter", "-A", "OUTPUT", "-p", "icmp", "-j", "REJECT"],
        ];

        for args in commands {
            Command::new("iptables").args(&args).output()?;
        }

        Ok(())
    }
}
