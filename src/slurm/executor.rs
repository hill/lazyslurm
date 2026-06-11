use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait SlurmExecutor: Send + Sync {
    async fn squeue(&self, user: Option<&str>, partition: Option<&str>) -> Result<String>;
    async fn scontrol_show_job(&self, job_id: &str) -> Result<String>;
    async fn scancel(&self, job_id: &str) -> Result<()>;
}
