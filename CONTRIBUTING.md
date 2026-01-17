# Contributing to Nipe

Thank you for your interest in contributing to Nipe! We welcome improvements, bug fixes, and new features.

## Getting Started

1.  **Fork the repository** on GitHub.
2.  **Clone your fork**:
    ```bash
    git clone https://github.com/your-username/nipe-Tor.git
    cd nipe
    ```
3.  **Create a branch** for your changes:
    ```bash
    git checkout -b feature/amazing-feature
    ```

## Development Environment

-   **Rust**: Ensure you have the latest stable Rust toolchain (`rustup update`).
-   **Tor**: Tor must be installed on your system (`brew install tor` or `apt install tor`).
-   **Permissions**: Nipe requires root/sudo for firewall manipulation.

## Code Style

-   Run `cargo fmt` to format your code.
-   Run `cargo clippy` to ensure your code is idiomatic and error-free.
-   We use [Rust formatting guidelines](https://github.com/rust-lang/rustfmt).

## Submission Process

1.  **Commit your changes** with clear messages.
2.  **Push to your fork**.
3.  **Open a Pull Request** against the `main` branch.
4.  Describe your changes and why they are needed.

## Testing

Run the test suite before submitting:
```bash
cargo test
```

## Security

If you discover a security vulnerability, please do NOT open a public issue. Report it privately to the maintainers.
