use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub is_tor: bool,
    pub current_ip: String,
    pub exit_country: Option<String>,
}

impl ConnectionStatus {
    pub async fn check() -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all("socks5h://127.0.0.1:9050")?)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        match client
            .get("https://check.torproject.org/api/ip")
            .send()
            .await
        {
            Ok(response) => {
                let json: serde_json::Value = response.json().await?;

                Ok(Self {
                    is_tor: json["IsTor"].as_bool().unwrap_or(false),
                    current_ip: json["IP"].as_str().unwrap_or("Unknown").to_string(),
                    exit_country: None,
                })
            }
            Err(e) => {
                // Fallback: check if we can reach the internet directly
                Ok(Self {
                    is_tor: false,
                    // Show the actual error to the user for debugging
                    current_ip: format!("Not Connected ({})", e),
                    exit_country: None,
                })
            }
        }
    }

    pub fn display(&self) {
        println!("\n{}", "━".repeat(60).bright_blue());
        println!(
            "{}",
            "              NIPE CONNECTION STATUS              "
                .bright_blue()
                .bold()
        );
        println!("{}", "━".repeat(60).bright_blue());
        println!();

        if self.is_tor {
            println!(
                "  {} {}",
                "Status:".bold(),
                "🟢 CONNECTED (ANONYMOUS)".bright_green().bold()
            );
            println!(
                "  {} {}",
                "Current IP:".bold(),
                self.current_ip.bright_cyan()
            );
            println!(
                "  {} {}",
                "Protection:".bold(),
                "Kill Switch Active".bright_green()
            );
        } else {
            println!(
                "  {} {}",
                "Status:".bold(),
                "🔴 NOT CONNECTED".bright_red().bold()
            );
            println!(
                "  {} {}",
                "Current IP:".bold(),
                self.current_ip.bright_red()
            );
            println!("  {} {}", "Protection:".bold(), "None".bright_red());
        }

        println!();
        println!("{}", "━".repeat(60).bright_blue());
        println!();
    }
}
