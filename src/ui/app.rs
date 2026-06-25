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
    Fullscreen,
}

/// Which dashboard panel currently holds keyboard focus. Drives the accent
/// glow and what Up/Down act on. Summary is not focusable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FocusPanel {
    Jobs,
    Details,
    Logs,
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
    /// Frame counter driven by the event loop, used to animate the spinner.
    pub tick: u64,
    /// Quote shown on the empty-state panel, picked once per session.
    pub quote: crate::ui::quotes::Quote,
    /// Panel holding keyboard focus on the dashboard.
    pub focus: FocusPanel,
    /// Scroll offsets for the inline Details and Logs panels.
    pub details_scroll: u16,
    pub logs_scroll: u16,
    /// Job snapshotted when a pane is fullscreened, so a background refresh
    /// can't swap content out from under the view. Unused by the Jobs pane,
    /// which renders the live list.
    pub fullscreen_job: Option<Job>,
    /// Which pane is zoomed while `state == Fullscreen`.
    pub fullscreen_panel: FocusPanel,
    /// Scroll offset for the fullscreen Details and Logs views.
    pub fullscreen_scroll: u16,
    /// Whether the fullscreen Logs view auto-scrolls to the newest line.
    pub log_follow: bool,
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
            tick: 0,
            quote: crate::ui::quotes::pick(),
            focus: FocusPanel::Jobs,
            details_scroll: 0,
            logs_scroll: 0,
            fullscreen_job: None,
            fullscreen_panel: FocusPanel::Jobs,
            fullscreen_scroll: 0,
            log_follow: true,
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
        // A different job is now selected, so its inline panels start at the top.
        self.details_scroll = 0;
        self.logs_scroll = 0;
    }

    /// Left/Right move between the two columns. The left column is just the
    /// Jobs list; the right column is the Details/Logs stack.
    pub fn focus_left(&mut self) {
        self.focus = FocusPanel::Jobs;
    }

    pub fn focus_right(&mut self) {
        if self.focus == FocusPanel::Jobs {
            self.focus = FocusPanel::Details;
        }
    }

    /// Up/Down move between the stacked panes on the right.
    pub fn focus_up(&mut self) {
        if self.focus == FocusPanel::Logs {
            self.focus = FocusPanel::Details;
        }
    }

    pub fn focus_down(&mut self) {
        if self.focus == FocusPanel::Details {
            self.focus = FocusPanel::Logs;
        }
    }

    pub fn scroll_focused_down(&mut self, lines: u16) {
        match self.focus {
            FocusPanel::Details => self.details_scroll = self.details_scroll.saturating_add(lines),
            FocusPanel::Logs => self.logs_scroll = self.logs_scroll.saturating_add(lines),
            FocusPanel::Jobs => {}
        }
    }

    pub fn scroll_focused_up(&mut self, lines: u16) {
        match self.focus {
            FocusPanel::Details => self.details_scroll = self.details_scroll.saturating_sub(lines),
            FocusPanel::Logs => self.logs_scroll = self.logs_scroll.saturating_sub(lines),
            FocusPanel::Jobs => {}
        }
    }

    /// Zoom the focused pane to fullscreen. Snapshots the selected job so a
    /// refresh can't swap content out from under Details/Logs.
    pub fn open_fullscreen(&mut self) {
        if self.selected_job.is_some() {
            self.fullscreen_job = self.selected_job.clone();
            self.fullscreen_panel = self.focus;
            self.fullscreen_scroll = 0;
            self.log_follow = true;
            self.state = AppState::Fullscreen;
        }
    }

    pub fn close_fullscreen(&mut self) {
        self.fullscreen_job = None;
        self.state = AppState::Normal;
    }

    /// Scrolling away from the bottom pauses the live tail (Logs only).
    pub fn fullscreen_scroll_up(&mut self, lines: u16) {
        self.log_follow = false;
        self.fullscreen_scroll = self.fullscreen_scroll.saturating_sub(lines);
    }

    pub fn fullscreen_scroll_down(&mut self, lines: u16) {
        self.log_follow = false;
        self.fullscreen_scroll = self.fullscreen_scroll.saturating_add(lines);
    }

    pub fn fullscreen_follow(&mut self) {
        self.log_follow = true;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_moves_spatially_between_panels() {
        let mut app = App::new();
        assert_eq!(app.focus, FocusPanel::Jobs);

        app.focus_left();
        assert_eq!(app.focus, FocusPanel::Jobs, "left edge stays on Jobs");

        app.focus_right();
        assert_eq!(app.focus, FocusPanel::Details);
        app.focus_down();
        assert_eq!(app.focus, FocusPanel::Logs);
        app.focus_down();
        assert_eq!(app.focus, FocusPanel::Logs, "nothing focusable below Logs");
        app.focus_up();
        assert_eq!(app.focus, FocusPanel::Details);
        app.focus_left();
        assert_eq!(app.focus, FocusPanel::Jobs);
    }

    #[test]
    fn scrolling_a_panel_only_moves_its_own_offset() {
        let mut app = App::new();
        app.focus = FocusPanel::Logs;
        app.scroll_focused_down(1);
        app.scroll_focused_down(1);
        assert_eq!(app.logs_scroll, 2);
        assert_eq!(app.details_scroll, 0);
        app.scroll_focused_up(1);
        assert_eq!(app.logs_scroll, 1);
    }
}
