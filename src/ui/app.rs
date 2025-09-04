use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::models::{Job, JobList};
use crate::slurm::{SlurmCommands, SlurmParser};

#[derive(Debug, Clone)]
pub enum AppEvent {
    Refresh,
    JobSelected(String),
    Quit,
}

#[derive(Debug)]
pub struct App {
    pub job_list: JobList,
    pub selected_job_index: usize,
    pub selected_job: Option<Job>,
    pub current_user: Option<String>,
    pub last_refresh: Instant,
    pub refresh_interval: Duration,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub event_sender: mpsc::UnboundedSender<AppEvent>,
    pub event_receiver: mpsc::UnboundedReceiver<AppEvent>,
}

impl App {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        Self {
            job_list: JobList::new(),
            selected_job_index: 0,
            selected_job: None,
            current_user: std::env::var("USER").ok(),
            last_refresh: Instant::now(),
            refresh_interval: Duration::from_secs(2), // Refresh every 2 seconds
            is_loading: false,
            error_message: None,
            event_sender,
            event_receiver,
        }
    }

    pub async fn refresh_jobs(&mut self) -> Result<()> {
        self.is_loading = true;
        self.error_message = None;

        match self.fetch_jobs().await {
            Ok(jobs) => {
                self.job_list.update(jobs);
                self.update_selected_job();
                self.last_refresh = Instant::now();
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to fetch jobs: {}", e));
            }
        }

        self.is_loading = false;
        Ok(())
    }

    async fn fetch_jobs(&self) -> Result<Vec<Job>> {
        // Get basic job list from squeue
        let squeue_output = SlurmCommands::squeue(self.current_user.as_deref()).await?;
        let mut jobs = SlurmParser::parse_squeue_output(&squeue_output)?;

        // For each job, get detailed info from scontrol (but only for first few to avoid overwhelming)
        for job in jobs.iter_mut().take(10) {
            if let Ok(scontrol_output) = SlurmCommands::scontrol_show_job(&job.job_id).await {
                if let Ok(fields) = SlurmParser::parse_scontrol_output(&scontrol_output) {
                    SlurmParser::enhance_job_with_scontrol_data(job, fields);
                }
            }
        }

        Ok(jobs)
    }

    pub fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= self.refresh_interval
    }

    pub fn select_next_job(&mut self) {
        if !self.job_list.jobs.is_empty() && self.selected_job_index < self.job_list.jobs.len() - 1 {
            self.selected_job_index += 1;
            self.update_selected_job();
        }
    }

    pub fn select_previous_job(&mut self) {
        if self.selected_job_index > 0 {
            self.selected_job_index -= 1;
            self.update_selected_job();
        }
    }

    fn update_selected_job(&mut self) {
        self.selected_job = self.job_list.jobs.get(self.selected_job_index).cloned();
    }

    pub fn get_selected_job(&self) -> Option<&Job> {
        self.selected_job.as_ref()
    }

    pub fn running_jobs(&self) -> Vec<&Job> {
        self.job_list.running_jobs()
    }

    pub fn pending_jobs(&self) -> Vec<&Job> {
        self.job_list.pending_jobs()
    }

    pub fn completed_jobs(&self) -> Vec<&Job> {
        self.job_list.completed_jobs()
    }

    pub async fn cancel_selected_job(&mut self) -> Result<()> {
        if let Some(job) = &self.selected_job {
            SlurmCommands::scancel(&job.job_id).await?;
            // Refresh immediately to show the change
            self.refresh_jobs().await?;
        }
        Ok(())
    }

    pub fn send_event(&self, event: AppEvent) -> Result<()> {
        self.event_sender.send(event)?;
        Ok(())
    }

    pub async fn receive_event(&mut self) -> Option<AppEvent> {
        self.event_receiver.recv().await
    }
}