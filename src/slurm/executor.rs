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

    /// Finished-job accounting for the History tab (`sacct`). Errors when
    /// slurmdbd accounting isn't configured, which the caller surfaces as a
    /// "accounting unavailable" message rather than an empty list.
    async fn sacct(&self, user: Option<&str>) -> Result<String>;

    /// Full accounting detail for one job, including its step rows so MaxRSS is
    /// available. Backs the History detail view.
    async fn sacct_job(&self, job_id: &str) -> Result<String>;
}
