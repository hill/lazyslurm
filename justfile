# Development commands for LazySlurm

# list available recipes
[private]
default:
    @just --list --unsorted

# Build and start SLURM development environment
slurm_up:
    cd dev && docker-compose build
    cd dev && docker-compose up -d
    @echo "SLURM container started!"
    @echo "Get shell: just slurm_shell"

# Get into SLURM container for development
slurm_shell:
    docker exec -it lazyslurm_dev bash

# Submit test jobs from host
slurm_populate:
    @echo "Submitting test jobs..."
    docker exec lazyslurm_dev sbatch --wrap="echo 'Starting job 1...'; i=1; while [ \$i -le 30 ]; do echo 'Job 1 progress: step '\$i'/30'; sleep 2; i=\$((i+1)); done; echo 'Job 1 completed!'" --job-name=test_job_1 --output=/tmp/slurm-%j.out --error=/tmp/slurm-%j.err
    docker exec lazyslurm_dev sbatch --wrap="echo 'Starting long job...'; i=1; while [ \$i -le 60 ]; do echo 'Long job processing batch '\$i'/60'; sleep 2; i=\$((i+1)); done; echo 'Long job finished!'" --job-name=long_job --output=/tmp/slurm-%j.out --error=/tmp/slurm-%j.err
    docker exec lazyslurm_dev sbatch --wrap="echo 'Quick job starting...'; i=1; while [ \$i -le 15 ]; do echo 'Quick task '\$i'/15 complete'; sleep 1; i=\$((i+1)); done; echo 'Quick job done!'" --job-name=quick_job --output=/tmp/slurm-%j.out --error=/tmp/slurm-%j.err
    @echo "Jobs submitted!"

# Check SLURM status
slurm_status:
    docker exec lazyslurm_dev squeue
    @echo ""
    docker exec lazyslurm_dev sinfo

# Stop SLURM environment
slurm_down:
    cd dev && docker-compose down

# Build and run tests
test:
    cargo test

# Lint with Clippy
lint:
    cargo clippy -- -D warnings
# Clean up everything
clean:
    cd dev && docker-compose down -v
    docker system prune -f
    cargo clean

# Show running jobs in a watch loop
watch_jobs:
    watch -n 2 "docker exec lazyslurm_dev squeue"