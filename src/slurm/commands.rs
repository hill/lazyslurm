use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Command;
use tokio::process::Command as TokioCommand;

use crate::slurm::executor::SlurmExecutor;

pub struct SlurmProcess;

#[async_trait]
impl SlurmExecutor for SlurmProcess {
    async fn squeue(&self, user: Option<&str>, partition: Option<&str>) -> Result<String> {
        let mut cmd = TokioCommand::new("squeue");

        if let Some(user) = user {
            cmd.arg("-u").arg(user);
        }

        if let Some(partition) = partition {
            cmd.arg("-p").arg(partition);
        }

        cmd.arg("--format=%i,%j,%u,%t,%M,%N,%P");

        let output = cmd.output().await.context("Failed to execute squeue")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("squeue failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn scontrol_show_job(&self, job_id: &str) -> Result<String> {
        let output = TokioCommand::new("scontrol")
            .arg("show")
            .arg("job")
            .arg(job_id)
            .output()
            .await
            .context("Failed to execute scontrol")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("scontrol show job failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn scancel(&self, job_id: &str) -> Result<()> {
        let output = TokioCommand::new("scancel")
            .arg(job_id)
            .output()
            .await
            .context("Failed to execute scancel")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("scancel failed: {}", stderr);
        }

        Ok(())
    }

    async fn sinfo_nodes(&self) -> Result<String> {
        // host|state|cpus A/I/O/T|memory|free mem|gres|partition
        let output = TokioCommand::new("sinfo")
            .args(["-h", "-N", "-o", "%n|%T|%C|%m|%e|%G|%P"])
            .output()
            .await
            .context("Failed to execute sinfo")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("sinfo failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn sinfo_partitions(&self) -> Result<String> {
        // partition|availability|nodes A/I/O/T|time limit
        let output = TokioCommand::new("sinfo")
            .args(["-h", "-s", "-o", "%P|%a|%F|%l"])
            .output()
            .await
            .context("Failed to execute sinfo")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("sinfo failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn sacct(&self, user: Option<&str>) -> Result<String> {
        let mut cmd = TokioCommand::new("sacct");
        cmd.args([
            "-X",
            "-n",
            "-P",
            "--format=JobID,JobName,State,ExitCode,Elapsed,Start,End",
        ]);

        if let Some(user) = user {
            cmd.arg("-u").arg(user);
        }

        let output = cmd.output().await.context("Failed to execute sacct")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("sacct failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn sacct_job(&self, job_id: &str) -> Result<String> {
        // No -X: MaxRSS only appears on the per-step rows.
        let output = TokioCommand::new("sacct")
            .args([
                "-j",
                job_id,
                "-n",
                "-P",
                "--format=JobID,JobName,User,Account,Partition,NodeList,AllocCPUS,ReqMem,MaxRSS,TotalCPU,State,ExitCode,Submit,Start,End,Elapsed,WorkDir",
            ])
            .output()
            .await
            .context("Failed to execute sacct")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("sacct failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

pub fn check_slurm_available() -> bool {
    Command::new("which")
        .arg("squeue")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
