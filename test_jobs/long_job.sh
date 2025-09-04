#!/bin/bash
#SBATCH --job-name=long_running_experiment
#SBATCH --output=logs/experiment_%j.log
#SBATCH --error=logs/experiment_%j.err
#SBATCH --time=00:10:00
#SBATCH --partition=debug
#SBATCH --nodes=1
#SBATCH --ntasks=1

echo "Starting long-running experiment at $(date)"
echo "This simulates your typical ML/experiment workflow"

# Simulate experiment phases
phases=("data_loading" "preprocessing" "training" "validation" "cleanup")

for phase in "${phases[@]}"; do
    echo "[$phase] Starting phase: $phase"
    
    # Random work duration
    duration=$((30 + RANDOM % 60))
    
    for ((i=1; i<=duration; i++)); do
        if [ $((i % 10)) -eq 0 ]; then
            echo "[$phase] Progress: $i/$duration steps completed"
        fi
        sleep 1
    done
    
    echo "[$phase] Completed phase: $phase"
done

echo "Experiment completed successfully at $(date)"