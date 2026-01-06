use crate::config::NipeConfig;
use crate::error::{NipeError, Result};
use crate::platform::{Firewall, FirewallProvider};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tracing::info;

pub struct NipeEngine {
    config: NipeConfig,
    tor_process: Option<Child>,
}

impl NipeEngine {
    pub fn new(config: NipeConfig) -> Result<Self> {
        Ok(Self {
            config,
            tor_process: None,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Nipe engine");

        // 1. Stop any existing instance
        let _ = self.stop().await;

        // 2. Create data directory
        std::fs::create_dir_all(&self.config.tor.data_directory)?;

        // 3. Generate torrc
        let torrc_path = self.generate_torrc()?;

        // 4. Start Tor process
        info!("Starting Tor process");
        // Redirect Tor logs to file
        let log_dir = PathBuf::from("/tmp/nipe");
        std::fs::create_dir_all(&log_dir).map_err(|e| {
            NipeError::TorStartFailed(format!("Failed to create log directory: {}", e))
        })?;
        let log_file_path = log_dir.join("tor.log");
        let log_file = std::fs::File::create(&log_file_path).map_err(|e| {
            NipeError::TorStartFailed(format!(
                "Failed to create log file {}: {}",
                log_file_path.display(),
                e
            ))
        })?;

        let stdout_log = log_file
            .try_clone()
            .map_err(|e| NipeError::TorStartFailed(format!("Failed to clone log handle: {}", e)))?;

        let child = Command::new("tor")
            .arg("-f")
            .arg(&torrc_path)
            .stdout(stdout_log)
            .stderr(log_file)
            .spawn()
            .map_err(|e| NipeError::TorStartFailed(e.to_string()))?;

        self.tor_process = Some(child);

        // 5. Wait for Tor to bootstrap
        info!("Waiting for Tor to bootstrap");
        self.wait_for_bootstrap().await?;

        // 6. Configure firewall/kill switch
        info!("Configuring firewall");
        let firewall = Firewall::new()?;
        firewall.enable_kill_switch()?;
        firewall.enable_socks_proxy(self.config.tor.socks_port)?;

        info!("Nipe engine started successfully");

        // Detach Tor process so it keeps running after CLI exits
        // The Drop impl kills it if it's still in self.tor_process
        let _ = self.tor_process.take();

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping Nipe engine");

        // 1. Disable firewall
        let firewall = Firewall::new()?;
        firewall.disable_kill_switch()?;
        firewall.disable_socks_proxy()?;

        // 2. Stop Tor process
        if let Some(mut process) = self.tor_process.take() {
            info!("Killing Tor process");
            process
                .kill()
                .await
                .map_err(|e| NipeError::TorStopFailed(e.to_string()))?;
        } else {
            // Try to kill any running Tor process
            let _ = Command::new("pkill")
                .arg("-f")
                .arg("tor -f /tmp/nipe_torrc")
                .output()
                .await;
        }

        info!("Nipe engine stopped successfully");
        Ok(())
    }

    pub async fn rotate(&self) -> Result<()> {
        info!("Rotating Tor identity");

        // Send NEWNYM signal via control port
        let addr = format!("127.0.0.1:{}", self.config.tor.control_port);
        let mut stream = tokio::net::TcpStream::connect(&addr).await.map_err(|e| {
            NipeError::Other(format!("Failed to connect to Tor control port: {}", e))
        })?;

        // Authenticate (no password)
        stream.write_all(b"AUTHENTICATE \"\"\r\n").await?;

        // Send NEWNYM signal
        stream.write_all(b"SIGNAL NEWNYM\r\n").await?;

        info!("Identity rotation signal sent");
        Ok(())
    }

    async fn wait_for_bootstrap(&self) -> Result<()> {
        use tokio::time::{sleep, Duration};

        let max_attempts = 60; // Increased from 30 to 60 seconds
        for attempt in 0..max_attempts {
            if self.check_tor_connection().await.is_ok() {
                info!("Tor bootstrap complete");
                return Ok(());
            }

            if attempt % 5 == 0 {
                info!(
                    "Waiting for Tor bootstrap... ({}/{})",
                    attempt, max_attempts
                );
            }

            sleep(Duration::from_secs(1)).await;
        }

        Err(NipeError::BootstrapTimeout)
    }

    async fn check_tor_connection(&self) -> Result<()> {
        let proxy_url = format!("socks5h://127.0.0.1:{}", self.config.tor.socks_port);

        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(&proxy_url)?)
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let response = client
            .get("https://check.torproject.org/api/ip")
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        if json["IsTor"].as_bool() == Some(true) {
            Ok(())
        } else {
            Err(NipeError::NotConnected)
        }
    }

    fn generate_torrc(&self) -> Result<PathBuf> {
        // Handle Bridge Configuration
        let bridge_config = if self.config.tor.use_bridges {
            let mut config = String::from("\n# Bridge Configuration\nUseBridges 1\n");

            // 1. ClientTransportPlugin
            if let Some(path) = &self.config.tor.client_transport_plugin {
                config.push_str(&format!("ClientTransportPlugin obfs4 exec {}\n", path));
            } else {
                // Try to find obfs4proxy in path, otherwise fallback to standard paths
                #[cfg(not(target_os = "windows"))]
                let paths = [
                    "/usr/bin/obfs4proxy",
                    "/usr/local/bin/obfs4proxy",
                    "/opt/homebrew/bin/obfs4proxy",
                ];

                #[cfg(target_os = "windows")]
                let paths = [
                    r"C:\Program Files\Tor\obfs4proxy.exe",
                    r"C:\Program Files (x86)\Tor\obfs4proxy.exe",
                ];

                let found_path = paths.iter().find(|p| std::path::Path::new(p).exists());

                if let Some(p) = found_path {
                    // On Windows, paths with spaces must be quoted, but usually torrc handles exec paths well
                    // However, passing raw backslashes can be tricky.
                    config.push_str(&format!("ClientTransportPlugin obfs4 exec {}\n", p));
                } else {
                    // Fallback logic
                    #[cfg(not(target_os = "windows"))]
                    config.push_str("ClientTransportPlugin obfs4 exec /usr/bin/obfs4proxy\n");

                    #[cfg(target_os = "windows")]
                    config.push_str("ClientTransportPlugin obfs4 exec obfs4proxy.exe\n");
                    // Hope it's in PATH
                }
            }

            // 2. Add Bridges
            for bridge in &self.config.tor.bridges {
                config.push_str(&format!("Bridge {}\n", bridge));
            }
            config
        } else {
            String::new()
        };

        let torrc_content = format!(
            r#"
# Nipe Tor Configuration
SocksPort {}
ControlPort {}
DataDirectory {}

# Basic settings
Log notice stdout
DisableNetwork 0
{}
# Exit nodes preference (if specified)
{}
"#,
            self.config.tor.socks_port,
            self.config.tor.control_port,
            self.config.tor.data_directory.display(),
            bridge_config,
            if self.config.tor.exit_nodes.is_empty() {
                String::new()
            } else {
                format!("ExitNodes {{{}}}", self.config.tor.exit_nodes.join(","))
            }
        );

        let path = PathBuf::from("/tmp/nipe_torrc");
        std::fs::write(&path, torrc_content)?;

        Ok(path)
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &NipeConfig {
        &self.config
    }
}

impl Drop for NipeEngine {
    fn drop(&mut self) {
        if let Some(process) = self.tor_process.take() {
            let _ = std::process::Command::new("kill")
                .arg("-9")
                .arg(process.id().unwrap().to_string())
                .output();
        }
    }
}
