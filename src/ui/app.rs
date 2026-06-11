use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::models::{Job, JobList};
use crate::slurm::{SlurmExecutor, SlurmParser, SlurmProcess};

#[derive(Debug)]
pub enum AppEvent {
    JobsFetched {
        generation: u64,
        result: Result<Vec<Job>, String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppState {
    Normal,
    PartitionSearchPopup,
    UserSearchPopup,
    CancelJobPopup,
}

pub struct App {
    pub job_list: JobList,
    pub state: AppState,
    pub selected_job_index: usize,
    pub selected_job: Option<Job>,
    pub current_user: Option<String>,
    pub current_partition: Option<String>,
    pub last_refresh: Instant,
    pub refresh_interval: Duration,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub event_sender: mpsc::UnboundedSender<AppEvent>,
    pub event_receiver: mpsc::UnboundedReceiver<AppEvent>,
    /// Job snapshotted when the cancel popup opens, so the cancel always
    /// applies to the job the user confirmed, even if the list refreshes
    /// underneath the popup.
    pub cancel_target: Option<Job>,
    pub input: String,
    pub executor: Arc<dyn SlurmExecutor>,
    /// Bumped whenever the user/partition filter changes, so results from
    /// fetches started under the old filter are dropped on arrival.
    refresh_generation: u64,
}

impl App {
    pub fn new() -> Self {
        Self::with_executor(Arc::new(SlurmProcess))
    }

    pub fn with_executor(executor: Arc<dyn SlurmExecutor>) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Self {
            job_list: JobList::new(),
            state: AppState::Normal,
            selected_job_index: 0,
            selected_job: None,
            current_user: std::env::var("USER").ok(),
            current_partition: None,
            last_refresh: Instant::now(),
            refresh_interval: Duration::from_secs(2),
            is_loading: false,
            error_message: None,
            event_sender,
            event_receiver,
            cancel_target: None,
            input: "".to_string(),
            executor,
            refresh_generation: 0,
        }
    }

    pub fn with_cli(user: Option<String>, partition: Option<String>) -> Self {
        let mut app = Self::new();
        if user.is_some() {
            app.current_user = user;
        }
        app.current_partition = partition;
        app
    }

    /// Fetch jobs and wait for the result. Used by headless mode, the
    /// initial load, and tests; the TUI loop uses [`Self::start_refresh`].
    pub async fn refresh_jobs(&mut self) -> Result<()> {
        self.is_loading = true;
        let result = Self::fetch_jobs(
            self.executor.clone(),
            self.current_user.clone(),
            self.current_partition.clone(),
        )
        .await
        .map_err(|e| e.to_string());
        self.apply_fetch_result(result);
        Ok(())
    }

    /// Kick off a fetch on a background task so the UI keeps rendering.
    /// The result arrives as an [`AppEvent::JobsFetched`] and is applied
    /// by [`Self::drain_events`]. No-op while a fetch is already running.
    pub fn start_refresh(&mut self) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;

        let executor = self.executor.clone();
        let user = self.current_user.clone();
        let partition = self.current_partition.clone();
        let generation = self.refresh_generation;
        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            let result = Self::fetch_jobs(executor, user, partition)
                .await
                .map_err(|e| e.to_string());
            let _ = sender.send(AppEvent::JobsFetched { generation, result });
        });
    }

    /// Apply any results that background fetches have delivered.
    pub fn drain_events(&mut self) {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                AppEvent::JobsFetched { generation, result } => {
                    if generation == self.refresh_generation {
                        self.apply_fetch_result(result);
                    }
                }
            }
        }
    }

    /// Discard any in-flight fetch and start a fresh one. Called when the
    /// user/partition filter changes so stale results can't apply.
    pub fn invalidate_and_refresh(&mut self) {
        self.refresh_generation += 1;
        self.is_loading = false;
        self.start_refresh();
    }

    fn apply_fetch_result(&mut self, result: Result<Vec<Job>, String>) {
        match result {
            Ok(jobs) => {
                let previous_id = self.selected_job.as_ref().map(|j| j.job_id.clone());
                self.job_list.update(jobs);
                self.sync_selection(previous_id.as_deref());
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to fetch jobs: {}", e));
            }
        }
        self.last_refresh = Instant::now();
        self.is_loading = false;
    }

    async fn fetch_jobs(
        executor: Arc<dyn SlurmExecutor>,
        user: Option<String>,
        partition: Option<String>,
    ) -> Result<Vec<Job>> {
        let squeue_output = executor
            .squeue(user.as_deref(), partition.as_deref())
            .await?;
        let mut jobs = SlurmParser::parse_squeue_output(&squeue_output)?;

        // For each job, get detailed info from scontrol (but only for first few to avoid overwhelming)
        for job in jobs.iter_mut().take(10) {
            if let Ok(scontrol_output) = executor.scontrol_show_job(&job.job_id).await
                && let Ok(fields) = SlurmParser::parse_scontrol_output(&scontrol_output)
            {
                SlurmParser::enhance_job_with_scontrol_data(job, fields);
            }
        }

        Ok(jobs)
    }

    pub fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= self.refresh_interval
    }

    pub fn select_next_job(&mut self) {
        if !self.job_list.jobs.is_empty() && self.selected_job_index < self.job_list.jobs.len() - 1
        {
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

    /// Re-resolve the selection after the job list changes. Follows the
    /// previously selected job by id if it still exists, otherwise clamps
    /// the index so it stays in bounds.
    pub fn sync_selection(&mut self, previous_id: Option<&str>) {
        if let Some(idx) =
            previous_id.and_then(|id| self.job_list.jobs.iter().position(|j| j.job_id == id))
        {
            self.selected_job_index = idx;
        } else if self.selected_job_index >= self.job_list.jobs.len() {
            self.selected_job_index = self.job_list.jobs.len().saturating_sub(1);
        }
        self.update_selected_job();
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

    pub fn open_cancel_popup(&mut self) {
        if self.selected_job.is_some() {
            self.cancel_target = self.selected_job.clone();
            self.state = AppState::CancelJobPopup;
        }
    }

    pub fn dismiss_cancel_popup(&mut self) {
        self.cancel_target = None;
        self.state = AppState::Normal;
    }

    pub async fn confirm_cancel(&mut self) -> Result<()> {
        if let Some(job) = self.cancel_target.take() {
            if let Err(e) = self.executor.scancel(&job.job_id).await {
                self.error_message = Some(format!("Failed to cancel job {}: {}", job.job_id, e));
            } else {
                self.start_refresh();
            }
        }
        self.state = AppState::Normal;
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
