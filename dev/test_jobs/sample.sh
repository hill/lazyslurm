#!/bin/bash
#SBATCH --job-name=sample_job
#SBATCH --time=00:02:00
#SBATCH --output=/home/slurm/data/job_%j.out

echo "Job $SLURM_JOB_ID running on $HOSTNAME at $(date)"
sleep 60
echo "Job completed at $(date)"
