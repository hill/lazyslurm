#!/bin/bash
#SBATCH --job-name=array_test
#SBATCH --output=logs/array_%A_%a.out
#SBATCH --error=logs/array_%A_%a.err
#SBATCH --array=1-5
#SBATCH --time=00:03:00
#SBATCH --partition=debug

echo "Array job $SLURM_ARRAY_JOB_ID, task $SLURM_ARRAY_TASK_ID"
echo "Running on node: $SLURMD_NODENAME"

# Simulate different work for each array task
case $SLURM_ARRAY_TASK_ID in
    1) echo "Processing dataset alpha"; sleep 60 ;;
    2) echo "Processing dataset beta"; sleep 90 ;;
    3) echo "Processing dataset gamma"; sleep 45 ;;
    4) echo "Processing dataset delta"; sleep 75 ;;
    5) echo "Processing dataset epsilon"; sleep 30 ;;
esac

echo "Array task $SLURM_ARRAY_TASK_ID completed at $(date)"