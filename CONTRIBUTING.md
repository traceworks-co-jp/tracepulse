# Contributing to TracePulse

Thank you for your interest in contributing to TracePulse. Contributions are welcome, including bug fixes, feature improvements, documentation updates, and vendor template changes.

## Before You Start: Issue First

For typos, documentation-only changes, and small, self-contained bug fixes, you may open a Pull Request directly.

For a new feature or a large specification, architectural, or behavioral change, please discuss it in a GitHub Issue or Discussion **before opening a Pull Request**. Early discussion helps confirm that the proposal fits TracePulse's goals, including its Rust implementation, portable single-binary distribution, and TUI/Web UI support. It also helps avoid rework or a rejected Pull Request.

For usage questions and environment-specific support, please use [GitHub Discussions](https://github.com/traceworks-co-jp/trace-pulse/discussions).

## Local Development Setup

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

Run the interactive terminal interface (TUI):

```bash
cargo run -- --cli
```

Run the embedded Web UI:

```bash
cargo run -- --web
```

When using Web UI mode, open `http://localhost:8080` in a browser after the server starts. TracePulse uses local configuration and SQLite data files during development; do not include local databases, credentials, or environment-specific secrets in a Pull Request.

## Code Quality

Before submitting a Pull Request, run all of the following commands locally from the repository root and make sure they pass:

```bash
# Check formatting
cargo fmt --check

# Run Clippy with warnings treated as errors
cargo clippy -- -D warnings

# Run the test suite
cargo test
```

Please keep changes focused and follow the existing Rust module structure and project conventions. Add or update tests when behavior changes. Documentation changes should describe the behavior that is actually implemented.

## Commit Messages

New commit messages are strongly recommended to be written in English. Use the [Conventional Commits](https://www.conventionalcommits.org/) format:

```text
<type>(optional scope): <short description>
```

Common types include:

- `feat:` A new feature, such as `feat(snmp): add SNMPv3 support`
- `fix:` A bug fix, such as `fix(tui): prevent layout overflow on narrow terminals`
- `docs:` Documentation changes
- `refactor:` Code changes that do not alter behavior
- `test:` Adding or updating tests
- `chore:` Maintenance changes

Use a concise, imperative description and keep unrelated changes in separate commits when practical.

## Pull Request Workflow

1. Start from an up-to-date `main` branch and create a feature branch. Do not work directly on `main`.

   ```bash
   git switch main
   git pull origin main
   git switch -c feature/short-description
   ```

2. Make the smallest focused change that addresses the Issue or agreed proposal.
3. Run the complete code quality checks listed above.
4. Commit your changes using the commit message guidelines.
5. Push the feature branch to your fork or working remote.

   ```bash
   git push -u origin feature/short-description
   ```

6. Open a Pull Request against the official repository's `main` branch.
7. Respond to review feedback and keep the branch up to date if requested.

### Pull Request Checklist

Before submitting your Pull Request, confirm that:

- [ ] The change is linked to an existing Issue or prior Discussion when it is a new feature or large change.
- [ ] The Pull Request description explains what changed and why.
- [ ] The change is limited to the intended scope.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy -- -D warnings` passes.
- [ ] `cargo test` passes.
- [ ] Tests or documentation have been updated where needed.
- [ ] No credentials, local databases, generated build output, or unrelated files are included.
- [ ] The target branch is `main`.

Thank you for helping make TracePulse more useful and reliable.