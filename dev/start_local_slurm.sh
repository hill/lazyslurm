#!/bin/bash

set -e

SLURM_DIR="/tmp/slurm"
CONFIG_FILE="$PWD/dev/local_slurm.conf"
LOG_DIR="$PWD/dev/test_data"

echo "Setting up local SLURM cluster..."

# Create directories
mkdir -p "$SLURM_DIR"/{state,spool} "$LOG_DIR"

# Start SLURM daemons
echo "Starting slurmctld..."
slurmctld -D -f "$CONFIG_FILE" &
SLURMCTLD_PID=$!

sleep 2

echo "Starting slurmd..."
slurmd -D -f "$CONFIG_FILE" &
SLURMD_PID=$!

sleep 2

# Configure node
echo "Configuring node..."
scontrol -F "$CONFIG_FILE" update NodeName=localhost State=RESUME

echo "SLURM cluster ready!"
echo "slurmctld PID: $SLURMCTLD_PID"
echo "slurmd PID: $SLURMD_PID"

# Keep script running
trap "echo 'Stopping SLURM...'; kill $SLURMCTLD_PID $SLURMD_PID; exit" SIGINT SIGTERM
wait