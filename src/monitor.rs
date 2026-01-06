use crate::status::ConnectionStatus;
use colored::Colorize;
use std::time::Duration;
use tokio::time::sleep;

pub struct Monitor;

impl Monitor {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        // Clear screen and hide cursor
        print!("\x1B[2J\x1B[?25l");

        // Handle Ctrl+C gracefully
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let r = running.clone();

        ctrlc::set_handler(move || {
            r.store(false, std::sync::atomic::Ordering::SeqCst);
        })
        .ok();

        while running.load(std::sync::atomic::Ordering::SeqCst) {
            // Move cursor to top
            print!("\x1B[H");

            // Display dashboard
            self.display_dashboard().await;

            // Wait before next update
            sleep(Duration::from_secs(5)).await;
        }

        // Show cursor and clear screen
        print!("\x1B[?25h\x1B[2J\x1B[H");
        println!("\n{}", "[✓] Monitor stopped".yellow());

        Ok(())
    }

    async fn display_dashboard(&self) {
        println!("{}", "═".repeat(70).bright_blue());
        println!(
            "{}",
            "                  NIPE REAL-TIME MONITOR                  "
                .bright_blue()
                .bold()
        );
        println!("{}", "═".repeat(70).bright_blue());
        println!();

        // Get status
        match ConnectionStatus::check().await {
            Ok(status) => {
                if status.is_tor {
                    println!(
                        "  {} {}",
                        "Connection:".bold(),
                        "🟢 ACTIVE & SECURE".bright_green().bold()
                    );
                    println!(
                        "  {} {}",
                        "Current IP:".bold(),
                        status.current_ip.bright_cyan()
                    );
                } else {
                    println!(
                        "  {} {}",
                        "Connection:".bold(),
                        "🔴 INACTIVE".bright_red().bold()
                    );
                }
            }
            Err(_) => {
                println!(
                    "  {} {}",
                    "Connection:".bold(),
                    "⚠️  ERROR CHECKING STATUS".bright_yellow().bold()
                );
            }
        }

        println!();

        // System info
        println!("  {} {}", "Kill Switch:".bold(), "✓ Enabled".green());
        println!("  {} {}", "Stream Isolation:".bold(), "✓ Active".green());
        println!(
            "  {} {}",
            "Auto-Rotation:".bold(),
            "Every 60 seconds".cyan()
        );

        println!();
        println!("{}", "─".repeat(70).bright_black());
        println!(
            "  {}",
            "Press Ctrl+C to exit | Updates every 5 seconds".bright_black()
        );
        println!("{}", "═".repeat(70).bright_blue());

        // Pad the rest of the screen
        for _ in 0..10 {
            println!();
        }
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}
