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
    Start {
        /// Select exit node country (e.g., "us", "de")
        #[arg(short, long)]
        country: Option<String>,
    },
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

    let cli = Cli::parse();
    let config = NipeConfig::load().unwrap_or_default();

    // Check for root/sudo unless just checking version/help (which clap handles before this)
    if !is_root() {
        eprintln!(
            "{}",
            "Error: Nipe must be run as root (use sudo)"
                .bright_red()
                .bold()
        );
        std::process::exit(1);
    }

    match cli.command {
        Commands::Start { country } => {
            println!("{}", "━".repeat(50).bright_blue());
            println!("{}", "  Starting Nipe...".bright_blue().bold());
            println!("{}", "━".repeat(50).bright_blue());

            // Ensure all prerequisites are met (Tor, self-install, bridges)
            installer::Installer::ensure_prerequisites(&config)?;

            // Prepare configuration (possibly overridden by CLI args)
            let run_config = if let Some(c) = country {
                let mut cfg = config.clone();
                cfg.tor.country = Some(c);
                cfg
            } else {
                config
            };

            let mut engine = NipeEngine::new(run_config)?;

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
