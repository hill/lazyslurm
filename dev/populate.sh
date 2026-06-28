#!/usr/bin/env bash
# Submit a "job zoo" into the dev cluster: a deliberate mix of users, accounts,
# partitions, QOS, GPU requests, an array, a dependency chain, and a couple of
# fast finishers so every tab (Jobs, Nodes, Partitions, History) shows variety.
#
# Run from the host with `just slurm_populate`, or inside the container with
# `bash /workspace/dev/populate.sh`. Jobs are submitted as the cluster users
# (alice/bob/carol/dave) via runuser so the USER column is real.
#
# There are only 2 fake GPUs, so the GPU requests intentionally exceed supply:
# some land RUNNING, the rest sit PENDING with Reason=Resources.
set -u

# A short busy-loop the running jobs execute so they stay visible for a while.
loop() { echo "for i in \$(seq 1 $1); do echo \"$2 step \$i/$1\"; sleep $3; done"; }

submit() {
    # submit <user> <sbatch args...> -- <shell command>
    local user="$1"; shift
    runuser -u "$user" -- sbatch \
        --output=/tmp/slurm-%j.out --error=/tmp/slurm-%j.err "$@"
}

echo "Submitting job zoo..."

# --- GPU partition: 3 GPU jobs + a 4-task array, but only 2 GPUs exist ---
submit alice -p gpu --qos=high   -J train_resnet --gres=gpu:1 -n1 --mem=2G \
    --wrap="$(loop 600 'train_resnet' 2)"
submit bob   -p gpu --qos=normal -J train_bert   --gres=gpu:1 -n1 --mem=2G \
    --wrap="$(loop 600 'train_bert' 2)"
submit carol -p gpu --qos=normal -J sweep --array=1-4 --gres=gpu:1 --mem=1G \
    --wrap="$(loop 600 'sweep' 2)"

# --- CPU partition: a normal job and a low-priority one behind it ---
submit alice -p cpu --qos=normal -J preprocess   -n4 --mem=2G \
    --wrap="$(loop 600 'preprocess' 2)"
submit carol -p cpu --qos=low    -J low_prio_sim -n4 --mem=2G \
    --wrap="$(loop 600 'low_prio_sim' 2)"

# --- bigmem partition, low QOS: competes for the node and tends to wait ---
# (a single dev node can't hold a job too big to ever run; such a request is
# rejected at submit rather than queued, so we keep this one schedulable.)
submit bob -p bigmem --qos=low -J big_matrix -n2 --mem=2G \
    --wrap="$(loop 600 'big_matrix' 2)"

# --- A dependency chain: stage2 waits for stage1 (shows Reason=Dependency) ---
stage1=$(runuser -u alice -- sbatch --parsable \
    --output=/tmp/slurm-%j.out --error=/tmp/slurm-%j.err \
    -p cpu --qos=normal -J stage1 --mem=1G --wrap="sleep 45")
runuser -u alice -- sbatch \
    --output=/tmp/slurm-%j.out --error=/tmp/slurm-%j.err \
    -p cpu --qos=normal -J stage2 --mem=1G --dependency="afterok:${stage1}" \
    --wrap="echo 'stage2 running after stage1'"

# --- Fast finishers, so the History tab fills quickly via sacct ---
submit dave -p debug --qos=normal -J smoke_test --mem=512M \
    --wrap="$(loop 10 'smoke_test' 1)"
submit dave -p debug --qos=normal -J flaky --mem=512M \
    --wrap="echo 'oops'; exit 1"

echo "Job zoo submitted. Watch it with: just slurm_status   (or the Jobs tab)"
