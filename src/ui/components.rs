use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use crate::slurm::logs::{LogRead, TAIL_BYTES, read_tail_for_job};
use crate::ui::theme;
use crate::ui::{App, FocusPanel};
use crate::{AppState, models::Job};

fn render_text_popup(title: &str, app: &App, frame: &mut Frame) {
    let popup_area = centered_rect(36, 9, frame.area());
    frame.render_widget(Clear, popup_area);

    let popup = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(theme::FG))
        .block(popup_block(title))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);

    frame.render_widget(popup, popup_area);
}

fn popup_block(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::ACCENT))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ))
}

pub fn render_app(frame: &mut Frame, app: &App) {
    if app.state == AppState::Fullscreen {
        render_fullscreen(frame, app, frame.area());
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Help/actions bar
        ])
        .split(frame.area());

    render_status_bar(frame, app, chunks[0]);

    let rects = panel_rects(chunks[1]);
    render_jobs_list(frame, app, rects.jobs);
    render_right_header(frame, app, rects.right_header);
    render_job_details(frame, app, rects.details);
    render_job_logs(frame, app, rects.logs);
    render_quick_info(frame, app, rects.summary);

    render_help_bar(app.state, frame, chunks[2]);

    match app.state {
        AppState::UserSearchPopup => render_text_popup("Search user", app, frame),
        AppState::PartitionSearchPopup => render_text_popup("Search partition", app, frame),
        AppState::CancelJobPopup => {
            let Some(target) = &app.cancel_target else {
                return;
            };
            let popup_area = centered_rect(36, 7, frame.area());
            frame.render_widget(Clear, popup_area);

            let body = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Cancel job ", Style::default().fg(theme::FG)),
                    Span::styled(
                        target.job_id.clone(),
                        Style::default()
                            .fg(theme::ACCENT_PINK)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" ?", Style::default().fg(theme::FG)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("y", Style::default().fg(theme::RUNNING).add_modifier(Modifier::BOLD)),
                    Span::styled(" confirm    ", Style::default().fg(theme::MUTED)),
                    Span::styled("n", Style::default().fg(theme::FAILED).add_modifier(Modifier::BOLD)),
                    Span::styled(" cancel", Style::default().fg(theme::MUTED)),
                ]),
            ];

            let popup = Paragraph::new(body)
                .block(popup_block("Confirm"))
                .alignment(Alignment::Center);
            frame.render_widget(popup, popup_area);
        }
        _ => {}
    }
}

/// Layout of the dashboard's main content area. The single source of truth so
/// rendering and mouse hit-testing can never drift apart.
pub struct PanelRects {
    pub jobs: Rect,
    pub right_header: Rect,
    pub details: Rect,
    pub logs: Rect,
    pub summary: Rect,
}

impl PanelRects {
    /// The focusable panel sitting under a point, if any.
    pub fn hit(&self, column: u16, row: u16) -> Option<FocusPanel> {
        let pos = ratatui::layout::Position::new(column, row);
        if self.jobs.contains(pos) {
            Some(FocusPanel::Jobs)
        } else if self.details.contains(pos) {
            Some(FocusPanel::Details)
        } else if self.logs.contains(pos) {
            Some(FocusPanel::Logs)
        } else {
            None
        }
    }
}

/// Split the main area into the panels. Used by `render_app` to draw and by the
/// mouse handler to map a click back to a panel.
pub fn panel_rects(area: Rect) -> PanelRects {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(cols[1]);

    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(40),
            Constraint::Percentage(20),
        ])
        .split(right[1]);

    PanelRects {
        jobs: cols[0],
        right_header: right[0],
        details: body[0],
        logs: body[1],
        summary: body[2],
    }
}

/// The selected job's name as a highlighted pill, centered above the right
/// column. Pink so it reads as job identity, distinct from the purple focus
/// borders.
fn render_right_header(frame: &mut Frame, app: &App, area: Rect) {
    let line = match app.get_selected_job() {
        Some(job) => Line::from(vec![
            Span::styled(
                format!(" {} ", job.name),
                Style::default()
                    .bg(theme::ACCENT_PINK)
                    .fg(theme::BADGE_FG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  job {}", job.display_id()),
                Style::default().fg(theme::MUTED),
            ),
        ]),
        None => Line::from(""),
    };
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(error) = &app.error_message {
        let line = Line::from(vec![
            Span::styled(
                " ✖ ",
                Style::default().bg(theme::FAILED).fg(theme::BADGE_FG),
            ),
            Span::styled(
                format!("  {error}"),
                Style::default().fg(theme::FAILED).add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(14)])
        .split(area);

    let mut left = vec![Span::styled(
        " ❄ lazyslurm ",
        Style::default()
            .bg(theme::ACCENT)
            .fg(theme::BADGE_FG)
            .add_modifier(Modifier::BOLD),
    )];

    let sep = || Span::styled("  ·  ", Style::default().fg(theme::DIM_BORDER));

    if let Some(user) = &app.current_user {
        left.push(sep());
        left.push(Span::styled("user ", Style::default().fg(theme::MUTED)));
        left.push(Span::styled(user.clone(), Style::default().fg(theme::FG)));
    }

    if let Some(part) = &app.current_partition {
        left.push(sep());
        left.push(Span::styled("part ", Style::default().fg(theme::MUTED)));
        left.push(Span::styled(part.clone(), Style::default().fg(theme::FG)));
    }

    left.push(sep());
    left.push(Span::styled(
        format!("{}", app.job_list.jobs.len()),
        Style::default().fg(theme::FG).add_modifier(Modifier::BOLD),
    ));
    left.push(Span::styled(" jobs", Style::default().fg(theme::MUTED)));

    frame.render_widget(Paragraph::new(Line::from(left)), cols[0]);

    if app.is_loading {
        let right = Line::from(vec![
            Span::styled(
                theme::spinner_frame(app.tick),
                Style::default().fg(theme::ACCENT),
            ),
            Span::styled(" refresh ", Style::default().fg(theme::MUTED)),
        ]);
        frame.render_widget(
            Paragraph::new(right).alignment(Alignment::Right),
            cols[1],
        );
    }
}

fn render_jobs_list(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!("Jobs ({})", app.job_list.jobs.len());
    let block = theme::panel(&title, app.focus == FocusPanel::Jobs);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let header = Line::from(Span::styled(
        format!("  {:<7}{:<15}{:<8}STATE", "JOBID", "NAME", "TIME"),
        Style::default().fg(theme::MUTED),
    ));
    frame.render_widget(Paragraph::new(header), rows[0]);

    let jobs: Vec<ListItem> = app
        .job_list
        .jobs
        .iter()
        .enumerate()
        .map(|(i, job)| {
            let selected = i == app.selected_job_index;
            let base = if selected {
                Style::default().bg(theme::SELECT_BG)
            } else {
                Style::default()
            };

            let rail = if selected {
                Span::styled("▌ ", Style::default().fg(theme::ACCENT))
            } else {
                Span::styled("  ", base)
            };

            let job_id = truncate(&job.display_id(), 6);
            let job_name = truncate(&job.name, 14);
            let time_used = job.time_used.as_deref().unwrap_or("--");

            ListItem::new(Line::from(vec![
                rail,
                Span::styled(format!("{:<7}", job_id), base.fg(theme::FG)),
                Span::styled(format!("{:<15}", job_name), base.fg(theme::FG)),
                Span::styled(format!("{:<8}", time_used), base.fg(theme::MUTED)),
                theme::state_badge(&job.state),
            ]))
            .style(base)
        })
        .collect();

    frame.render_widget(List::new(jobs), rows[1]);
}

fn render_job_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = theme::panel("Details", app.focus == FocusPanel::Details);

    let body = if let Some(job) = app.get_selected_job() {
        job_detail_lines(job)
    } else if app.job_list.jobs.is_empty() {
        empty_state_lines(app.quote)
    } else {
        vec![Line::styled(
            "Select a job to view details",
            Style::default().fg(theme::MUTED),
        )]
    };

    let offset = clamp_scroll(app.details_scroll, body.len(), block.inner(area).height);
    let details = Paragraph::new(body)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((offset, 0));
    frame.render_widget(details, area);
}

fn render_job_logs(frame: &mut Frame, app: &App, area: Rect) {
    let block = theme::panel("Logs", app.focus == FocusPanel::Logs);

    match app.get_selected_job().map(|job| read_tail_for_job(job, TAIL_BYTES)) {
        Some(LogRead::Lines { path, text }) => {
            let content = format!("{path}\n{}\n{text}", "─".repeat(40));
            let line_count = content.lines().count();
            let offset = clamp_scroll(app.logs_scroll, line_count, block.inner(area).height);
            frame.render_widget(
                Paragraph::new(content)
                    .style(Style::default().fg(theme::FG))
                    .block(block)
                    .wrap(Wrap { trim: true })
                    .scroll((offset, 0)),
                area,
            );
        }
        Some(LogRead::Empty(_)) => render_placeholder(frame, block, area, "This job's log is empty"),
        Some(LogRead::Missing(_)) => render_placeholder(frame, block, area, "No log output yet"),
        None => render_placeholder(frame, block, area, "Select a job to view logs"),
    }
}

/// Clamp a stored scroll offset to the last useful line given the viewport
/// height, so over-scrolling past the end of the text is a no-op.
fn clamp_scroll(offset: u16, total_lines: usize, viewport: u16) -> u16 {
    let max = (total_lines as u16).saturating_sub(viewport);
    offset.min(max)
}

/// A non-log message in the Logs panel, styled so it doesn't read as log
/// output: centered and muted italic.
fn render_placeholder(frame: &mut Frame, block: Block<'static>, area: Rect, message: &str) {
    let body = vec![
        Line::from(""),
        Line::styled(
            message.to_string(),
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ),
    ];
    frame.render_widget(
        Paragraph::new(body)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
}

fn render_quick_info(frame: &mut Frame, app: &App, area: Rect) {
    let running = app.running_jobs().len();
    let pending = app.pending_jobs().len();
    let completed = app.completed_jobs().len();

    let chips = Line::from(vec![
        count_chip(running, "running", theme::RUNNING),
        Span::raw("  "),
        count_chip(pending, "pending", theme::PENDING),
        Span::raw("  "),
        count_chip(completed, "done", theme::COMPLETED),
    ]);

    let bar = proportion_bar(running, pending, completed, 26);

    let body = vec![Line::from(""), chips, Line::from(""), bar];
    let info = Paragraph::new(body).block(theme::panel("Summary", false));
    frame.render_widget(info, area);
}

fn render_help_bar(app_state: AppState, frame: &mut Frame, area: Rect) {
    let pairs: &[(&str, &str)] = match app_state {
        AppState::Normal => &[
            ("q", "quit"),
            ("←→", "focus"),
            ("↑↓", "nav"),
            ("⏎", "view"),
            ("r", "refresh"),
            ("c", "cancel"),
            ("p", "partition"),
            ("u", "user"),
        ],
        AppState::CancelJobPopup => &[("y", "confirm"), ("n", "reject"), ("esc", "reject")],
        AppState::PartitionSearchPopup | AppState::UserSearchPopup => {
            &[("esc", "close"), ("Enter", "submit")]
        }
        AppState::Fullscreen => &[("esc", "back"), ("↑↓", "scroll"), ("q", "quit")],
    };

    let help = Paragraph::new(hint_line(pairs)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::DIM_BORDER)),
    );

    frame.render_widget(help, area);
}

/// A row of `key label` hints, keys in accent and labels muted.
fn hint_line(pairs: &[(&str, &str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, (key, label)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::default().fg(theme::DIM_BORDER)));
        }
        spans.extend(theme::key_hint(key, label));
    }
    Line::from(spans)
}

fn count_chip(count: usize, label: &str, color: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        format!("● {count} {label}"),
        Style::default().fg(color),
    )
}

/// Braille beads for the proportion bar. `FILL` is a dense, slightly-perforated
/// glyph so filled segments read as a textured ribbon rather than a solid block;
/// `TRACK` is a faint dotted midline for the empty remainder.
const BAR_FILL: &str = "⠿";
const BAR_TRACK: &str = "⠒";

fn proportion_bar(running: usize, pending: usize, completed: usize, width: usize) -> Line<'static> {
    let total = running + pending + completed;
    if total == 0 {
        return Line::from(Span::styled(
            BAR_TRACK.repeat(width),
            Style::default().fg(theme::DIM_BORDER),
        ));
    }

    let cells = |n: usize| ((n as f32 / total as f32) * width as f32).round() as usize;
    let r = cells(running);
    let p = cells(pending);
    let c = cells(completed);
    let rest = width.saturating_sub(r + p + c);

    Line::from(vec![
        Span::styled(BAR_FILL.repeat(r), Style::default().fg(theme::RUNNING)),
        Span::styled(BAR_FILL.repeat(p), Style::default().fg(theme::PENDING)),
        Span::styled(BAR_FILL.repeat(c), Style::default().fg(theme::COMPLETED)),
        Span::styled(BAR_TRACK.repeat(rest), Style::default().fg(theme::DIM_BORDER)),
    ])
}

fn empty_state_lines(quote: crate::ui::quotes::Quote) -> Vec<Line<'static>> {
    let (text, author) = quote;
    vec![
        Line::from(""),
        theme::gradient_line("L A Z Y S L U R M"),
        Line::styled(
            "a tiny SLURM dashboard",
            Style::default().fg(theme::MUTED),
        ),
        Line::from(""),
        Line::styled("No jobs found", Style::default().fg(theme::FG)),
        Line::from(""),
        Line::styled(
            "Try: lazyslurm --user <username>",
            Style::default().fg(theme::MUTED),
        ),
        Line::styled(
            "or check that SLURM is reachable.",
            Style::default().fg(theme::MUTED),
        ),
        Line::from(""),
        Line::styled(
            format!("\"{text}\""),
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ),
        Line::from(Span::styled(
            format!("— {author}"),
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ))
        .alignment(Alignment::Right),
    ]
}

fn kv(key: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<11}"), Style::default().fg(theme::MUTED)),
        Span::styled(value, Style::default().fg(theme::FG)),
    ])
}

fn job_detail_lines(job: &Job) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(theme::state_badge(&job.state)),
        Line::from(""),
        kv("User", job.user.clone()),
        kv("Partition", job.partition.clone()),
    ];

    if let Some(nodes) = job.nodes {
        lines.push(kv("Nodes", nodes.to_string()));
    }
    if let Some(node_list) = &job.node_list {
        lines.push(kv("Node list", node_list.clone()));
    }
    if let Some(submit_time) = &job.submit_time {
        lines.push(kv(
            "Submitted",
            submit_time.format("%Y-%m-%d %H:%M:%S").to_string(),
        ));
    }
    if let Some(start_time) = &job.start_time {
        let label = if matches!(job.state, crate::models::JobState::Pending) {
            "Est. start"
        } else {
            "Started"
        };
        lines.push(kv(label, start_time.format("%Y-%m-%d %H:%M:%S").to_string()));
    }
    if let Some(duration) = job.duration() {
        let total = duration.num_seconds();
        lines.push(kv(
            "Duration",
            format!("{}h {}m {}s", total / 3600, (total % 3600) / 60, total % 60),
        ));
    }
    if let Some(working_dir) = &job.working_dir {
        lines.push(kv("Work dir", working_dir.clone()));
    }
    if let Some(std_out) = &job.std_out {
        lines.push(kv("Log file", std_out.clone()));
    }
    if let Some(reason) = &job.reason {
        lines.push(kv("Reason", reason.clone()));
    }

    lines
}

/// The focused pane, zoomed to the whole screen with a header and key hints.
fn render_fullscreen(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(0),    // body
            Constraint::Length(1), // key hints
        ])
        .split(area);

    let hints: &[(&str, &str)] = match app.fullscreen_panel {
        FocusPanel::Jobs => &[("esc", "back"), ("↑↓", "select"), ("q", "quit")],
        FocusPanel::Details => &[("esc", "back"), ("↑↓", "scroll"), ("q", "quit")],
        FocusPanel::Logs => &[("esc", "back"), ("↑↓", "scroll"), ("G", "follow"), ("q", "quit")],
    };

    frame.render_widget(Paragraph::new(fullscreen_header(app)), rows[0]);

    match app.fullscreen_panel {
        FocusPanel::Jobs => render_jobs_list(frame, app, rows[1]),
        FocusPanel::Details => render_fullscreen_details(frame, app, rows[1]),
        FocusPanel::Logs => render_fullscreen_logs(frame, app, rows[1]),
    }

    frame.render_widget(Paragraph::new(hint_line(hints)), rows[2]);
}

fn fullscreen_header(app: &App) -> Line<'static> {
    let title = match app.fullscreen_panel {
        FocusPanel::Jobs => "Jobs",
        FocusPanel::Details => "Details",
        FocusPanel::Logs => "Logs",
    };

    let mut spans = vec![Span::styled(
        format!(" {title} "),
        Style::default()
            .bg(theme::ACCENT)
            .fg(theme::BADGE_FG)
            .add_modifier(Modifier::BOLD),
    )];

    if let Some(job) = &app.fullscreen_job {
        spans.push(Span::styled(
            format!("  {}  ", job.name),
            Style::default().fg(theme::FG).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!("job {}", job.display_id()),
            Style::default().fg(theme::MUTED),
        ));
    }

    if app.fullscreen_panel == FocusPanel::Logs {
        if app.log_follow {
            spans.push(Span::styled(
                format!("   {} ", theme::spinner_frame(app.tick)),
                Style::default().fg(theme::ACCENT),
            ));
            spans.push(Span::styled(
                "[FOLLOWING]",
                Style::default()
                    .fg(theme::RUNNING)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                "    [PAUSED]",
                Style::default()
                    .fg(theme::PENDING)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    Line::from(spans)
}

fn render_fullscreen_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = theme::panel("Details", true);
    let body = match &app.fullscreen_job {
        Some(job) => job_detail_lines(job),
        None => vec![Line::styled(
            "No job selected",
            Style::default().fg(theme::MUTED),
        )],
    };
    let offset = clamp_scroll(app.fullscreen_scroll, body.len(), block.inner(area).height);
    frame.render_widget(
        Paragraph::new(body)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((offset, 0)),
        area,
    );
}

fn render_fullscreen_logs(frame: &mut Frame, app: &App, area: Rect) {
    let Some(job) = app.fullscreen_job.as_ref() else {
        return;
    };

    match read_tail_for_job(job, TAIL_BYTES) {
        LogRead::Lines { path, text } => {
            let block = theme::panel(&format!("Logs · {path}"), true);
            let viewport = block.inner(area).height;
            let total = text.lines().count();
            let offset = if app.log_follow {
                (total as u16).saturating_sub(viewport)
            } else {
                clamp_scroll(app.fullscreen_scroll, total, viewport)
            };
            frame.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(theme::FG))
                    .block(block)
                    .wrap(Wrap { trim: false })
                    .scroll((offset, 0)),
                area,
            );
        }
        LogRead::Empty(_) => {
            render_placeholder(frame, theme::panel("Logs", true), area, "This job's log is empty")
        }
        LogRead::Missing(_) => {
            render_placeholder(frame, theme::panel("Logs", true), area, "No log output yet")
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let keep: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{keep}...")
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncate_handles_multibyte_names() {
        assert_eq!(truncate("héllo_wörld_jobby", 10), "héllo_w...");
        assert_eq!(
            truncate("日本語のジョブ名テスト確認", 10),
            "日本語のジョブ..."
        );
        assert_eq!(truncate("short", 10), "short");
    }

    #[test]
    fn truncate_handles_emoji_names() {
        assert_eq!(truncate("train_😀_model_v2", 10), "train_😀...");
        assert_eq!(truncate("🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀", 10), "🚀🚀🚀🚀🚀🚀🚀...");
        assert_eq!(truncate("job_🎉", 10), "job_🎉");
    }
}
