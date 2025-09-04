# LazySlurm

A terminal UI for SLURM job management. Like the awesome [lazygit](https://github.com/jesseduffield/lazygit) but for HPC.

## Controls

- `q` quit
- `↑/↓` navigate 
- `r` refresh
- `c` cancel job

## Installation

### Binary Releases

### Homebrew

### Cargo

### Gah

```sh
gah install hill/lazyslurm
```

## Development

Requires Docker and [just](https://github.com/casey/just).

```bash
# Start SLURM container
just slurm_up

# Get into container for development
just slurm_shell

# Inside container: your code is at /workspace
cargo run

# Submit test jobs (from host or container)
just slurm_populate
```

Your source code is mounted into the container so changes are immediately available.

## Why This Exists

SLURM's CLI is powerful but clunky for monitoring. This gives you the lazygit experience: see state, take actions, see new state. Real-time updates, keyboard navigation, visual feedback.

Built in Rust with ratatui because single binaries are beautiful on HPC systems.