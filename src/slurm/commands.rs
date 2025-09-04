use anyhow::{Context, Result};
use std::process::Command;
use tokio::process::Command as TokioCommand;

pub struct SlurmCommands;

impl SlurmCommands {
    pub async fn squeue(user: Option<&str>) -> Result<String> {
        let mut cmd = TokioCommand::new("squeue");
        
        if let Some(user) = user {
            cmd.arg("-u").arg(user);
        }
        
        cmd.arg("--format=%i,%j,%u,%t,%M,%N,%P");

        let output = cmd.output().await.context("Failed to execute squeue")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("squeue failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn scontrol_show_job(job_id: &str) -> Result<String> {
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

    pub async fn scancel(job_id: &str) -> Result<()> {
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

    pub fn check_slurm_available() -> bool {
        Command::new("which")
            .arg("squeue")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}