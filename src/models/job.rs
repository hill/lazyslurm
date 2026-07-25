use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Cancelled,
    Failed,
    Timeout,
    NodeFail,
    Preempted,
    Unknown(String),
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobState::Pending => write!(f, "PD"),
            JobState::Running => write!(f, "R"),
            JobState::Completed => write!(f, "CD"),
            JobState::Cancelled => write!(f, "CA"),
            JobState::Failed => write!(f, "F"),
            JobState::Timeout => write!(f, "TO"),
            JobState::NodeFail => write!(f, "NF"),
            JobState::Preempted => write!(f, "PR"),
            JobState::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl From<&str> for JobState {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "PENDING" | "PD" => JobState::Pending,
            "RUNNING" | "R" => JobState::Running,
            "COMPLETED" | "CD" | "COMPLETING" => JobState::Completed,
            "CANCELLED" | "CA" => JobState::Cancelled,
            "FAILED" | "F" => JobState::Failed,
            "TIMEOUT" | "TO" => JobState::Timeout,
            "NODE_FAIL" | "NF" => JobState::NodeFail,
            "PREEMPTED" | "PR" => JobState::Preempted,
            _ => JobState::Unknown(s.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub array_job_id: Option<String>,
    pub array_task_id: Option<u32>,
    pub name: String,
    pub user: String,
    pub partition: String,
    pub state: JobState,
    pub time_limit: Option<String>,
    pub time_used: Option<String>,
    pub submit_time: Option<DateTime<Utc>>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub nodes: Option<u32>,
    pub node_list: Option<String>,
    pub cpus: Option<u32>,
    pub memory: Option<String>,
    pub working_dir: Option<String>,
    pub std_out: Option<String>,
    pub std_err: Option<String>,
    pub exit_code: Option<i32>,
    pub reason: Option<String>,
}

impl Job {
    pub fn new(job_id: String, name: String, user: String, state: JobState) -> Self {
        Self {
            job_id,
            array_job_id: None,
            array_task_id: None,
            name,
            user,
            partition: "".to_string(),
            state,
            time_limit: None,
            time_used: None,
            submit_time: None,
            start_time: None,
            end_time: None,
            nodes: None,
            node_list: None,
            cpus: None,
            memory: None,
            working_dir: None,
            std_out: None,
            std_err: None,
            exit_code: None,
            reason: None,
        }
    }

    pub fn is_array_job(&self) -> bool {
        self.array_job_id.is_some()
    }

    pub fn display_id(&self) -> String {
        match (&self.array_job_id, &self.array_task_id) {
            (Some(array_id), Some(task_id)) => format!("{}_{}", array_id, task_id),
            _ => self.job_id.clone(),
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, JobState::Running)
    }

    pub fn is_completed(&self) -> bool {
        matches!(
            self.state,
            JobState::Completed | JobState::Failed | JobState::Cancelled | JobState::Timeout
        )
    }

    pub fn elapsed_secs(&self) -> Option<u64> {
        self.time_used.as_deref().and_then(parse_duration_secs)
    }

    pub fn limit_secs(&self) -> Option<u64> {
        self.time_limit.as_deref().and_then(parse_duration_secs)
    }

    /// Fraction of the wall-clock time limit consumed, clamped to `0.0..=1.0`.
    /// `None` when either value is missing or the limit is unbounded.
    pub fn walltime_fraction(&self) -> Option<f32> {
        let (elapsed, limit) = (self.elapsed_secs()?, self.limit_secs()?);
        if limit == 0 {
            return None;
        }
        Some((elapsed as f32 / limit as f32).clamp(0.0, 1.0))
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        match (&self.start_time, &self.end_time) {
            (Some(start), Some(end)) => Some(*end - *start),
            // Pending jobs carry SLURM's estimated (future) start time, so
            // only count elapsed time once the job has actually started.
            (Some(start), None) => {
                let elapsed = Utc::now() - *start;
                (elapsed >= chrono::Duration::zero()).then_some(elapsed)
            }
            _ => None,
        }
    }
}

/// Parse a SLURM duration (`D-HH:MM:SS`, `HH:MM:SS`, `MM:SS`, or `SS`) to
/// seconds. Sentinels like `UNLIMITED` or `N/A` return `None`.
pub fn parse_duration_secs(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if !s.as_bytes()[0].is_ascii_digit() {
        // UNLIMITED, N/A, NOT_SET, INVALID, Partition_Limit, ...
        return None;
    }

    let (days, hms) = match s.split_once('-') {
        Some((d, rest)) => (d.parse::<u64>().ok()?, rest),
        None => (0, s),
    };

    let parts: Vec<&str> = hms.split(':').collect();
    let (h, m, sec): (u64, u64, u64) = match parts.as_slice() {
        [h, m, s] => (h.parse().ok()?, m.parse().ok()?, s.parse().ok()?),
        [m, s] => (0, m.parse().ok()?, s.parse().ok()?),
        [s] => (0, 0, s.parse().ok()?),
        _ => return None,
    };

    Some(days * 86_400 + h * 3_600 + m * 60 + sec)
}

/// Number of array tasks a squeue job id stands for. A concrete task (`123_4`)
/// is one; a bracketed spec (`123_[2-4]`, `123_[1,3-5%2]`) counts its members.
pub fn array_task_count(job_id: &str) -> u64 {
    let Some((_, spec)) = job_id.split_once('_') else {
        return 1;
    };
    let spec = spec.trim();
    if !spec.starts_with('[') {
        return 1;
    }

    // Strip the brackets and any "%N" concurrency cap, then count the members.
    let inner = spec.trim_start_matches('[').trim_end_matches(']');
    let inner = inner.split('%').next().unwrap_or(inner);

    let mut count = 0u64;
    for part in inner.split(',') {
        match part.split_once('-') {
            Some((a, b)) => {
                if let (Ok(a), Ok(b)) = (a.trim().parse::<u64>(), b.trim().parse::<u64>()) {
                    count += b.saturating_sub(a) + 1;
                }
            }
            None => {
                if part.trim().parse::<u64>().is_ok() {
                    count += 1;
                }
            }
        }
    }

    count.max(1)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobList {
    pub jobs: Vec<Job>,
    pub last_updated: DateTime<Utc>,
}

impl JobList {
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            last_updated: Utc::now(),
        }
    }

    pub fn update(&mut self, jobs: Vec<Job>) {
        self.jobs = jobs;
        self.last_updated = Utc::now();
    }

    pub fn running_jobs(&self) -> Vec<&Job> {
        self.jobs.iter().filter(|job| job.is_running()).collect()
    }

    pub fn pending_jobs(&self) -> Vec<&Job> {
        self.jobs
            .iter()
            .filter(|job| matches!(job.state, JobState::Pending))
            .collect()
    }

    pub fn completed_jobs(&self) -> Vec<&Job> {
        self.jobs.iter().filter(|job| job.is_completed()).collect()
    }
}

impl Default for JobList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_job_with_estimated_future_start_has_no_duration() {
        let mut job = Job::new("1".into(), "j".into(), "u".into(), JobState::Pending);
        job.start_time = Some(Utc::now() + chrono::Duration::hours(1));
        assert_eq!(job.duration(), None);
    }

    #[test]
    fn running_job_reports_elapsed_duration() {
        let mut job = Job::new("1".into(), "j".into(), "u".into(), JobState::Running);
        job.start_time = Some(Utc::now() - chrono::Duration::seconds(30));
        let d = job.duration().expect("should have a duration");
        assert!(d.num_seconds() >= 30 && d.num_seconds() < 120);
    }

    #[test]
    fn parses_slurm_durations() {
        assert_eq!(parse_duration_secs("13:08"), Some(13 * 60 + 8));
        assert_eq!(parse_duration_secs("1:42:09"), Some(3600 + 42 * 60 + 9));
        assert_eq!(parse_duration_secs("1-00:00:00"), Some(86_400));
        assert_eq!(parse_duration_secs("45"), Some(45));
        assert_eq!(parse_duration_secs("UNLIMITED"), None);
        assert_eq!(parse_duration_secs("N/A"), None);
        assert_eq!(parse_duration_secs(""), None);
    }

    #[test]
    fn walltime_fraction_is_elapsed_over_limit() {
        let mut job = Job::new("1".into(), "j".into(), "u".into(), JobState::Running);
        job.time_used = Some("12:00:00".into());
        job.time_limit = Some("24:00:00".into());
        assert_eq!(job.walltime_fraction(), Some(0.5));

        // Over-run clamps to 1.0 rather than exceeding the bar.
        job.time_used = Some("2-00:00:00".into());
        assert_eq!(job.walltime_fraction(), Some(1.0));

        job.time_limit = Some("UNLIMITED".into());
        assert_eq!(job.walltime_fraction(), None);
    }

    #[test]
    fn counts_array_tasks() {
        assert_eq!(array_task_count("48210_5"), 1);
        assert_eq!(array_task_count("48210_[2-4]"), 3);
        assert_eq!(array_task_count("48210_[2-4%2]"), 3);
        assert_eq!(array_task_count("48210_[1,3-5]"), 4);
        assert_eq!(array_task_count("48210"), 1);
    }
}
