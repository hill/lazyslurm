# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Cross-platform GitHub Actions CI/CD pipeline
- Automated binary releases for Linux, macOS, and Windows
- Homebrew formula
- README with installation instructions

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