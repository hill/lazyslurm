#!/usr/bin/env bash
# Boot order matters: munge (auth) -> MariaDB -> slurmdbd + cluster registered
# -> slurmctld/slurmd. slurmctld must not start until the accounting database
# knows about this cluster, otherwise finished jobs never get recorded.
set -e

service munge start

# Initialise the MariaDB data directory if a fresh volume shadowed the one the
# package built. The service is named `mysql` on Ubuntu even with MariaDB.
if [ ! -d /var/lib/mysql/mysql ]; then
    mysql_install_db --user=mysql --datadir=/var/lib/mysql >/dev/null 2>&1 || true
fi
service mysql start

# Accounting database and the user slurmdbd connects as. Idempotent so a
# restart on an existing volume is harmless.
mysql <<'SQL'
CREATE DATABASE IF NOT EXISTS slurm_acct_db;
CREATE USER IF NOT EXISTS 'slurm'@'localhost' IDENTIFIED BY 'slurmpass';
CREATE USER IF NOT EXISTS 'slurm'@'%' IDENTIFIED BY 'slurmpass';
GRANT ALL PRIVILEGES ON slurm_acct_db.* TO 'slurm'@'localhost';
GRANT ALL PRIVILEGES ON slurm_acct_db.* TO 'slurm'@'%';
FLUSH PRIVILEGES;
SQL

slurmdbd -D &

# Wait for slurmdbd to accept connections before registering the cluster.
for _ in $(seq 1 30); do
    if sacctmgr -i show cluster >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
sacctmgr -i add cluster lazyslurm_dev >/dev/null 2>&1 || true

# Match the configured CPU count to whatever the Docker VM actually exposes,
# otherwise slurmctld rejects the node ("low cpu count") and jobs never run.
CPUS=$(nproc)
sed -i "s/^NodeName=.*/NodeName=slurmctld CPUs=${CPUS} State=UNKNOWN/" /etc/slurm-llnl/slurm.conf

slurmctld -D &
slurmd -D &
sleep 5
# RESUME (not IDLE) so a drain left over from a previous hard kill clears too.
# A "Kill task failed" drain persists in the saved state across restarts and
# would otherwise leave every new job stuck pending.
scontrol update NodeName=slurmctld State=RESUME || true

echo 'SLURM ready (accounting enabled)! Your project is at /workspace'
echo 'Run: cargo run'
echo 'Submit test jobs: just slurm_populate    |    History tab is backed by: sacct'

# Keep PID 1 alive so the container never exits on its own when detached. Get a
# shell with `just slurm_shell` (docker exec), not the main process.
exec sleep infinity
