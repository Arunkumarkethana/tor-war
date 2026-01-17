use crate::config::NipeConfig;
use colored::Colorize;
use std::process::Command;
use tracing::info;

pub struct Installer;

impl Installer {
    pub fn ensure_prerequisites(config: &NipeConfig) -> anyhow::Result<()> {
        // 1. Check Tor
        println!("{}", "[+] Checking Tor installation...".cyan());
        if let Err(e) = Self::check_and_install_tor() {
            eprintln!("{} {}", "[✗] Tor installation failed:".bright_red(), e);
            eprintln!(
                "\n{}",
                "Please install Tor manually and try again.".yellow()
            );
            std::process::exit(1);
        }

        // 2. Check obfs4proxy
        if config.tor.use_bridges
            && config.tor.client_transport_plugin.is_none()
            && Self::check_obfs4proxy().is_err()
        {
            eprintln!(
                "{}",
                "[!] Warning: Bridge support is enabled but 'obfs4proxy' was not found in PATH."
                    .yellow()
            );
            eprintln!(
                "{}",
                "    Tor functionality might fail if bridges are required.".yellow()
            );
            eprintln!("{}", "    Please install it system-wide (e.g. 'brew install obfs4proxy' or 'apt install obfs4proxy').".yellow());
        }

        // 3. Self-install
        Self::check_and_install_system_wide()?;

        Ok(())
    }

    fn check_and_install_system_wide() -> anyhow::Result<()> {
        #[cfg(target_os = "windows")]
        {
            let install_dir = std::path::Path::new("C:\\Program Files\\Nipe");
            let install_path = install_dir.join("nipe.exe");
            if let Ok(current_exe) = std::env::current_exe() {
                if current_exe != install_path {
                    println!("{}", "[+] Checking system-wide installation...".cyan());
                    if !install_dir.exists() {
                        let _ = std::fs::create_dir_all(install_dir);
                    }
                    match std::fs::copy(&current_exe, &install_path) {
                        Ok(_) => println!(
                            "{}",
                            "[✓] Installed Nipe to C:\\Program Files\\Nipe\\nipe.exe".green()
                        ),
                        Err(e) => eprintln!(
                            "{} {}",
                            "[!] Failed to install system-wide (ignoring):".yellow(),
                            e
                        ),
                    }
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(current_exe) = std::env::current_exe() {
                let install_dir = std::path::Path::new("/usr/local/bin");
                let install_path = install_dir.join("nipe");
                // Don't try to install if we are running from the target path
                if current_exe != install_path {
                    println!("{}", "[+] Checking system-wide installation...".cyan());
                    if !install_dir.exists() {
                        let _ = std::fs::create_dir_all(install_dir);
                    }
                    // simple copy
                    match std::fs::copy(&current_exe, &install_path) {
                        Ok(_) => {
                            println!("{}", "[✓] Installed Nipe to /usr/local/bin/nipe".green())
                        }
                        Err(e) => eprintln!(
                            "{} {}",
                            "[!] Failed to install system-wide (ignoring):".yellow(),
                            e
                        ),
                    }
                }
            }
        }
        Ok(())
    }
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
        let output = Command::new("brew").args(["install", "tor"]).status()?;

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
