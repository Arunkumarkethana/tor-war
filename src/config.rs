use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NipeConfig {
    pub tor: TorConfig,
    pub firewall: FirewallConfig,
    pub rotation: RotationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorConfig {
    pub socks_port: u16,
    pub control_port: u16,
    pub dns_port: u16,
    pub data_directory: PathBuf,
    #[serde(default)]
    pub use_bridges: bool,
    #[serde(default)]
    pub client_transport_plugin: Option<String>,
    #[serde(default)]
    pub bridges: Vec<String>,
    #[serde(default)]
    pub exit_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallConfig {
    pub enable_kill_switch: bool,
    pub allow_lan: bool,
    pub block_ipv6: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    pub auto_rotate: bool,
    pub interval_seconds: u64,
}

impl Default for NipeConfig {
    fn default() -> Self {
        Self {
            tor: TorConfig {
                socks_port: 9050,
                control_port: 9051,
                dns_port: 9061,
                data_directory: PathBuf::from("/tmp/nipe/tor-data"),
                use_bridges: false,
                client_transport_plugin: None,
                bridges: vec![],
                exit_nodes: vec![],
            },
            firewall: FirewallConfig {
                enable_kill_switch: true,
                allow_lan: true,
                block_ipv6: true,
            },
            rotation: RotationConfig {
                auto_rotate: true,
                interval_seconds: 60,
            },
        }
    }
}

impl NipeConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            let default = Self::default();
            default.save()?;
            Ok(default)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nipe");

        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;

        Ok(())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nipe")
            .join("config.toml")
    }
}
