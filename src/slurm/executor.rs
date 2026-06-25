use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait SlurmExecutor: Send + Sync {
    async fn squeue(&self, user: Option<&str>, partition: Option<&str>) -> Result<String>;
    async fn scontrol_show_job(&self, job_id: &str) -> Result<String>;
    async fn scancel(&self, job_id: &str) -> Result<()>;

    /// Per-node listing for the Nodes tab (`sinfo -N`).
    async fn sinfo_nodes(&self) -> Result<String>;

    /// Per-partition summary for the Partitions tab (`sinfo -s`).
    async fn sinfo_partitions(&self) -> Result<String>;

    /// Finished-job accounting (`sacct`). Errors if accounting isn't configured.
    async fn sacct(&self, user: Option<&str>) -> Result<String>;

    /// Full detail for one job, including step rows so MaxRSS is available.
    async fn sacct_job(&self, job_id: &str) -> Result<String>;
}
