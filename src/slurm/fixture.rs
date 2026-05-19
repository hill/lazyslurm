use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::slurm::executor::SlurmExecutor;

/// A fake [`SlurmExecutor`] that reads canned outputs from a fixture directory.
///
/// Layout:
/// ```text
/// <fixture_dir>/
///   squeue.txt              # squeue output for any user/partition filter
///   scontrol/<job_id>.txt   # one file per job_id
/// ```
///
/// `scancel` calls are recorded in [`Self::cancelled`] for test assertions.
pub struct FixtureSlurm {
    pub fixture_dir: PathBuf,
    pub cancelled: Mutex<Vec<String>>,
}

impl FixtureSlurm {
    pub fn new(fixture_dir: impl Into<PathBuf>) -> Self {
        Self {
            fixture_dir: fixture_dir.into(),
            cancelled: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl SlurmExecutor for FixtureSlurm {
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
}
