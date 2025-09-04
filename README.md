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

Requires SLURM and [just](https://github.com/casey/just).

```bash
# Install SLURM
brew install slurm

# Start local cluster (terminal 1)
just slurm_up

# Submit test jobs (terminal 2)
just slurm_populate

# Run LazySlurm
just run
```

The local SLURM setup runs real slurmctld/slurmd daemons on your machine. Real commands, real job IDs, real log paths in `dev/test_data/`.

## Why This Exists

SLURM's CLI is powerful but clunky for monitoring. This gives you the lazygit experience: see state, take actions, see new state. Real-time updates, keyboard navigation, visual feedback.

Built in Rust with ratatui because single binaries are beautiful on HPC systems.