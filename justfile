# Development commands for LazySlurm

# list available recipes
[private]
default:
    @just --list --unsorted

# Start local SLURM cluster
slurm_up:
    @echo "Starting local SLURM cluster..."
    dev/start_local_slurm.sh

# Submit test jobs to local SLURM
slurm_populate:
    dev/submit_test_jobs.sh

# Check SLURM status
slurm_status:
    squeue -F dev/local_slurm.conf
    @echo ""
    sinfo -F dev/local_slurm.conf

# Run LazySlurm with local SLURM
run:
    SLURM_CONF_FILE=dev/local_slurm.conf cargo run

# Build and run tests
test:
    cargo test

# Clean up everything
clean:
    pkill -f slurmctld || true
    pkill -f slurmd || true
    rm -rf /tmp/slurm
    cargo clean

# Check if SLURM is installed
check_slurm:
    @echo "Checking if SLURM is installed..."
    @which squeue || echo "❌ squeue not found - run: brew install slurm"
    @which scontrol || echo "❌ scontrol not found - run: brew install slurm"  
    @which scancel || echo "❌ scancel not found - run: brew install slurm"
    @echo "✅ SLURM commands available"

# Show running jobs in a watch loop
watch_jobs:
    watch -n 2 "squeue -F dev/local_slurm.conf"