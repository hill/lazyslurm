# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2026-08-10

### Added
- Themes. Nine ship with the app, `lazyslurm` (the default), `gruvbox-dark`,
  `gruvbox-light`, `catppuccin-mocha`, `catppuccin-latte`, `nord`, `dracula`,
  `tokyonight` and `tokyonight-day`. Press `T` for a picker that applies each one as you
  move through it
- Pick a theme with `--theme`, the `LAZYSLURM_THEME` environment variable, or
  `theme = "..."` in `~/.config/lazyslurm/config.toml`. Confirming in the picker writes
  the choice to that file, leaving the rest of it untouched
- Custom themes from `~/.config/lazyslurm/themes/*.toml`, named after the file and able
  to `extends` a built-in and override only the slots you care about
- `--list-themes` and `--print-theme`, the latter emitting the active palette as a
  starting point for your own
- Nix flake, so you can run LazySlurm with `nix run github:hill/lazyslurm` or get a dev
  shell with `nix develop`. Thanks to @ashan-p for contributing it! (#11)

## [0.4.0] - 2026-07-25

### Added
- Usage tab showing your fairshare standing from `sshare`
- Jobs default to your own user now; press `a` to toggle between your jobs and everyone's
- Update notifications: a badge appears in the status bar when a newer release is on
  crates.io, and clicking it opens the crates.io page. Opt out with `--no-update-check`
  or the `LAZYSLURM_NO_UPDATE_CHECK` environment variable

### Fixed
- Nodes tab no longer merges the CPU and memory columns for nodes with large core
  counts or memory. Thanks to @joan-aluja-oraa for the fix (#10)
- User filter no longer falls back to showing all users when `$USER` is unset (for
  example under `docker exec`); it now resolves your login name reliably
- Corrected the binary download URLs in the README

## [0.3.1] - 2026-06-26

### Fixed
- Cancel-job confirmation popup now shows its prompt and y/n help inside the
  window instead of clipping them on normal-sized terminals

## [0.3.0] - 2026-06-26

### Added
- New aesthetic redesign
- Tab views for Jobs, History, Partitions, and Nodes
- Expand panels to full screen, or raw mode for easy text copying
- Job history via `sacct`

## [0.1.0] - 2025-09-05

### Added
- Initial release of LazySlurm
- Real-time SLURM job monitoring with terminal UI
- Job list view with status indicators
- Job details panel with comprehensive information
- Job log tailing functionality
- Keyboard navigation (q: quit, ↑/↓: navigate, r: refresh, c: cancel)
- SLURM parser for job data extraction
- Development environment with Docker and mock jobs
- Support for user filtering and job management

### Dependencies
- ratatui 0.28 for terminal UI
- crossterm 0.28 for cross-platform terminal handling
- clap 4.5 for CLI argument parsing
- tokio 1.0 for async runtime
- chrono 0.4 for date/time handling
- anyhow 1.0 for error handling
- regex 1.10 for parsing
- serde 1.0 for serialization

### Development
- Docker-based SLURM development environment
- Just command runner for development tasks
- Mock job generation for testing
- Incremental compilation optimization
