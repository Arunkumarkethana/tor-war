# Abstract

**Project Title:** Nipe (Rust Edition): High-Performance Cross-Platform Tor Gateway with Advanced Censorship Circumvention

**Abstract:**

In an era of increasing digital surveillance and internet censorship, personal privacy tools remain either inaccessible to non-technical users or limited in application scope. This project presents **Nipe**, a complete rewrite of the original Perl-based anonymous network gateway, now engineered in **Rust** to prioritize memory safety, performance, and cross-platform compatibility.

Nipe acts as a system-wide **transparent proxy**, automatically routing all outgoing network traffic—including background processes, terminal commands, and third-party applications—through the **Tor anonymity network**. Unlike traditional browser-based solutions (like Tor Browser) which only secure web traffic, Nipe secures the entire operating system stack, preventing accidental identity leaks from non-configured applications.

**Key innovations include:**
1.  **Rust Architecture**: Leverages Rust’s ownership model to ensure memory safety and achieves a **200x performance improvement** over the legacy Perl implementation.
2.  **Universal Compatibility**: A unified codebase supporting **macOS (Intel/Apple Silicon), Linux, and Windows**, utilizing native firewall APIs (`pf`, `iptables`, and `netsh`) for seamless traffic interception.
3.  **Strict "Kill Switch" Mechanism**: An integrated fail-safe that instantly blocks all outbound network traffic if the Tor connection is interrupted, guaranteeing zero IP leakage.
4.  **Censorship Resistance**: Native support for Pluggable Transports (**obfs4 bridges**), allowing operation in highly restricted network environments (e.g., corporate firewalls or authoritarian regimes) where standard Tor connections are blocked.
5.  **Stream Isolation & Hygiene**: Implements automatic identity rotation and stream isolation to mitigate correlation attacks, decoupling user identity from network activity.

This project demonstrates that high-assurance privacy tools can be both performant and accessible, providing a "one-click" anonymity shield for diverse operating systems.
