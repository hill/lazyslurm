#!/bin/bash

CONFIG_FILE="$PWD/dev/local_slurm.conf"
LOG_DIR="$PWD/dev/test_data"

echo "Submitting test jobs..."

# Create log directory
mkdir -p "$LOG_DIR"

# Submit various test jobs
echo "Submitting quick jobs..."
sbatch -F "$CONFIG_FILE" --job-name=quick_job --output="$LOG_DIR/quick_%j.out" --wrap="echo 'Quick job starting'; sleep 30; echo 'Quick job done'"

sbatch -F "$CONFIG_FILE" --job-name=medium_job --output="$LOG_DIR/medium_%j.out" --wrap="echo 'Medium job starting'; sleep 120; echo 'Medium job done'"

echo "Submitting long-running jobs..."
for i in {1..3}; do
    duration=$((60 + i * 30))
    sbatch -F "$CONFIG_FILE" --job-name="experiment_$i" --output="$LOG_DIR/exp_${i}_%j.out" \
        --wrap="echo 'Experiment $i starting'; for j in {1..10}; do echo 'Step \$j'; sleep $((duration/10)); done; echo 'Experiment $i complete'"
done

echo "Submitting array job..."
sbatch -F "$CONFIG_FILE" --job-name=array_test --array=1-5 --output="$LOG_DIR/array_%A_%a.out" \
    --wrap="echo 'Array task \$SLURM_ARRAY_TASK_ID starting'; sleep \$((30 + SLURM_ARRAY_TASK_ID * 10)); echo 'Array task \$SLURM_ARRAY_TASK_ID done'"

echo "Test jobs submitted! Check with: squeue -F $CONFIG_FILE"