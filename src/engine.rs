use crate::config::NipeConfig;
use crate::error::{NipeError, Result};
use crate::platform::{Firewall, FirewallProvider};
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tracing::{debug, info, warn};

pub struct NipeEngine {
    config: NipeConfig,
    tor_process: Option<Child>,
    tor_user: Option<(u32, u32)>, // uid, gid
}

impl NipeEngine {
    pub fn new(config: NipeConfig) -> Result<Self> {
        Ok(Self {
            config,
            tor_process: None,
            tor_user: None,
        })
    }

    fn find_tor_user() -> Option<(u32, u32)> {
        // Try standard Tor users
        let users = ["debian-tor", "tor", "nobody"];

        for user in users {
            let output = std::process::Command::new("id")
                .arg("-u")
                .arg(user)
                .output()
                .ok()?;

            if output.status.success() {
                let uid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(uid) = uid_str.parse::<u32>() {
                    // Get GID as well
                    let output_gid = std::process::Command::new("id")
                        .arg("-g")
                        .arg(user)
                        .output()
                        .ok()?;

                    let gid_str = String::from_utf8_lossy(&output_gid.stdout)
                        .trim()
                        .to_string();
                    if let Ok(gid) = gid_str.parse::<u32>() {
                        info!(
                            "Found unprivileged user for Tor: {} (uid: {}, gid: {})",
                            user, uid, gid
                        );
                        return Some((uid, gid));
                    }
                }
            }
        }
        warn!("No unprivileged user found. Tor will run as root!");
        None
    }

    fn find_tor_path() -> String {
        let common_paths = [
            "/usr/bin/tor",
            "/usr/sbin/tor",
            "/usr/local/bin/tor",
            "/opt/homebrew/bin/tor", // macOS Apple Silicon
            "/opt/local/bin/tor",    // MacPorts
        ];

        for path in common_paths {
            if std::path::Path::new(path).exists() {
                return path.to_string();
            }
        }

        // Fallback to system PATH
        "tor".to_string()
    }

    fn set_owner(path: &std::path::Path, uid: u32, gid: u32) -> Result<()> {
        use std::os::unix::ffi::OsStrExt;
        let path_c = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|e| NipeError::Other(e.to_string()))?;

        unsafe {
            if libc::chown(path_c.as_ptr(), uid, gid) != 0 {
                return Err(NipeError::Other(format!("Failed to chown {:?}", path)));
            }
        }
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Nipe engine");

        // 1. Stop any existing instance
        let _ = self.stop().await;

        match self.start_internal().await {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("Start failed, performing rollback: {}", e);
                let _ = self.stop().await;
                Err(e)
            }
        }
    }

    async fn start_internal(&mut self) -> Result<()> {
        // 2. Create data directory
        // 1.5 Find Tor user
        self.tor_user = Self::find_tor_user();

        // 2. Create data directory with secure permissions
        // Ensure parent dir exists
        let parent = self.config.tor.data_directory.parent().unwrap();
        debug!("Creating parent directory: {:?}", parent);
        std::fs::create_dir_all(parent)?;

        debug!(
            "Creating data directory: {:?}",
            self.config.tor.data_directory
        );
        std::fs::create_dir_all(&self.config.tor.data_directory)?;

        // Lock down permissions to 700 (rwx------)
        debug!("Setting permissions on data directory");
        std::fs::set_permissions(
            &self.config.tor.data_directory,
            Permissions::from_mode(0o700),
        )?;

        // Set ownership if we have a target user
        if let Some((uid, gid)) = self.tor_user {
            debug!("Setting owner on data directory to {}:{}", uid, gid);
            Self::set_owner(&self.config.tor.data_directory, uid, gid)?;
        }

        // 3. Generate torrc
        debug!("Generating torrc");
        let torrc_path = self.generate_torrc()?;
        debug!("Generated torrc at: {:?}", torrc_path);

        // Ensure torrc is readable by the user
        if let Some((uid, gid)) = self.tor_user {
            debug!("Setting owner on torrc");
            Self::set_owner(&torrc_path, uid, gid)?;
        }

        // 4. Start Tor process
        info!("Starting Tor process");
        // Redirect Tor logs to file
        let log_dir = self
            .config
            .tor
            .data_directory
            .parent()
            .unwrap()
            .to_path_buf();

        // Ensure log dir exists with secure permissions
        if !log_dir.exists() {
            std::fs::create_dir_all(&log_dir).map_err(|e| {
                NipeError::TorStartFailed(format!("Failed to create log directory: {}", e))
            })?;
        }

        // Determine ownership
        let (uid, gid) = if let Some((u, g)) = self.tor_user {
            (u, g)
        } else {
            // Fallback to current user (root) if no user found, but we warn about this
            unsafe { (libc::geteuid(), libc::getegid()) }
        };

        // Apply permissions to log dir
        std::fs::set_permissions(&log_dir, Permissions::from_mode(0o755))?; // needs to be readable
        Self::set_owner(&log_dir, uid, gid)?;

        let log_file_path = log_dir.join("tor.log");
        let log_file = std::fs::File::create(&log_file_path).map_err(|e| {
            NipeError::TorStartFailed(format!(
                "Failed to create log file {}: {}",
                log_file_path.display(),
                e
            ))
        })?;

        // Secure log file
        std::fs::set_permissions(&log_file_path, Permissions::from_mode(0o640))?;
        if let Some((u, g)) = self.tor_user {
            Self::set_owner(&log_file_path, u, g)?;
        }

        let stdout_log = log_file
            .try_clone()
            .map_err(|e| NipeError::TorStartFailed(format!("Failed to clone log handle: {}", e)))?;

        // Resolve absolute path to Tor to avoid PATH issues with sudo
        let tor_cmd = Self::find_tor_path();
        debug!("Using Tor binary at: {}", tor_cmd);

        let mut cmd = Command::new(tor_cmd);
        cmd.arg("-f")
            .arg(&torrc_path)
            .stdout(stdout_log)
            .stderr(log_file);

        // Drop privileges
        if let Some((u, g)) = self.tor_user {
            cmd.uid(u);
            cmd.gid(g);
        }

        let child = cmd
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
                if let Some(country) = &self.config.tor.country {
                    format!("ExitNodes {{{}}}\nStrictNodes 1", country)
                } else {
                    String::new()
                }
            } else {
                format!("ExitNodes {{{}}}", self.config.tor.exit_nodes.join(","))
            }
        );

        let path = self
            .config
            .tor
            .data_directory
            .parent()
            .unwrap()
            .join("torrc");
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
