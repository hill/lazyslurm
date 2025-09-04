#!/bin/bash
#SBATCH --job-name=sample_job
#SBATCH --output=logs/sample_%j.out
#SBATCH --error=logs/sample_%j.err
#SBATCH --time=00:05:00
#SBATCH --partition=debug
#SBATCH --nodes=1
#SBATCH --ntasks=1

echo "Starting sample job at $(date)"
echo "Job ID: $SLURM_JOB_ID"
echo "Job Name: $SLURM_JOB_NAME"
echo "Node: $SLURMD_NODENAME"

# Simulate some work
for i in {1..30}; do
    echo "Processing step $i/30..."
    sleep 2
done

echo "Job completed successfully at $(date)"