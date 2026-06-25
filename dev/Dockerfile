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

# Basic SLURM config. The AccountingStorage* lines point the controller at
# slurmdbd so finished jobs land in the database that `sacct` reads, and
# JobAcctGather records per-job resource usage.
RUN echo "ClusterName=lazyslurm_dev" > /etc/slurm-llnl/slurm.conf && \
    echo "ControlMachine=slurmctld" >> /etc/slurm-llnl/slurm.conf && \
    echo "SlurmUser=root" >> /etc/slurm-llnl/slurm.conf && \
    echo "SlurmdUser=root" >> /etc/slurm-llnl/slurm.conf && \
    echo "SlurmctldPort=6817" >> /etc/slurm-llnl/slurm.conf && \
    echo "SlurmdPort=6818" >> /etc/slurm-llnl/slurm.conf && \
    echo "StateSaveLocation=/var/spool/slurm" >> /etc/slurm-llnl/slurm.conf && \
    echo "SlurmdSpoolDir=/var/spool/slurm" >> /etc/slurm-llnl/slurm.conf && \
    echo "SwitchType=switch/none" >> /etc/slurm-llnl/slurm.conf && \
    echo "ProctrackType=proctrack/pgid" >> /etc/slurm-llnl/slurm.conf && \
    echo "TaskPlugin=task/none" >> /etc/slurm-llnl/slurm.conf && \
    echo "ReturnToService=2" >> /etc/slurm-llnl/slurm.conf && \
    echo "MaxJobCount=10000" >> /etc/slurm-llnl/slurm.conf && \
    echo "AccountingStorageType=accounting_storage/slurmdbd" >> /etc/slurm-llnl/slurm.conf && \
    echo "AccountingStorageHost=localhost" >> /etc/slurm-llnl/slurm.conf && \
    echo "AccountingStoragePort=6819" >> /etc/slurm-llnl/slurm.conf && \
    echo "JobAcctGatherType=jobacct_gather/linux" >> /etc/slurm-llnl/slurm.conf && \
    echo "JobAcctGatherFrequency=30" >> /etc/slurm-llnl/slurm.conf && \
    echo "NodeName=slurmctld CPUs=5 State=UNKNOWN" >> /etc/slurm-llnl/slurm.conf && \
    echo "PartitionName=debug Nodes=slurmctld Default=YES MaxTime=INFINITE State=UP" >> /etc/slurm-llnl/slurm.conf

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

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

COPY dev/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

CMD ["/entrypoint.sh"]
