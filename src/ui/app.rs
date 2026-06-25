use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::models::{AcctDetail, AcctEntry, Job, JobList, Node, Partition};
use crate::slurm::{SlurmExecutor, SlurmParser, SlurmProcess};

#[derive(Debug)]
pub enum AppEvent {
    JobsFetched {
        generation: u64,
        result: Result<Vec<Job>, String>,
    },
    NodesFetched(Result<Vec<Node>, String>),
    PartitionsFetched(Result<Vec<Partition>, String>),
    HistoryFetched(Result<Vec<AcctEntry>, String>),
    // Boxed: AcctDetail is much larger than the other variants' payloads.
    HistoryDetailFetched(Result<Box<AcctDetail>, String>),
}

/// The top-level views, switched with Tab or the number keys. Jobs is the
/// original dashboard; the rest are cluster-wide read-only views.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveTab {
    Jobs,
    Nodes,
    Partitions,
    History,
}

impl ActiveTab {
    pub const ALL: [ActiveTab; 4] = [
        ActiveTab::Jobs,
        ActiveTab::Nodes,
        ActiveTab::Partitions,
        ActiveTab::History,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            ActiveTab::Jobs => "Jobs",
            ActiveTab::Nodes => "Nodes",
            ActiveTab::Partitions => "Partitions",
            ActiveTab::History => "History",
        }
    }
}

fn next_index(i: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (i + 1).min(len - 1) }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppState {
    Normal,
    PartitionSearchPopup,
    UserSearchPopup,
    CancelJobPopup,
    Fullscreen,
    /// Fullscreen detail for a finished job on the History tab.
    HistoryDetail,
    /// Typing into the live job-list filter on the Jobs tab.
    FilterInput,
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
    /// Which top-level view is showing.
    pub active_tab: ActiveTab,
    pub nodes: Vec<Node>,
    pub selected_node_index: usize,
    pub partitions: Vec<Partition>,
    pub selected_partition_index: usize,
    pub history: Vec<AcctEntry>,
    pub selected_history_index: usize,
    pub nodes_loading: bool,
    pub partitions_loading: bool,
    pub history_loading: bool,
    /// Last error from each cluster view's fetch, shown in place of the list.
    /// History sets this when slurmdbd accounting isn't available.
    pub nodes_error: Option<String>,
    pub partitions_error: Option<String>,
    pub history_error: Option<String>,
    /// The finished job whose detail view is open, fetched on demand. The id is
    /// snapshotted separately so the log path can be rebuilt before detail lands.
    pub history_detail: Option<AcctDetail>,
    pub history_detail_id: Option<String>,
    pub history_detail_loading: bool,
    pub history_detail_error: Option<String>,
    pub history_detail_scroll: u16,
    /// Live, case-insensitive substring filter over the Jobs list (name or id).
    /// Applied in memory, so it updates on every keystroke.
    pub filter_query: String,
    /// Job ids the user pinned. Pinned jobs float to the top of the list and
    /// stay visible even when the filter would otherwise hide them.
    pub pinned: std::collections::HashSet<String>,
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
            active_tab: ActiveTab::Jobs,
            nodes: Vec::new(),
            selected_node_index: 0,
            partitions: Vec::new(),
            selected_partition_index: 0,
            history: Vec::new(),
            selected_history_index: 0,
            nodes_loading: false,
            partitions_loading: false,
            history_loading: false,
            nodes_error: None,
            partitions_error: None,
            history_error: None,
            history_detail: None,
            history_detail_id: None,
            history_detail_loading: false,
            history_detail_error: None,
            history_detail_scroll: 0,
            filter_query: String::new(),
            pinned: std::collections::HashSet::new(),
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

    /// Switch to a tab and fetch its data if the tab has any of its own.
    pub fn switch_tab(&mut self, tab: ActiveTab) {
        self.active_tab = tab;
        self.refresh_active_tab();
    }

    pub fn next_tab(&mut self) {
        let i = ActiveTab::ALL
            .iter()
            .position(|t| *t == self.active_tab)
            .unwrap_or(0);
        self.switch_tab(ActiveTab::ALL[(i + 1) % ActiveTab::ALL.len()]);
    }

    pub fn prev_tab(&mut self) {
        let i = ActiveTab::ALL
            .iter()
            .position(|t| *t == self.active_tab)
            .unwrap_or(0);
        let n = ActiveTab::ALL.len();
        self.switch_tab(ActiveTab::ALL[(i + n - 1) % n]);
    }

    /// Re-fetch whichever cluster view is showing. The Jobs tab refreshes on
    /// its own timer, so it's a no-op here.
    pub fn refresh_active_tab(&mut self) {
        match self.active_tab {
            ActiveTab::Jobs => {}
            ActiveTab::Nodes => self.start_nodes_refresh(),
            ActiveTab::Partitions => self.start_partitions_refresh(),
            ActiveTab::History => self.start_history_refresh(),
        }
    }

    pub fn start_nodes_refresh(&mut self) {
        if self.nodes_loading {
            return;
        }
        self.nodes_loading = true;
        let executor = self.executor.clone();
        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            let result = executor
                .sinfo_nodes()
                .await
                .map(|out| SlurmParser::parse_sinfo_nodes(&out))
                .map_err(|e| e.to_string());
            let _ = sender.send(AppEvent::NodesFetched(result));
        });
    }

    pub fn start_partitions_refresh(&mut self) {
        if self.partitions_loading {
            return;
        }
        self.partitions_loading = true;
        let executor = self.executor.clone();
        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            let result = executor
                .sinfo_partitions()
                .await
                .map(|out| SlurmParser::parse_sinfo_partitions(&out))
                .map_err(|e| e.to_string());
            let _ = sender.send(AppEvent::PartitionsFetched(result));
        });
    }

    pub fn start_history_refresh(&mut self) {
        if self.history_loading {
            return;
        }
        self.history_loading = true;
        let executor = self.executor.clone();
        let user = self.current_user.clone();
        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            let result = executor
                .sacct(user.as_deref())
                .await
                .map(|out| SlurmParser::parse_sacct(&out))
                .map_err(|e| e.to_string());
            let _ = sender.send(AppEvent::HistoryFetched(result));
        });
    }

    /// Open the detail view for the selected History row and kick off its
    /// fetch. The selected entry's id is snapshotted so the log path can be
    /// rebuilt even before the richer detail arrives.
    pub fn open_history_detail(&mut self) {
        let Some(entry) = self.history.get(self.selected_history_index) else {
            return;
        };
        let job_id = entry.job_id.clone();
        self.history_detail = None;
        self.history_detail_error = None;
        self.history_detail_id = Some(job_id.clone());
        self.history_detail_scroll = 0;
        self.state = AppState::HistoryDetail;
        self.start_history_detail_refresh(job_id);
    }

    pub fn close_history_detail(&mut self) {
        self.history_detail = None;
        self.history_detail_id = None;
        self.state = AppState::Normal;
    }

    fn start_history_detail_refresh(&mut self, job_id: String) {
        self.history_detail_loading = true;
        let executor = self.executor.clone();
        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            let result = executor
                .sacct_job(&job_id)
                .await
                .map_err(|e| e.to_string())
                .and_then(|out| {
                    SlurmParser::parse_sacct_detail(&out, &job_id)
                        .ok_or_else(|| format!("No accounting record for job {job_id}"))
                })
                .map(Box::new);
            let _ = sender.send(AppEvent::HistoryDetailFetched(result));
        });
    }

    pub fn history_detail_scroll_up(&mut self, lines: u16) {
        self.history_detail_scroll = self.history_detail_scroll.saturating_sub(lines);
    }

    pub fn history_detail_scroll_down(&mut self, lines: u16) {
        self.history_detail_scroll = self.history_detail_scroll.saturating_add(lines);
    }

    /// Move the selection down in whichever cluster list is showing.
    pub fn list_next(&mut self) {
        match self.active_tab {
            ActiveTab::Nodes => {
                self.selected_node_index = next_index(self.selected_node_index, self.nodes.len())
            }
            ActiveTab::Partitions => {
                self.selected_partition_index =
                    next_index(self.selected_partition_index, self.partitions.len())
            }
            ActiveTab::History => {
                self.selected_history_index =
                    next_index(self.selected_history_index, self.history.len())
            }
            ActiveTab::Jobs => {}
        }
    }

    pub fn list_prev(&mut self) {
        match self.active_tab {
            ActiveTab::Nodes => {
                self.selected_node_index = self.selected_node_index.saturating_sub(1)
            }
            ActiveTab::Partitions => {
                self.selected_partition_index = self.selected_partition_index.saturating_sub(1)
            }
            ActiveTab::History => {
                self.selected_history_index = self.selected_history_index.saturating_sub(1)
            }
            ActiveTab::Jobs => {}
        }
    }

    pub fn selected_node(&self) -> Option<&Node> {
        self.nodes.get(self.selected_node_index)
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
                AppEvent::NodesFetched(result) => {
                    match result {
                        Ok(nodes) => {
                            let prev = self
                                .nodes
                                .get(self.selected_node_index)
                                .map(|n| n.name.clone());
                            self.nodes = nodes;
                            if let Some(i) =
                                prev.and_then(|name| self.nodes.iter().position(|n| n.name == name))
                            {
                                self.selected_node_index = i;
                            }
                            self.selected_node_index =
                                self.selected_node_index.min(self.nodes.len().saturating_sub(1));
                            self.nodes_error = None;
                        }
                        Err(e) => self.nodes_error = Some(e),
                    }
                    self.nodes_loading = false;
                }
                AppEvent::PartitionsFetched(result) => {
                    match result {
                        Ok(partitions) => {
                            self.partitions = partitions;
                            self.selected_partition_index = self
                                .selected_partition_index
                                .min(self.partitions.len().saturating_sub(1));
                            self.partitions_error = None;
                        }
                        Err(e) => self.partitions_error = Some(e),
                    }
                    self.partitions_loading = false;
                }
                AppEvent::HistoryFetched(result) => {
                    match result {
                        Ok(history) => {
                            self.history = history;
                            self.selected_history_index = self
                                .selected_history_index
                                .min(self.history.len().saturating_sub(1));
                            self.history_error = None;
                        }
                        Err(e) => self.history_error = Some(e),
                    }
                    self.history_loading = false;
                }
                AppEvent::HistoryDetailFetched(result) => {
                    // Ignore a detail that arrives after the view was closed or
                    // a different row was opened.
                    let still_open = self.state == AppState::HistoryDetail;
                    match result {
                        Ok(detail) if still_open => {
                            if self.history_detail_id.as_deref() == Some(detail.job_id.as_str()) {
                                self.history_detail = Some(*detail);
                                self.history_detail_error = None;
                            }
                        }
                        Ok(_) => {}
                        Err(e) => self.history_detail_error = Some(e),
                    }
                    self.history_detail_loading = false;
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

    /// The jobs actually shown in the list: pinned jobs first (always visible),
    /// then everything matching the filter, each keeping squeue order. The
    /// single source of truth for what the list shows and what selection indexes
    /// into, so rendering and navigation can't disagree.
    pub fn visible_jobs(&self) -> Vec<&Job> {
        let q = self.filter_query.to_lowercase();
        let matches = |job: &Job| {
            q.is_empty()
                || job.name.to_lowercase().contains(&q)
                || job.display_id().to_lowercase().contains(&q)
        };

        let mut pinned = Vec::new();
        let mut rest = Vec::new();
        for job in &self.job_list.jobs {
            if self.pinned.contains(&job.job_id) {
                pinned.push(job);
            } else if matches(job) {
                rest.push(job);
            }
        }
        pinned.extend(rest);
        pinned
    }

    pub fn is_pinned(&self, job: &Job) -> bool {
        self.pinned.contains(&job.job_id)
    }

    pub fn select_next_job(&mut self) {
        let len = self.visible_jobs().len();
        if len > 0 && self.selected_job_index < len - 1 {
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
        self.selected_job = self
            .visible_jobs()
            .get(self.selected_job_index)
            .map(|job| (*job).clone());
        // A different job is now selected, so its inline panels start at the top.
        self.details_scroll = 0;
        self.logs_scroll = 0;
    }

    /// Enter filter-typing mode. The list filters live as the query changes.
    pub fn open_filter(&mut self) {
        self.state = AppState::FilterInput;
    }

    /// Apply the typed filter and return to normal navigation, keeping it set.
    pub fn commit_filter(&mut self) {
        self.state = AppState::Normal;
    }

    /// Leave filter mode and drop the filter entirely.
    pub fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.state = AppState::Normal;
        self.reset_selection_to_top();
    }

    pub fn filter_push(&mut self, c: char) {
        self.filter_query.push(c);
        self.reset_selection_to_top();
    }

    pub fn filter_backspace(&mut self) {
        self.filter_query.pop();
        self.reset_selection_to_top();
    }

    /// Pin or unpin the selected job. The selection follows the job by id so it
    /// stays put even though pinning reorders the list.
    pub fn toggle_pin(&mut self) {
        let Some(id) = self.selected_job.as_ref().map(|job| job.job_id.clone()) else {
            return;
        };
        if !self.pinned.remove(&id) {
            self.pinned.insert(id.clone());
        }
        self.sync_selection(Some(&id));
    }

    /// Snap the selection back to the first visible row. Used when the filter
    /// changes the list out from under the old index.
    fn reset_selection_to_top(&mut self) {
        self.selected_job_index = 0;
        self.update_selected_job();
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
        let visible = self.visible_jobs();
        let followed = previous_id.and_then(|id| visible.iter().position(|j| j.job_id == id));
        let len = visible.len();
        drop(visible);

        if let Some(idx) = followed {
            self.selected_job_index = idx;
        } else if self.selected_job_index >= len {
            self.selected_job_index = len.saturating_sub(1);
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

    fn job(id: &str, name: &str) -> Job {
        Job::new(
            id.into(),
            name.into(),
            "u".into(),
            crate::models::JobState::Running,
        )
    }

    fn app_with_jobs(jobs: Vec<Job>) -> App {
        let mut app = App::new();
        app.job_list.update(jobs);
        app
    }

    #[test]
    fn filter_matches_name_or_id_case_insensitively() {
        let mut app = app_with_jobs(vec![
            job("100", "train_resnet"),
            job("101", "eval_run"),
            job("202", "train_bert"),
        ]);

        app.filter_query = "TRAIN".into();
        let ids: Vec<&str> = app.visible_jobs().iter().map(|j| j.job_id.as_str()).collect();
        assert_eq!(ids, vec!["100", "202"]);

        app.filter_query = "202".into();
        let ids: Vec<&str> = app.visible_jobs().iter().map(|j| j.job_id.as_str()).collect();
        assert_eq!(ids, vec!["202"]);
    }

    #[test]
    fn pinned_jobs_float_to_top_and_survive_the_filter() {
        let mut app = app_with_jobs(vec![
            job("100", "train_resnet"),
            job("101", "eval_run"),
            job("202", "train_bert"),
        ]);

        // Pin a job that the filter would otherwise hide.
        app.pinned.insert("101".into());
        app.filter_query = "train".into();

        let ids: Vec<&str> = app.visible_jobs().iter().map(|j| j.job_id.as_str()).collect();
        assert_eq!(ids, vec!["101", "100", "202"], "pinned first, then matches");
    }

    #[test]
    fn toggling_a_pin_keeps_the_same_job_selected() {
        let mut app = app_with_jobs(vec![
            job("100", "a"),
            job("101", "b"),
            job("202", "c"),
        ]);

        // Select the last job, then pin it: it floats to the top but stays selected.
        app.selected_job_index = 2;
        app.update_selected_job();
        assert_eq!(app.selected_job.as_ref().unwrap().job_id, "202");

        app.toggle_pin();
        assert_eq!(app.selected_job_index, 0);
        assert_eq!(app.selected_job.as_ref().unwrap().job_id, "202");
        assert!(app.is_pinned(&job("202", "c")));
    }

    #[test]
    fn clearing_the_filter_resets_selection_to_top() {
        let mut app = app_with_jobs(vec![job("1", "alpha"), job("2", "beta")]);
        app.filter_query = "beta".into();
        app.reset_selection_to_top();
        assert_eq!(app.selected_job.as_ref().unwrap().job_id, "2");

        app.clear_filter();
        assert_eq!(app.selected_job_index, 0);
        assert_eq!(app.selected_job.as_ref().unwrap().job_id, "1");
    }
}
