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
    docker exec lazyslurm_dev sbatch --wrap="sleep 60; echo Job 1 done" --job-name=test_job_1
    docker exec lazyslurm_dev sbatch --wrap="sleep 120; echo Job 2 done" --job-name=long_job
    docker exec lazyslurm_dev sbatch --wrap="sleep 30; echo Job 3 done" --job-name=quick_job
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

# Clean up everything
clean:
    cd dev && docker-compose down -v
    docker system prune -f
    cargo clean

# Show running jobs in a watch loop
watch_jobs:
    watch -n 2 "docker exec lazyslurm_dev squeue"