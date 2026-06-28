# Development commands for LazySlurm

# list available recipes
[private]
default:
    @just --list --unsorted

# Run lazyslurm
dev:
    cargo run

# Run lazyslurm inside the SLURM container (for local dev without SLURM).
# Uses a separate target dir so the container's Linux build never clobbers
# the host's macOS build artifacts in target/.
dev-docker:
    docker exec -it -e CARGO_TARGET_DIR=/workspace/target-linux lazyslurm_dev cargo run

# Build and start SLURM development environment
slurm_up:
    cd dev && docker compose build
    cd dev && docker compose up -d
    @echo "SLURM container started!"
    @echo "Get shell: just slurm_shell"

# Get into SLURM container for development
slurm_shell:
    docker exec -it lazyslurm_dev bash

# Submit the job zoo (multiple users, partitions, QOS, GPUs, array, deps)
slurm_populate:
    docker exec lazyslurm_dev bash /workspace/dev/populate.sh

# Cancel every queued/running job, then clear any drain a kill leaves behind
# (accounting history is kept)
slurm_clear_jobs:
    docker exec lazyslurm_dev bash -c 'ids=$(squeue -h -o %i | tr "\n" " "); [ -n "$ids" ] && scancel $ids; true'
    docker exec lazyslurm_dev scontrol update NodeName=slurmctld State=RESUME || true

# Check SLURM status
slurm_status:
    docker exec lazyslurm_dev squeue --format="%.8i %.12j %.8u %.8T %.10P %.6q %.12R"
    @echo ""
    docker exec lazyslurm_dev sinfo -o "%P %a %D %C %G"

# Show the scheduler's priority breakdown for queued jobs
slurm_prio:
    docker exec lazyslurm_dev sprio -l

# Show fairshare usage per account/user
slurm_share:
    docker exec lazyslurm_dev sshare -a

# Stop SLURM environment
slurm_down:
    cd dev && docker compose down

# Build and run tests
test:
    cargo test

# Lint with Clippy
lint:
    cargo clippy -- -D warnings
# Clean up everything
clean:
    cd dev && docker compose down -v
    docker system prune -f
    cargo clean

# Show running jobs in a watch loop
watch_jobs:
    watch -n 2 "docker exec lazyslurm_dev squeue"

# Release: bump version, tag, and push
# Usage:
#   just release                # bump patch
#   just release patch|minor|major
#   just release 1.2.3          # set explicit version
release version="patch":
    #!/usr/bin/env bash
    set -euo pipefail

    # Ensure clean working tree
    if ! git diff --quiet || ! git diff --cached --quiet; then
        echo "Error: Working tree has uncommitted changes." >&2
        exit 1
    fi

    # Compute new version and update Cargo.toml via helper script
    new_ver=$(python3 scripts/bump_version.py "{{version}}")

    echo "Bumped version to ${new_ver}"

    # Update Cargo.lock to reflect new root version
    if command -v cargo >/dev/null 2>&1; then
        cargo generate-lockfile >/dev/null 2>&1 || cargo check -q || true
    fi

    # Commit, tag, and push
    git add Cargo.toml Cargo.lock || git add Cargo.toml
    git commit -m "Release v${new_ver}"
    git tag -a "v${new_ver}" -m "Release v${new_ver}"
    git push
    git push --tags

    echo "Release v${new_ver} pushed. GitHub Actions will build and upload binaries."

    # Wait for CI to publish binaries, then update the Homebrew tap formula.
    # If interrupted, re-run with: just update-tap ${new_ver}
    just update-tap "${new_ver}"

# Update the Homebrew tap formula for a released version (waits for CI binaries)
# Usage: just update-tap 0.3.0
update-tap version:
    python3 scripts/update_tap.py "{{version}}"
