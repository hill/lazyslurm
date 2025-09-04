use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::models::{Job, JobState};
use crate::ui::App;

pub fn render_app(frame: &mut Frame, app: &App) {
    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Status bar
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Help/actions bar
        ])
        .split(frame.area());

    // Render status bar
    render_status_bar(frame, app, chunks[0]);

    // Main content area - split horizontally
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // Jobs list
            Constraint::Percentage(60),  // Details/logs
        ])
        .split(chunks[1]);

    // Render jobs list
    render_jobs_list(frame, app, main_chunks[0]);

    // Right side - split vertically for details and logs
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(70),  // Job details
            Constraint::Percentage(30),  // Quick info/logs
        ])
        .split(main_chunks[1]);

    // Render details and quick info
    render_job_details(frame, app, right_chunks[0]);
    render_quick_info(frame, app, right_chunks[1]);

    // Render help bar
    render_help_bar(frame, chunks[2]);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mut status_text = format!("LazySlurm");
    
    if let Some(user) = &app.current_user {
        status_text.push_str(&format!(" - User: {}", user));
    }
    
    status_text.push_str(&format!(" - Jobs: {}", app.job_list.jobs.len()));
    
    if app.is_loading {
        status_text.push_str(" - Loading...");
    }
    
    if let Some(error) = &app.error_message {
        status_text = format!("ERROR: {}", error);
    }

    let status = Paragraph::new(status_text)
        .style(if app.error_message.is_some() {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        });
    
    frame.render_widget(status, area);
}

fn render_jobs_list(frame: &mut Frame, app: &App, area: Rect) {
    let jobs: Vec<ListItem> = app
        .job_list
        .jobs
        .iter()
        .enumerate()
        .map(|(i, job)| {
            let style = if i == app.selected_job_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };

            let state_color = match job.state {
                JobState::Running => Color::Green,
                JobState::Pending => Color::Yellow,
                JobState::Completed => Color::Cyan,
                JobState::Failed => Color::Red,
                JobState::Cancelled => Color::Magenta,
                _ => Color::Gray,
            };

            let job_id = job.display_id();
            let job_name = truncate(&job.name, 15);
            let time_used = job.time_used.as_deref().unwrap_or("--");
            
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<12} ", job_id), Style::default()),
                Span::styled(format!("{:<15} ", job_name), Style::default()),
                Span::styled(format!("{} ", job.state), Style::default().fg(state_color)),
                Span::styled(time_used.to_string(), Style::default()),
            ]))
            .style(style)
        })
        .collect();

    let title = format!("Jobs ({} total)", app.job_list.jobs.len());
    let jobs_list = List::new(jobs)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(jobs_list, area);
}

fn render_job_details(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(job) = app.get_selected_job() {
        format_job_details(job)
    } else if app.job_list.jobs.is_empty() {
        "No jobs found.\n\nTry running: lazyslurm --user <username>\nor check if SLURM is available.".to_string()
    } else {
        "Select a job to view details".to_string()
    };

    let details = Paragraph::new(content)
        .block(Block::default().title("Job Details").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

fn render_quick_info(frame: &mut Frame, app: &App, area: Rect) {
    let running_count = app.running_jobs().len();
    let pending_count = app.pending_jobs().len();
    let completed_count = app.completed_jobs().len();
    
    let content = format!(
        "Running: {} | Pending: {} | Completed: {}\n\nLast updated: {:.1}s ago",
        running_count,
        pending_count,
        completed_count,
        app.last_refresh.elapsed().as_secs_f64()
    );

    let quick_info = Paragraph::new(content)
        .block(Block::default().title("Summary").borders(Borders::ALL));

    frame.render_widget(quick_info, area);
}

fn render_help_bar(frame: &mut Frame, area: Rect) {
    let help_text = "q: quit | ↑↓: navigate | r: refresh | c: cancel job | Enter: job details | ?:help";
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(help, area);
}

fn format_job_details(job: &Job) -> String {
    let mut details = Vec::new();
    
    details.push(format!("Job ID: {}", job.display_id()));
    details.push(format!("Name: {}", job.name));
    details.push(format!("User: {}", job.user));
    details.push(format!("State: {}", job.state));
    details.push(format!("Partition: {}", job.partition));

    if let Some(nodes) = job.nodes {
        details.push(format!("Nodes: {}", nodes));
    }

    if let Some(node_list) = &job.node_list {
        details.push(format!("Node List: {}", node_list));
    }

    if let Some(submit_time) = &job.submit_time {
        details.push(format!("Submitted: {}", submit_time.format("%Y-%m-%d %H:%M:%S")));
    }

    if let Some(start_time) = &job.start_time {
        details.push(format!("Started: {}", start_time.format("%Y-%m-%d %H:%M:%S")));
    }

    if let Some(duration) = job.duration() {
        let total_seconds = duration.num_seconds();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        details.push(format!("Duration: {}h {}m {}s", hours, minutes, seconds));
    }

    if let Some(working_dir) = &job.working_dir {
        details.push(format!("Work Dir: {}", working_dir));
    }

    if let Some(std_out) = &job.std_out {
        details.push(format!("Log File: {}", std_out));
    }

    if let Some(reason) = &job.reason {
        details.push(format!("Reason: {}", reason));
    }

    details.join("\n")
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}