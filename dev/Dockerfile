FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive

# Install SLURM and dependencies
RUN apt-get update && apt-get install -y \
    slurm-wlm \
    slurm-wlm-basic-plugins \
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

# Create basic SLURM config
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
    echo "NodeName=slurmctld CPUs=5 State=UNKNOWN" >> /etc/slurm-llnl/slurm.conf && \
    echo "PartitionName=debug Nodes=slurmctld Default=YES MaxTime=INFINITE State=UP" >> /etc/slurm-llnl/slurm.conf

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Startup script
CMD bash -c " \
    service munge start && \
    slurmctld -D & \
    slurmd -D & \
    sleep 5 && \
    scontrol update NodeName=slurmctld State=IDLE && \
    echo 'SLURM ready! Your project is at /workspace' && \
    echo 'Run: cargo run' && \
    echo 'Submit test: sbatch --wrap=\"sleep 60\" --job-name=test' && \
    bash"