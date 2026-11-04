# Contributing to TracePulse

Thank you for your interest in TracePulse Core.

## Contribution Policy: Issues, Not External Code PRs

To keep code provenance and release review manageable, TracePulse Core accepts
external contributions through GitHub Issues and Discussions, not code pull
requests. Pull requests from external contributors are not accepted or merged
and may be closed without review. Pull requests are reserved for the project
owner and authorized maintainers/collaborators.

Reports and proposals are welcome, but an Issue does not guarantee that a
change will be implemented. The maintainer may investigate and implement a
change independently based on the reported behavior or requirements.

## How You Can Help

- **Bug reports**: Include the TracePulse version, operating system, steps to
   reproduce, expected behavior, and actual behavior.
- **Device and OID compatibility**: Include the vendor, exact model, firmware
   version, affected OID or feature, and sanitized command output where useful.
- **Feature requests**: Describe the problem, affected workflow, and expected
   outcome rather than submitting code.

Before posting logs or SNMP output, remove communities, passwords, tokens,
private addresses, serial numbers, and other sensitive information. Do not
upload proprietary MIB files or material you are not authorized to share.

For usage questions and environment-specific support, use
[GitHub Discussions](https://github.com/traceworks-co-jp/trace-pulse/discussions).
For suspected security vulnerabilities, follow the private reporting
instructions in [SECURITY.md](SECURITY.md); do not report them in a public Issue.

## Scope and Non-Goals of TracePulse Core

TracePulse Core covers self-hosted SNMP monitoring of multiple devices, CIDR discovery, local alert recording and display, and Web UI diagnostics. Issue reports about these capabilities, including vendor template compatibility, are welcome within the Community edition's documented limits.

Outbound alert delivery (including Slack, Teams, and email), multi-user RBAC, SIEM integrations, and Enterprise-only analytics are outside Core's scope. These capabilities are considered separately for TracePulse Enterprise; their mention here does not mean they are currently available. See the [Community Edition and Enterprise Features](README.md#community-edition-and-enterprise-features) section for the implementation boundary and edition limits.

Use a GitHub Issue or Discussion to ask whether a proposal fits this boundary.

## Reproducing Issues Locally

### Prerequisites

- Git
- Rust toolchain with the latest stable channel

Install Rust using [rustup](https://www.rust-lang.org/tools/install). Confirm that the stable toolchain is available:

```bash
rustup toolchain install stable
rustup default stable
rustc --version
cargo --version
```

### Clone and Run

Clone the repository and change into its directory:

```bash
git clone https://github.com/traceworks-co-jp/trace-pulse.git
cd trace-pulse
```

Build the project with Cargo:

```bash
cargo build
```

Run the embedded Web UI:

```bash
cargo run -- --web
```

When using Web UI mode, open `http://localhost:8080` in a browser after the server starts. TracePulse uses local configuration and SQLite data files during development; do not include local databases, credentials, or environment-specific secrets in a public Issue.

## Optional Local Validation

If you build TracePulse to reproduce an Issue, these checks can help verify the
local environment. They are not a request for an external code contribution.

```bash
# Check formatting
cargo fmt --check

# Run Clippy with warnings treated as errors
cargo clippy -- -D warnings

# Run the test suite
cargo test
```

### Optical transceiver templates

For optical transceiver templates, see the
[DOM/DDM schema and vendor mapping rules](docs/design/03-snmp-collection.md#optical-transceiver-power-dom--ddm).
Verify the actual power units, raw scale/sentinels, and mapping to ifIndex;
never assume sensor indices are interface indices. Include model/firmware and
sanitized validation evidence, and do not commit proprietary MIB source text.

Thank you for helping improve TracePulse through clear, reproducible reports.