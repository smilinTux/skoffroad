# Contributing to SandK Offroad

Thank you for considering contributing to SandK Offroad!

## Branching Strategy
- Use `main` for stable releases.
- Use `develop` for ongoing development.
- Feature branches: `feature/<short-description>`
- Bugfix branches: `bugfix/<short-description>`
- Use pull requests for all merges to `main` or `develop`.

## Code Style
- Follow Rust 2021 edition and PEP8 for Python scripts.
- Use `rustfmt` for formatting (see `rustfmt.toml`).
- Use `clippy` for linting (see `clippy.toml`).
- Follow rules in `.cursor/rules/` for code conventions and workflow.

## Testing
- All new features must include tests in `/tests` mirroring the main app structure.
- Run `cargo test` before submitting a PR.

## CI/CD
- All PRs must pass CI checks (see `.github/workflows/rust.yml`).

## Development Environment
- Use the toolchain specified in `rust-toolchain.toml`.
- See `README.md` for setup instructions.

## Commit Messages
- Use clear, descriptive commit messages.
- Reference task IDs from Task Master when possible.

## Questions?
Open an issue or start a discussion! 