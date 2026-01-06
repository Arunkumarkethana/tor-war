use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use tracing::info;

mod config;
mod engine;
mod error;
mod installer;
mod monitor;
mod platform;
mod status;

use config::NipeConfig;
use engine::NipeEngine;

#[derive(Parser)]
#[command(name = "nipe")]
#[command(version, about = "Route all traffic through Tor network", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start Nipe (enable Tor routing)
    Start,
    /// Stop Nipe (disable Tor routing)
    Stop,
    /// Check connection status
    Status,
    /// Rotate IP identity
    Rotate,
    /// Real-time monitoring dashboard
    Monitor,
    /// Restart Nipe
    Restart,
    /// Show current configuration
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Check for root/sudo
    if !is_root() {
        eprintln!(
            "{}",
            "Error: Nipe must be run as root (use sudo)"
                .bright_red()
                .bold()
        );
        std::process::exit(1);
    }

    let cli = Cli::parse();
    let config = NipeConfig::load().unwrap_or_default();

    match cli.command {
        Commands::Start => {
            println!("{}", "━".repeat(50).bright_blue());
            println!("{}", "  Starting Nipe...".bright_blue().bold());
            println!("{}", "━".repeat(50).bright_blue());

            // Check and install Tor if needed
            println!("{}", "[+] Checking Tor installation...".cyan());
            if let Err(e) = installer::Installer::check_and_install_tor() {
                eprintln!("{} {}", "[✗] Tor installation failed:".bright_red(), e);
                eprintln!(
                    "\n{}",
                    "Please install Tor manually and try again.".yellow()
                );
                std::process::exit(1);
            }

            // Check for obfs4proxy if bridges are enabled and no custom path is provided
            if config.tor.use_bridges && config.tor.client_transport_plugin.is_none() {
                if let Err(_) = installer::Installer::check_obfs4proxy() {
                    eprintln!("{}", "[!] Warning: Bridge support is enabled but 'obfs4proxy' was not found in PATH.".yellow());
                    eprintln!(
                        "{}",
                        "    Tor functionality might fail if bridges are required.".yellow()
                    );
                    eprintln!("{}", "    Please install it system-wide (e.g. 'brew install obfs4proxy' or 'apt install obfs4proxy').".yellow());
                }
            }

            // Auto-install to system path (OS-specific)
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
                // Existing Unix auto‑install logic
                if let Ok(current_exe) = std::env::current_exe() {
                    let install_dir = std::path::Path::new("/usr/local/bin");
                    let install_path = install_dir.join("nipe");
                    if current_exe != install_path {
                        println!("{}", "[+] Checking system-wide installation...".cyan());
                        if !install_dir.exists() {
                            let _ = std::fs::create_dir_all(install_dir);
                        }
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

            let mut engine = NipeEngine::new(config)?;

            match engine.start().await {
                Ok(_) => {
                    println!("{}", "[✓] Tor process started".green());
                    println!("{}", "[✓] Kill switch enabled".green());
                    println!("{}", "[✓] System proxy configured".green());
                    println!(
                        "\n{}",
                        "Nipe is now active - All traffic routed through Tor"
                            .bright_green()
                            .bold()
                    );
                    println!("{}", "━".repeat(50).bright_blue());
                }
                Err(e) => {
                    eprintln!("{} {}", "[✗] Failed to start:".bright_red(), e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Stop => {
            println!("{}", "━".repeat(50).bright_yellow());
            println!("{}", "  Stopping Nipe...".bright_yellow().bold());
            println!("{}", "━".repeat(50).bright_yellow());

            let mut engine = NipeEngine::new(config)?;

            match engine.stop().await {
                Ok(_) => {
                    println!("{}", "[✓] Tor process stopped".yellow());
                    println!("{}", "[✓] Kill switch disabled".yellow());
                    println!("{}", "[✓] System proxy removed".yellow());
                    println!(
                        "\n{}",
                        "Nipe stopped - Direct internet connection restored"
                            .bright_yellow()
                            .bold()
                    );
                    println!("{}", "━".repeat(50).bright_yellow());
                }
                Err(e) => {
                    eprintln!("{} {}", "[✗] Failed to stop:".bright_red(), e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Status => {
            info!("Checking status...");
            match status::ConnectionStatus::check().await {
                Ok(status) => status.display(),
                Err(e) => {
                    eprintln!("{} {}", "[✗] Failed to check status:".bright_red(), e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Rotate => {
            println!("{}", "[+] Rotating identity...".bright_cyan());

            let engine = NipeEngine::new(config)?;

            match engine.rotate().await {
                Ok(_) => {
                    println!("{}", "[✓] New identity acquired".bright_green());

                    // Show new IP
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    if let Ok(status) = status::ConnectionStatus::check().await {
                        println!("{} {}", "New IP:".bold(), status.current_ip.bright_cyan());
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", "[✗] Failed to rotate:".bright_red(), e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Monitor => {
            println!("{}", "Starting real-time monitor...".bright_blue());
            monitor::Monitor::new().run().await?;
        }

        Commands::Restart => {
            println!("{}", "Restarting Nipe...".bright_cyan());

            let mut engine = NipeEngine::new(config)?;

            // Stop first
            if let Err(e) = engine.stop().await {
                eprintln!("{} {}", "[!] Warning during stop:".yellow(), e);
            }

            // Wait a moment
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // Start again
            engine.start().await?;

            println!("{}", "[✓] Nipe restarted successfully".bright_green());
        }

        Commands::Config => {
            use std::io::Write;
            let mut stdout = std::io::stdout();
            let _ = writeln!(stdout, "{}", "Current Configuration:".bright_blue().bold());
            let _ = writeln!(stdout, "{}", "━".repeat(50).bright_blue());
            let _ = writeln!(stdout, "{:#?}", config);
        }
    }

    Ok(())
}

#[cfg(unix)]
fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(unix))]
fn is_root() -> bool {
    true // Windows check would go here
}
