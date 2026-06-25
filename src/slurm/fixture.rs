use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::slurm::executor::SlurmExecutor;

/// A fake [`SlurmExecutor`] reading canned outputs from a fixture directory
/// (`squeue.txt`, `scontrol/<id>.txt`, etc). Records `scancel` ids for tests.
pub struct SlurmFixture {
    pub fixture_dir: PathBuf,
    pub cancelled: Mutex<Vec<String>>,
}

impl SlurmFixture {
    pub fn new(fixture_dir: impl Into<PathBuf>) -> Self {
        Self {
            fixture_dir: fixture_dir.into(),
            cancelled: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl SlurmExecutor for SlurmFixture {
    async fn squeue(&self, _user: Option<&str>, _partition: Option<&str>) -> Result<String> {
        let path = self.fixture_dir.join("squeue.txt");
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read fixture: {}", path.display()))
    }

    async fn scontrol_show_job(&self, job_id: &str) -> Result<String> {
        let path = self
            .fixture_dir
            .join("scontrol")
            .join(format!("{}.txt", job_id));
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read fixture: {}", path.display()))
    }

    async fn scancel(&self, job_id: &str) -> Result<()> {
        self.cancelled.lock().unwrap().push(job_id.to_string());
        Ok(())
    }

    async fn sinfo_nodes(&self) -> Result<String> {
        let path = self.fixture_dir.join("sinfo_nodes.txt");
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read fixture: {}", path.display()))
    }

    async fn sinfo_partitions(&self) -> Result<String> {
        let path = self.fixture_dir.join("sinfo_partitions.txt");
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read fixture: {}", path.display()))
    }

    async fn sacct(&self, _user: Option<&str>) -> Result<String> {
        let path = self.fixture_dir.join("sacct.txt");
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read fixture: {}", path.display()))
    }

    async fn sacct_job(&self, job_id: &str) -> Result<String> {
        let path = self
            .fixture_dir
            .join("sacct_job")
            .join(format!("{}.txt", job_id));
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read fixture: {}", path.display()))
    }
}
