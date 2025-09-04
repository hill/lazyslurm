FROM ubuntu:20.04

# Prevent interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies
RUN apt-get update && apt-get install -y \
    slurm-wlm \
    slurm-wlm-basic-plugins \
    munge \
    sudo \
    vim \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Configure existing slurm user
RUN usermod -aG sudo slurm
RUN mkdir -p /home/slurm && chown slurm:slurm /home/slurm

# Create necessary directories
RUN mkdir -p /var/spool/slurm \
    /var/log/slurm \
    /var/run/slurm \
    /home/slurm/jobs \
    /home/slurm/data

# Set ownership
RUN chown -R slurm:slurm /var/spool/slurm /var/log/slurm /var/run/slurm /home/slurm

# Create munge key
RUN /usr/sbin/create-munge-key
RUN chown munge:munge /etc/munge/munge.key
RUN chmod 400 /etc/munge/munge.key

# Copy configuration files
COPY dev/config/slurm.conf /etc/slurm/slurm.conf
COPY dev/config/start_slurm.sh /start_slurm.sh
RUN chmod +x /start_slurm.sh

# Switch to slurm user
USER slurm
WORKDIR /home/slurm

EXPOSE 6817 6818

CMD ["/start_slurm.sh"]