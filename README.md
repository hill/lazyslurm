# LazySlurm

A terminal UI for SLURM job management. Like the awesome [lazygit](https://github.com/jesseduffield/lazygit) but for HPC.

## Controls

- `q` quit
- `↑/↓` navigate 
- `r` refresh
- `c` cancel job

## Development

Requires Docker and [just](https://github.com/casey/just).

```bash
# Start test environment
just slurm_up
just slurm_populate

# Run LazySlurm
just run
```

```bash
just               # list commands
just slurm_up      # start SLURM container
just slurm_status  # check cluster
just run           # run LazySlurm
```

The Docker setup gives you a working SLURM cluster with sample jobs. Real squeue/scontrol commands, real job IDs, real log paths.

## Why This Exists

SLURM's CLI is powerful but clunky for monitoring. This gives you the lazygit experience: see state, take actions, see new state. Real-time updates, keyboard navigation, visual feedback.

Built in Rust with ratatui because single binaries are beautiful on HPC systems.