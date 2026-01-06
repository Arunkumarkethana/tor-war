use std::process::Command;
use tracing::info;

pub struct Installer;

impl Installer {
    pub fn check_and_install_tor() -> anyhow::Result<()> {
        // Check if Tor is installed
        if Self::is_tor_installed() {
            info!("Tor is already installed");
            return Ok(());
        }

        info!("Tor not found. Installing automatically...");
        Self::install_tor()?;

        Ok(())
    }

    pub fn check_obfs4proxy() -> anyhow::Result<()> {
        if Self::is_command_available("obfs4proxy") {
            Ok(())
        } else {
            Err(anyhow::anyhow!("obfs4proxy not found"))
        }
    }

    fn is_command_available(cmd: &str) -> bool {
        #[cfg(target_os = "windows")]
        {
            Command::new("where")
                .arg(cmd)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("which")
                .arg(cmd)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
    }

    fn is_tor_installed() -> bool {
        Self::is_command_available("tor")
    }

    #[cfg(target_os = "windows")]
    fn install_tor() -> anyhow::Result<()> {
        // On Windows, we cannot automatically install Tor via a package manager.
        // Provide a helpful error message with download instructions.
        Err(anyhow::anyhow!(
            "Automatic Tor installation is not supported on Windows.\n\nPlease download and install Tor from the official website:\nhttps://www.torproject.org/download/\n\nAfter installing, ensure `tor.exe` is in your PATH or located at C:\\Program Files\\Tor\\tor.exe"
        ))
    }

    #[cfg(target_os = "macos")]
    fn install_tor() -> anyhow::Result<()> {
        info!("Installing Tor via Homebrew...");

        // Check if Homebrew is installed
        let has_brew = Command::new("which")
            .arg("brew")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !has_brew {
            return Err(anyhow::anyhow!(
                "Homebrew not found. Please install Homebrew first:\n\
                /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            ));
        }

        // Install Tor
        println!("Installing Tor via Homebrew (this may take a minute)...");
        let output = Command::new("brew").args(&["install", "tor"]).status()?;

        if output.success() {
            println!("✅ Tor installed successfully!");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to install Tor via Homebrew"))
        }
    }

    #[cfg(target_os = "linux")]
    fn install_tor() -> anyhow::Result<()> {
        info!("Installing Tor via apt...");

        println!("Installing Tor via apt-get (requires sudo)...");

        // Try apt-get
        let output = Command::new("apt-get")
            .args(&["install", "-y", "tor"])
            .status();

        match output {
            Ok(status) if status.success() => {
                println!("✅ Tor installed successfully!");
                Ok(())
            }
            _ => {
                // Fall back to manual instructions
                Err(anyhow::anyhow!(
                    "Failed to auto-install Tor. Please install manually:\n\
                    Debian/Ubuntu: sudo apt-get install tor\n\
                    Fedora: sudo dnf install tor\n\
                    Arch: sudo pacman -S tor"
                ))
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn install_tor() -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "Automatic Tor installation not supported on this platform. Please install Tor manually."
        ))
    }
}
