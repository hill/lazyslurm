use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::fs;

use crate::slurm::SlurmParser;
use crate::ui::theme;
use crate::ui::App;
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Help/actions bar
        ])
        .split(frame.area());

    render_status_bar(frame, app, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Jobs list
            Constraint::Percentage(60), // Details/logs
        ])
        .split(chunks[1]);

    render_jobs_list(frame, app, main_chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // Job details
            Constraint::Percentage(40), // Job logs
            Constraint::Percentage(20), // Quick info/summary
        ])
        .split(main_chunks[1]);

    render_job_details(frame, app, right_chunks[0]);
    render_job_logs(frame, app, right_chunks[1]);
    render_quick_info(frame, app, right_chunks[2]);

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
    let block = theme::panel(&title, true);
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
    let block = theme::panel("Details", false);

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

    let details = Paragraph::new(body).block(block).wrap(Wrap { trim: false });
    frame.render_widget(details, area);
}

fn render_job_logs(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(job) = app.get_selected_job() {
        read_job_logs(job)
    } else {
        "Select a job to view logs".to_string()
    };

    let logs = Paragraph::new(content)
        .style(Style::default().fg(theme::FG))
        .block(theme::panel("Logs", false))
        .wrap(Wrap { trim: true });

    frame.render_widget(logs, area);
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
            ("↑↓", "nav"),
            ("r", "refresh"),
            ("c", "cancel"),
            ("p", "partition"),
            ("u", "user"),
        ],
        AppState::CancelJobPopup => &[("y", "confirm"), ("n", "reject"), ("esc", "reject")],
        AppState::PartitionSearchPopup | AppState::UserSearchPopup => {
            &[("esc", "close"), ("Enter", "submit")]
        }
    };

    let mut spans = Vec::new();
    for (i, (key, label)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::default().fg(theme::DIM_BORDER)));
        }
        spans.extend(theme::key_hint(key, label));
    }

    let help = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::DIM_BORDER)),
    );

    frame.render_widget(help, area);
}

fn count_chip(count: usize, label: &str, color: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        format!("● {count} {label}"),
        Style::default().fg(color),
    )
}

fn proportion_bar(running: usize, pending: usize, completed: usize, width: usize) -> Line<'static> {
    let total = running + pending + completed;
    if total == 0 {
        return Line::from(Span::styled(
            "░".repeat(width),
            Style::default().fg(theme::DIM_BORDER),
        ));
    }

    let cells = |n: usize| ((n as f32 / total as f32) * width as f32).round() as usize;
    let r = cells(running);
    let p = cells(pending);
    let c = cells(completed);
    let rest = width.saturating_sub(r + p + c);

    Line::from(vec![
        Span::styled("█".repeat(r), Style::default().fg(theme::RUNNING)),
        Span::styled("█".repeat(p), Style::default().fg(theme::PENDING)),
        Span::styled("█".repeat(c), Style::default().fg(theme::COMPLETED)),
        Span::styled("░".repeat(rest), Style::default().fg(theme::DIM_BORDER)),
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
        Line::from(vec![
            Span::styled(
                format!("{} ", job.display_id()),
                Style::default().fg(theme::FG).add_modifier(Modifier::BOLD),
            ),
            Span::styled(job.name.clone(), Style::default().fg(theme::MUTED)),
        ]),
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

fn read_job_logs(job: &Job) -> String {
    let log_paths = SlurmParser::get_job_log_paths(job);

    // Try each potential log path
    for path in &log_paths {
        if let Ok(content) = fs::read_to_string(path) {
            if content.is_empty() {
                return format!("Log file exists but is empty: {}", path);
            }

            // Show last 20 lines (tail-like behavior)
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(20);
            let tail_lines = &lines[start..];

            return format!(
                "Log file: {}\n{}\n{}",
                path,
                "-".repeat(50),
                tail_lines.join("\n")
            );
        }
    }

    // No logs found
    if log_paths.is_empty() {
        "No log file paths available".to_string()
    } else {
        format!("No logs found. Checked paths:\n{}", log_paths.join("\n"))
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
