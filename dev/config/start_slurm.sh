#!/bin/bash

# Start munge
service munge start

# Start SLURM services
slurmctld -D &
slurmd -D &

# Wait for services to start
sleep 5

# Configure node state
scontrol update NodeName=slurmctld State=RESUME

# Create sample job script
cat > /home/slurm/jobs/sample.sh << 'SCRIPT'
#!/bin/bash
#SBATCH --job-name=sample_job
#SBATCH --time=00:02:00
#SBATCH --output=/home/slurm/data/job_%j.out

echo "Job $SLURM_JOB_ID running on $HOSTNAME at $(date)"
sleep 60
echo "Job completed at $(date)"
SCRIPT

chmod +x /home/slurm/jobs/sample.sh

# Keep container running
wait