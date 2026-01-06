// src/platform/mod.rs

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::MacOSFirewall as Firewall;

#[cfg(target_os = "linux")]
pub use linux::LinuxFirewall as Firewall;

#[cfg(target_os = "windows")]
pub use windows::WindowsFirewall as Firewall;

use crate::error::Result;

pub trait FirewallProvider {
    fn new() -> Result<Self>
    where
        Self: Sized;
    fn enable_kill_switch(&self) -> Result<()>;
    fn disable_kill_switch(&self) -> Result<()>;
    fn enable_socks_proxy(&self, port: u16) -> Result<()>;
    fn disable_socks_proxy(&self) -> Result<()>;
}
