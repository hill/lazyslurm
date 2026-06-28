FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive

# Install SLURM, the accounting daemon (slurmdbd), and MariaDB for it to use.
RUN apt-get update && apt-get install -y \
    slurm-wlm \
    slurm-wlm-basic-plugins \
    slurmdbd \
    mariadb-server \
    munge \
    curl \
    build-essential \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create necessary directories
RUN mkdir -p /var/spool/slurm /var/log/slurm /var/run/slurm /etc/slurm-llnl
RUN chown slurm:slurm /var/spool/slurm /var/log/slurm /var/run/slurm

# Create munge key
RUN /usr/sbin/create-munge-key
RUN chown munge:munge /etc/munge/munge.key
RUN chmod 400 /etc/munge/munge.key

# SLURM config. select/cons_res with CR_CPU_Memory makes CPUs and memory
# consumable, so jobs queue on one fat node instead of running one at a time
# (Ubuntu 20.04 ships Slurm 19.05, which predates cons_tres). Multifactor
# priority lets fairshare and QOS reorder the queue rather than plain FIFO. The
# AccountingStorage* lines point the controller at slurmdbd so sacct/history
# works, and Gres=gpu:2 gives fake count-only GPUs (no device files needed
# under TaskPlugin=task/none). CPUs/RealMemory/Gres on the NodeName line are
# normalised at boot in entrypoint.sh.
RUN printf '%s\n' \
    "ClusterName=lazyslurm_dev" \
    "ControlMachine=slurmctld" \
    "SlurmUser=root" \
    "SlurmdUser=root" \
    "SlurmctldPort=6817" \
    "SlurmdPort=6818" \
    "StateSaveLocation=/var/spool/slurm" \
    "SlurmdSpoolDir=/var/spool/slurm" \
    "SwitchType=switch/none" \
    "ProctrackType=proctrack/linuxproc" \
    "TaskPlugin=task/none" \
    "ReturnToService=2" \
    "MaxJobCount=10000" \
    "UnkillableStepTimeout=120" \
    "SelectType=select/cons_res" \
    "SelectTypeParameters=CR_CPU_Memory" \
    "DefMemPerCPU=512" \
    "SchedulerType=sched/backfill" \
    "GresTypes=gpu" \
    "PriorityType=priority/multifactor" \
    "PriorityDecayHalfLife=1-0" \
    "PriorityWeightFairshare=100000" \
    "PriorityWeightQOS=10000" \
    "PriorityWeightAge=1000" \
    "PriorityWeightPartition=1000" \
    "PriorityWeightJobSize=1000" \
    "AccountingStorageType=accounting_storage/slurmdbd" \
    "AccountingStorageHost=localhost" \
    "AccountingStoragePort=6819" \
    "AccountingStorageTRES=gres/gpu" \
    "AccountingStorageEnforce=qos,limits" \
    "JobAcctGatherType=jobacct_gather/linux" \
    "JobAcctGatherFrequency=30" \
    "NodeName=slurmctld CPUs=8 RealMemory=16000 Gres=gpu:2 State=UNKNOWN" \
    "PartitionName=debug Nodes=slurmctld Default=YES MaxTime=00:30:00 State=UP" \
    "PartitionName=cpu Nodes=slurmctld MaxTime=2-00:00:00 State=UP" \
    "PartitionName=gpu Nodes=slurmctld MaxTime=1-00:00:00 State=UP" \
    "PartitionName=bigmem Nodes=slurmctld MaxTime=2-00:00:00 State=UP" \
    "PartitionName=long Nodes=slurmctld MaxTime=7-00:00:00 State=UP" \
    > /etc/slurm-llnl/slurm.conf

# gres.conf tells slurmd about the node's GPUs. Slurm 19.05 validates the count
# against real devices, so File must point at something: entrypoint.sh mknod's
# two dummy character devices (/dev/fakegpu0,1) before slurmd starts. With
# TaskPlugin=task/none there is no device-cgroup isolation, so dummies are
# enough for the scheduler to track and hand out --gres=gpu:N.
RUN echo "NodeName=slurmctld Name=gpu File=/dev/fakegpu[0-1]" > /etc/slurm-llnl/gres.conf

# slurmdbd config. Must be mode 600 and owned by SlurmUser or slurmdbd refuses
# to start.
RUN printf '%s\n' \
    "AuthType=auth/munge" \
    "DbdHost=localhost" \
    "DbdPort=6819" \
    "SlurmUser=root" \
    "DebugLevel=info" \
    "LogFile=/var/log/slurm/slurmdbd.log" \
    "PidFile=/var/run/slurm/slurmdbd.pid" \
    "StorageType=accounting_storage/mysql" \
    "StorageHost=localhost" \
    "StoragePort=3306" \
    "StorageUser=slurm" \
    "StoragePass=slurmpass" \
    "StorageLoc=slurm_acct_db" \
    > /etc/slurm-llnl/slurmdbd.conf && \
    chmod 600 /etc/slurm-llnl/slurmdbd.conf && \
    chown root:root /etc/slurm-llnl/slurmdbd.conf

# Let MariaDB bind locally; defaults are fine for a single-container dev box.
RUN sed -i 's/^bind-address.*/bind-address = 127.0.0.1/' /etc/mysql/mariadb.conf.d/50-server.cnf || true

# Cluster users. Their Slurm accounts and associations are created at boot in
# entrypoint.sh (those need slurmdbd running); these are just the unix accounts
# the job zoo submits as via `runuser -u <name>`.
RUN for u in alice bob carol dave; do useradd -m -s /bin/bash "$u"; done

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

COPY dev/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

CMD ["/entrypoint.sh"]
