# Development commands for LazySlurm

# list available recipes
[private]
default:
    @just --list --unsorted

# Build and start the SLURM test environment
slurm_up:
    mkdir -p dev/test_jobs dev/test_data
    cd dev && docker-compose build
    cd dev && docker-compose up -d
    @echo "Waiting for SLURM services to start..."
    sleep 15
    @echo "SLURM environment ready!"

# Stop the SLURM test environment  
slurm_down:
    cd dev && docker-compose down

# Get shell access to SLURM container
slurm_shell:
    docker exec -it lazyslurm_dev bash

# Check SLURM status
slurm_status:
    docker exec lazyslurm_dev sinfo
    docker exec lazyslurm_dev squeue

# Submit a test job
slurm_test_job name="test_job":
    docker exec lazyslurm_dev bash -c "cd /home/slurm/jobs && sbatch --job-name={{name}} --wrap='sleep 60; echo Job {{name}} completed'"

# Submit multiple test jobs for realistic testing
slurm_populate:
    just slurm_test_job "job_running_1"
    just slurm_test_job "job_running_2" 
    just slurm_test_job "experiment_alpha"
    just slurm_test_job "ml_training"
    just slurm_test_job "data_processing"
    @echo "Submitted 5 test jobs"

# Run LazySlurm against the test environment
run:
    cargo run

# Run LazySlurm with specific user
run_user user="slurm":
    SLURM_USER={{user}} cargo run

# Build and run tests
test:
    cargo test

# Clean up everything (containers, volumes, build artifacts)
clean:
    cd dev && docker-compose down -v
    docker system prune -f
    cargo clean

# Show logs from SLURM container
slurm_logs:
    docker logs -f lazyslurm_dev

# Check if SLURM commands are available from host
check_slurm:
    @echo "Checking if SLURM commands work from host..."
    @docker exec lazyslurm_dev which squeue || echo "❌ squeue not found"
    @docker exec lazyslurm_dev which scontrol || echo "❌ scontrol not found"  
    @docker exec lazyslurm_dev which scancel || echo "❌ scancel not found"
    @echo "✅ SLURM commands available in container"

# Show running jobs in a watch loop
watch_jobs:
    watch -n 2 "docker exec lazyslurm_dev squeue"