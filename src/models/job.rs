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

    pub fn duration(&self) -> Option<chrono::Duration> {
        match (&self.start_time, &self.end_time) {
            (Some(start), Some(end)) => Some(*end - *start),
            (Some(start), None) => Some(Utc::now() - *start),
            _ => None,
        }
    }
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
