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

# QOS tiers and accounts, so the multifactor priority plugin has something to
# weigh. Different QOS priorities and account fairshare make the queue reorder
# instead of running first-in-first-out. All idempotent (|| true) so a restart
# on an existing accounting volume is harmless.
sacctmgr -i add qos high set priority=1000 >/dev/null 2>&1 || true
sacctmgr -i add qos low set priority=10 >/dev/null 2>&1 || true
sacctmgr -i modify qos normal set priority=100 >/dev/null 2>&1 || true

sacctmgr -i add account ml-lab Description="ML lab" fairshare=60 >/dev/null 2>&1 || true
sacctmgr -i add account physics Description="Physics group" fairshare=30 >/dev/null 2>&1 || true
sacctmgr -i add account ops Description="Ops" fairshare=10 >/dev/null 2>&1 || true

# Each user lives in an account and may pick any of the three QOS (default
# normal). root keeps all QOS too, otherwise enforce=qos could block it.
sacctmgr -i modify user root set qos=normal,high,low >/dev/null 2>&1 || true
for entry in "alice:ml-lab" "bob:ml-lab" "carol:physics" "dave:ops"; do
    user="${entry%%:*}"
    acct="${entry##*:}"
    sacctmgr -i add user "$user" Account="$acct" qos=normal,high,low DefaultQOS=normal \
        >/dev/null 2>&1 || true
done

# Dummy GPU devices for gres.conf to bind to. Slurm 19.05 drains the node if
# the reported GRES count is lower than configured, so these must exist before
# slurmd starts. The major/minor numbers are arbitrary (no real driver).
mknod /dev/fakegpu0 c 195 0 2>/dev/null || true
mknod /dev/fakegpu1 c 195 1 2>/dev/null || true

# Normalise the node line to the VM's actual resources. CPUs must match nproc
# (otherwise "low cpu count" drains the node) and RealMemory must sit under what
# the kernel reports (otherwise "Low RealMemory" drains it). Leave headroom.
CPUS=$(nproc)
MEM_MB=$(( $(awk '/MemTotal/{print $2}' /proc/meminfo) / 1024 - 512 ))
[ "$MEM_MB" -lt 2048 ] && MEM_MB=2048
sed -i "s/^NodeName=.*/NodeName=slurmctld CPUs=${CPUS} RealMemory=${MEM_MB} Gres=gpu:2 State=UNKNOWN/" /etc/slurm-llnl/slurm.conf

slurmctld -D &
slurmd -D &
sleep 5
# RESUME (not IDLE) so a drain left over from a previous hard kill clears too.
# A "Kill task failed" drain persists in the saved state across restarts and
# would otherwise leave every new job stuck pending.
scontrol update NodeName=slurmctld State=RESUME || true

echo 'SLURM ready (accounting + QOS + fake GPUs enabled)! Your project is at /workspace'
echo 'Run: cargo run'
echo 'Partitions: debug cpu gpu bigmem long  |  Users: alice bob carol dave  |  QOS: high normal low'
echo 'Fill the queue: just slurm_populate     |     History tab is backed by: sacct'

# Keep PID 1 alive so the container never exits on its own when detached. Get a
# shell with `just slurm_shell` (docker exec), not the main process.
exec sleep infinity
