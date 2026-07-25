use crate::slurm::SlurmParser;
use crate::slurm::logs::{LogRead, TAIL_BYTES, read_tail_for_job, read_tail_for_paths};
use crate::ui::theme;
use crate::ui::{ActiveTab, App, FocusPanel};
use crate::{
    AppState,
    models::{AcctDetail, AcctEntry, FairShareBand, FairShareEntry, Job, Node},
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

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

    if app.state == AppState::HistoryDetail {
        render_history_detail(frame, app, frame.area());
        return;
    }

    if app.state == AppState::RawLog {
        render_raw_log(frame, app, frame.area());
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar (title + tabs)
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Help/actions bar
        ])
        .split(frame.area());

    render_status_bar(frame, app, chunks[0]);

    match app.active_tab {
        ActiveTab::Jobs => render_jobs_dashboard(frame, app, chunks[1]),
        ActiveTab::Nodes => render_nodes_tab(frame, app, chunks[1]),
        ActiveTab::Partitions => render_partitions_tab(frame, app, chunks[1]),
        ActiveTab::History => render_history_tab(frame, app, chunks[1]),
        ActiveTab::Usage => render_usage_tab(frame, app, chunks[1]),
    }

    render_help_bar(app, frame, chunks[2]);

    match app.state {
        AppState::UserSearchPopup => render_text_popup("Search user", app, frame),
        AppState::PartitionSearchPopup => render_text_popup("Search partition", app, frame),
        AppState::CancelJobPopup => {
            let Some(target) = &app.cancel_target else {
                return;
            };
            let popup_area = centered_rect_fixed(44, 7, frame.area());
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
                    Span::styled(
                        "y",
                        Style::default()
                            .fg(theme::RUNNING)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" confirm    ", Style::default().fg(theme::MUTED)),
                    Span::styled(
                        "n",
                        Style::default()
                            .fg(theme::FAILED)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" reject", Style::default().fg(theme::MUTED)),
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

/// The original Jobs view: the five-panel dashboard.
fn render_jobs_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let rects = panel_rects(area);
    render_jobs_list(frame, app, rects.jobs);
    render_right_header(frame, app, rects.right_header);
    render_job_details(frame, app, rects.details);
    render_job_logs(frame, app, rects.logs);
}

/// The tab strip for the right of the status bar. Returns the spans and their
/// width. The width is constant so the right-aligned tabs never shift.
fn tab_strip(app: &App) -> (Vec<Span<'static>>, u16) {
    let mut spans = Vec::new();

    for (i, tab) in ActiveTab::ALL.iter().enumerate() {
        let active = *tab == app.active_tab;
        let label = tab_cell(i, tab);
        if active {
            spans.push(Span::styled(
                label,
                Style::default()
                    .bg(theme::ACCENT)
                    .fg(theme::BADGE_FG)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(label, Style::default().fg(theme::MUTED)));
        }
        spans.push(Span::raw(" ")); // one-space gap between tabs
    }

    let width = spans.iter().map(|s| s.width()).sum::<usize>() as u16;
    (spans, width)
}

/// One tab pill's text, shared by the renderer and the hit-test.
fn tab_cell(i: usize, tab: &ActiveTab) -> String {
    format!(" {} {} ", i + 1, tab.title())
}

/// Screen rect of each clickable tab, mirroring `render_status_bar`'s geometry.
pub struct TabRects {
    rects: Vec<(ActiveTab, Rect)>,
}

impl TabRects {
    pub fn hit(&self, column: u16, row: u16) -> Option<ActiveTab> {
        let pos = ratatui::layout::Position::new(column, row);
        self.rects
            .iter()
            .find(|(_, r)| r.contains(pos))
            .map(|(tab, _)| *tab)
    }
}

pub fn tab_rects(area: Rect) -> TabRects {
    let status = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area)[0];

    let widths: Vec<(ActiveTab, u16)> = ActiveTab::ALL
        .iter()
        .enumerate()
        .map(|(i, tab)| (*tab, tab_cell(i, tab).chars().count() as u16))
        .collect();

    // Each cell is followed by a one-space gap, matching `tab_strip`.
    let total: u16 = widths.iter().map(|(_, w)| w + 1).sum();
    let mut x = status.x + status.width.saturating_sub(total);

    let rects = widths
        .into_iter()
        .map(|(tab, w)| {
            let rect = Rect::new(x, status.y, w, 1);
            x += w + 1;
            (tab, rect)
        })
        .collect();

    TabRects { rects }
}

fn tab_is_loading(app: &App) -> bool {
    match app.active_tab {
        ActiveTab::Jobs => app.is_loading,
        ActiveTab::Nodes => app.nodes_loading,
        ActiveTab::Partitions => app.partitions_loading,
        ActiveTab::History => app.history_loading,
        ActiveTab::Usage => app.fairshare_loading,
    }
}

/// Layout of the dashboard's main content area, shared by render and hit-test.
pub struct PanelRects {
    pub jobs: Rect,
    pub right_header: Rect,
    pub details: Rect,
    pub logs: Rect,
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

/// Split the main area into the dashboard panels.
pub fn panel_rects(area: Rect) -> PanelRects {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(cols[1]);

    // Details takes what it needs for the job metadata; Logs gets the rest,
    // which is the pane you actually read.
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(right[1]);

    PanelRects {
        jobs: cols[0],
        right_header: right[0],
        details: body[0],
        logs: body[1],
    }
}

/// The selected job's name as a header pill above the right column.
fn render_right_header(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::DIM_BORDER));
    let inner = block.inner(area);
    frame.render_widget(block, area);

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
        None => Line::styled(
            "no job selected",
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ),
    };
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), inner);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    // The tabs sit right-aligned on this same line, inline with the title.
    let (tabs, tab_width) = tab_strip(app);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(tab_width)])
        .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(tabs)).alignment(Alignment::Right),
        cols[1],
    );

    if let Some(error) = &app.error_message {
        let line = Line::from(vec![
            Span::styled(
                " ✖ ",
                Style::default().bg(theme::FAILED).fg(theme::BADGE_FG),
            ),
            Span::styled(
                format!("  {error}"),
                Style::default()
                    .fg(theme::FAILED)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), cols[0]);
        return;
    }

    let mut left = vec![Span::styled(
        " ❄ lazyslurm ",
        Style::default()
            .bg(theme::ACCENT)
            .fg(theme::BADGE_FG)
            .add_modifier(Modifier::BOLD),
    )];

    let sep = || Span::raw("   ");

    left.push(sep());
    left.push(Span::styled("user ", Style::default().fg(theme::MUTED)));
    match &app.current_user {
        Some(user) => left.push(Span::styled(user.clone(), Style::default().fg(theme::FG))),
        None => left.push(Span::styled(
            "all",
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        )),
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

    // Refresh spinner sits just after the count so it never disturbs the
    // right-aligned tabs when it pops in and out.
    if tab_is_loading(app) {
        left.push(sep());
        left.push(Span::styled(
            theme::spinner_frame(app.tick),
            Style::default().fg(theme::ACCENT),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(left)), cols[0]);
}

/// Widths for the jobs list columns; `0` means the column is hidden. NAME
/// flexes into whatever is left after the higher-priority columns are placed.
struct JobCols {
    id: usize,
    name: usize,
    partition: usize,
    user: usize,
    time: usize,
}

/// Budget the jobs list columns against the available width. JOBID and the
/// STATE badge are always kept; PARTITION, USER, then TIME drop in that order
/// as the pane narrows, and NAME never shrinks below a readable minimum.
fn job_columns(width: usize, all_users: bool) -> JobCols {
    const MIN_NAME: usize = 10;
    // 4 for the rail+pin prefix, ~12 for the trailing STATE badge.
    let base = width.saturating_sub(4 + 12);
    let id = 10.min(base);
    let mut remaining = base.saturating_sub(id);

    let mut partition = 0;
    let mut user = 0;
    let mut time = 0;
    if remaining >= MIN_NAME + 12 {
        partition = 12;
        remaining -= 12;
    }
    if all_users && remaining >= MIN_NAME + 10 {
        user = 10;
        remaining -= 10;
    }
    if remaining >= MIN_NAME + 8 {
        time = 8;
        remaining -= 8;
    }

    JobCols {
        id,
        name: remaining,
        partition,
        user,
        time,
    }
}

/// A left-aligned cell truncated to leave a one-column gap before the next.
fn col(text: &str, width: usize) -> String {
    format!(
        "{:<width$}",
        truncate(text, width.saturating_sub(1)),
        width = width
    )
}

fn render_jobs_list(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.visible_jobs();
    let total = app.job_list.jobs.len();
    let filtering = app.state == AppState::FilterInput || !app.filter_query.is_empty();

    let title = if app.filter_query.is_empty() {
        format!("Jobs ({total})")
    } else {
        format!("Jobs ({}/{total})", visible.len())
    };
    let block = theme::panel(&title, app.focus == FocusPanel::Jobs);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // A filter line appears above the header only while a filter is in play.
    let constraints: &[Constraint] = if filtering {
        &[
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ]
    } else {
        &[Constraint::Length(1), Constraint::Min(0)]
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let (header_row, list_row) = if filtering {
        frame.render_widget(Paragraph::new(filter_line(app)), rows[0]);
        (rows[1], rows[2])
    } else {
        (rows[0], rows[1])
    };

    // Four leading spaces cover the rail (2) and the pin mark (2).
    let all_users = app.current_user.is_none();
    let cols = job_columns(list_row.width as usize, all_users);

    let mut header_str = String::from("    ");
    header_str.push_str(&col("JOBID", cols.id));
    header_str.push_str(&col("NAME", cols.name));
    if cols.partition > 0 {
        header_str.push_str(&col("PART", cols.partition));
    }
    if cols.user > 0 {
        header_str.push_str(&col("USER", cols.user));
    }
    if cols.time > 0 {
        header_str.push_str(&col("TIME", cols.time));
    }
    header_str.push_str("STATE");
    let header = Line::from(Span::styled(header_str, Style::default().fg(theme::MUTED)));
    frame.render_widget(Paragraph::new(header), header_row);

    let jobs: Vec<ListItem> = visible
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
            let pin = if app.is_pinned(job) {
                Span::styled("★ ", base.fg(theme::ACCENT_PINK))
            } else {
                Span::styled("  ", base)
            };

            let mut spans = vec![
                rail,
                pin,
                Span::styled(col(&job.display_id(), cols.id), base.fg(theme::FG)),
                Span::styled(col(&job.name, cols.name), base.fg(theme::FG)),
            ];
            if cols.partition > 0 {
                spans.push(Span::styled(
                    col(&job.partition, cols.partition),
                    base.fg(theme::MUTED),
                ));
            }
            if cols.user > 0 {
                spans.push(Span::styled(col(&job.user, cols.user), base.fg(theme::MUTED)));
            }
            if cols.time > 0 {
                spans.push(Span::styled(
                    col(job.time_used.as_deref().unwrap_or("--"), cols.time),
                    base.fg(theme::MUTED),
                ));
            }
            spans.push(theme::state_badge(&job.state));

            ListItem::new(Line::from(spans)).style(base)
        })
        .collect();

    // Selection drives the ListState so a long, filtered list scrolls to keep
    // the highlighted row visible.
    let mut state = ratatui::widgets::ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.selected_job_index.min(visible.len() - 1)));
    }
    frame.render_stateful_widget(List::new(jobs), list_row, &mut state);
}

/// A fullscreen jobs-table column: title and fixed character width.
struct JobTableCol {
    title: &'static str,
    width: usize,
}

/// Columns for the fullscreen htop-style jobs table, left to right. The length
/// must stay in sync with `app::JOBS_TABLE_COLUMNS`.
const JOB_TABLE_COLS: [JobTableCol; 7] = [
    JobTableCol { title: "JOBID", width: 14 },
    JobTableCol { title: "NAME", width: 24 },
    JobTableCol { title: "USER", width: 12 },
    JobTableCol { title: "PARTITION", width: 12 },
    JobTableCol { title: "STATE", width: 12 },
    JobTableCol { title: "TIME", width: 10 },
    JobTableCol { title: "NODES", width: 20 },
];

/// The cell text for each column of one job, in `JOB_TABLE_COLS` order.
fn job_table_cells(job: &Job) -> [String; 7] {
    [
        job.display_id(),
        job.name.clone(),
        job.user.clone(),
        job.partition.clone(),
        theme::state_label(&job.state),
        job.time_used.clone().unwrap_or_else(|| "--".to_string()),
        job.node_list.clone().unwrap_or_else(|| "-".to_string()),
    ]
}

/// Render fixed-width cells into a `Line`, showing only the `[scroll, scroll+win)`
/// character window so the table can scroll horizontally without losing styles.
fn slice_cells(cells: &[(String, Style)], scroll: usize, win: usize) -> Line<'static> {
    let end = scroll + win;
    let mut x = 0usize;
    let mut spans: Vec<Span> = Vec::new();
    for (text, style) in cells {
        let chars: Vec<char> = text.chars().collect();
        let cell_start = x;
        let cell_end = x + chars.len();
        x = cell_end;

        let vis_start = cell_start.max(scroll);
        let vis_end = cell_end.min(end);
        if vis_start >= vis_end {
            continue;
        }
        let slice: String = chars[(vis_start - cell_start)..(vis_end - cell_start)]
            .iter()
            .collect();
        spans.push(Span::styled(slice, *style));
    }
    Line::from(spans)
}

/// The fullscreen, htop-style jobs table: every column, a focused column moved
/// with Left/Right, and horizontal scrolling when it is wider than the screen.
fn render_jobs_table(frame: &mut Frame, app: &App, area: Rect) {
    let visible = app.visible_jobs();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    let win = rows[1].width as usize;

    // Character x of each column start, after a 2-wide selection rail.
    const PREFIX: usize = 2;
    let mut starts = [0usize; JOB_TABLE_COLS.len()];
    let mut x = PREFIX;
    for (i, c) in JOB_TABLE_COLS.iter().enumerate() {
        starts[i] = x;
        x += c.width;
    }
    let total_width = x;

    // Scroll just far enough that the focused column's right edge is on screen.
    let focus = app.jobs_col.min(JOB_TABLE_COLS.len() - 1);
    let focus_end = starts[focus] + JOB_TABLE_COLS[focus].width;
    let scroll = if focus_end <= win {
        0
    } else {
        (focus_end - win).min(total_width.saturating_sub(win))
    };

    let mut header_cells: Vec<(String, Style)> = vec![("  ".to_string(), Style::default())];
    for (i, c) in JOB_TABLE_COLS.iter().enumerate() {
        let style = if i == focus {
            Style::default()
                .fg(theme::ACCENT)
                .bg(theme::COL_HL)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::MUTED)
        };
        header_cells.push((col(c.title, c.width), style));
    }
    frame.render_widget(
        Paragraph::new(slice_cells(&header_cells, scroll, win)),
        rows[0],
    );

    let items: Vec<ListItem> = visible
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
                ("▌ ".to_string(), base.fg(theme::ACCENT))
            } else {
                ("  ".to_string(), base)
            };
            let mut cells: Vec<(String, Style)> = vec![rail];

            let values = job_table_cells(job);
            for (ci, c) in JOB_TABLE_COLS.iter().enumerate() {
                let mut style = match c.title {
                    "STATE" => base.fg(theme::state_color(&job.state)),
                    "JOBID" | "NAME" => base.fg(theme::FG),
                    _ => base.fg(theme::MUTED),
                };
                if ci == focus {
                    // A background band marks the focused column across every row,
                    // overriding the row-selection background where they cross.
                    style = style.bg(theme::COL_HL).add_modifier(Modifier::BOLD);
                }
                cells.push((col(&values[ci], c.width), style));
            }

            ListItem::new(slice_cells(&cells, scroll, win)).style(base)
        })
        .collect();

    let mut state = ratatui::widgets::ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.selected_job_index.min(visible.len() - 1)));
    }
    frame.render_stateful_widget(List::new(items), rows[1], &mut state);
}

/// The live filter line above the job list.
fn filter_line(app: &App) -> Line<'static> {
    let typing = app.state == AppState::FilterInput;
    let accent = if typing { theme::ACCENT } else { theme::MUTED };

    let mut spans = vec![
        Span::styled(
            "/",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.filter_query.clone(),
            Style::default().fg(if typing { theme::FG } else { theme::MUTED }),
        ),
    ];

    if typing {
        spans.push(Span::styled("▏", Style::default().fg(theme::ACCENT)));
        spans.push(Span::styled(
            "   enter to apply, esc to clear",
            Style::default().fg(theme::DIM_BORDER),
        ));
    } else {
        spans.push(Span::styled(
            "   esc to clear",
            Style::default().fg(theme::DIM_BORDER),
        ));
    }

    Line::from(spans)
}

fn render_job_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = theme::panel("Details", app.focus == FocusPanel::Details);

    let body = if let Some(job) = app.get_selected_job() {
        job_detail_lines(app, job)
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
    let focused = app.focus == FocusPanel::Logs;

    match app
        .get_selected_job()
        .map(|job| read_tail_for_job(job, TAIL_BYTES))
    {
        Some(LogRead::Lines { path, text }) => {
            let block = logs_panel(&path, focused, area.width);
            let line_count = text.lines().count();
            let offset = clamp_scroll(app.logs_scroll, line_count, block.inner(area).height);
            frame.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(theme::FG))
                    .block(block)
                    .wrap(Wrap { trim: true })
                    .scroll((offset, 0)),
                area,
            );
        }
        Some(LogRead::Empty(_)) => {
            render_placeholder(frame, theme::panel("Logs", focused), area, "This job's log is empty")
        }
        Some(LogRead::Missing(_)) => {
            render_placeholder(frame, theme::panel("Logs", focused), area, "No log output yet")
        }
        None => render_placeholder(
            frame,
            theme::panel("Logs", focused),
            area,
            "Select a job to view logs",
        ),
    }
}

/// Clamp a scroll offset so over-scrolling past the end is a no-op.
fn clamp_scroll(offset: u16, total_lines: usize, viewport: u16) -> u16 {
    let max = (total_lines as u16).saturating_sub(viewport);
    offset.min(max)
}

/// A centered, muted message in place of log output.
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

fn render_help_bar(app: &App, frame: &mut Frame, area: Rect) {
    let jobs_hints: &[(&str, &str)] = &[
        ("q", "quit"),
        ("⇥", "tab"),
        ("↑↓", "nav"),
        ("⏎", "view"),
        ("/", "filter"),
        ("P", "pin"),
        ("u", "users"),
        ("a", "all"),
        ("r", "refresh"),
        ("c", "cancel"),
    ];
    let usage_hints: &[(&str, &str)] = &[
        ("q", "quit"),
        ("⇥", "tab"),
        ("a", "all"),
        ("r", "refresh"),
    ];
    let cluster_hints: &[(&str, &str)] = &[
        ("q", "quit"),
        ("⇥", "tab"),
        ("↑↓", "nav"),
        ("r", "refresh"),
        ("u", "user"),
    ];
    let history_hints: &[(&str, &str)] = &[
        ("q", "quit"),
        ("⇥", "tab"),
        ("↑↓", "nav"),
        ("⏎", "detail"),
        ("r", "refresh"),
        ("u", "user"),
    ];

    let pairs: &[(&str, &str)] = match app.state {
        AppState::Normal if app.active_tab == ActiveTab::Jobs => jobs_hints,
        AppState::Normal if app.active_tab == ActiveTab::History => history_hints,
        AppState::Normal if app.active_tab == ActiveTab::Usage => usage_hints,
        AppState::Normal => cluster_hints,
        AppState::CancelJobPopup => &[("y", "confirm"), ("n", "reject"), ("esc", "reject")],
        AppState::PartitionSearchPopup | AppState::UserSearchPopup => {
            &[("esc", "close"), ("Enter", "submit")]
        }
        AppState::Fullscreen => &[("esc", "back"), ("↑↓", "scroll"), ("q", "quit")],
        AppState::HistoryDetail => &[
            ("esc", "back"),
            ("↑↓", "scroll"),
            ("y", "raw"),
            ("q", "quit"),
        ],
        AppState::FilterInput => &[("⏎", "apply"), ("esc", "clear"), ("⌫", "delete")],
        AppState::RawLog => &[("↑↓", "scroll"), ("esc", "exit")],
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

/// Braille beads for the node/partition load bars: filled, then dim track.
const BAR_FILL: &str = "⠿";
const BAR_TRACK: &str = "⠒";

fn empty_state_lines(quote: crate::ui::quotes::Quote) -> Vec<Line<'static>> {
    let (text, author) = quote;
    vec![
        Line::from(""),
        theme::gradient_line("L A Z Y S L U R M"),
        Line::styled("a tiny SLURM dashboard", Style::default().fg(theme::MUTED)),
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

/// A labelled progress bar row (`label ▓▓▓░░  suffix`) for the Details pane.
fn progress_line(
    label: &str,
    filled: usize,
    total: usize,
    color: ratatui::style::Color,
    suffix: String,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{label:<11}"),
        Style::default().fg(theme::MUTED),
    )];
    spans.extend(mini_bar(filled, total, 12, color));
    spans.push(Span::styled(
        format!("  {suffix}"),
        Style::default().fg(theme::FG),
    ));
    Line::from(spans)
}

const SPARK: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A block-character sparkline of the per-tick deltas.
fn sparkline(deltas: &[u64]) -> String {
    let max = deltas.iter().copied().max().unwrap_or(0);
    if max == 0 {
        return SPARK[0].to_string().repeat(deltas.len().max(1));
    }
    deltas
        .iter()
        .map(|&d| {
            let idx = ((d as f64 / max as f64) * (SPARK.len() - 1) as f64).round() as usize;
            SPARK[idx.min(SPARK.len() - 1)]
        })
        .collect()
}

/// Progress rows derived from live signals: wall-clock budget, array task
/// aggregate, and a log-activity heartbeat. Only shown for running jobs.
fn job_progress_lines(app: &App, job: &Job) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    if !job.is_running() {
        return out;
    }

    if let (Some(frac), Some(used), Some(limit)) =
        (job.walltime_fraction(), &job.time_used, &job.time_limit)
    {
        let color = if frac < 0.75 {
            theme::RUNNING
        } else if frac < 0.9 {
            theme::PENDING
        } else {
            theme::FAILED
        };
        let pct = (frac * 100.0).round() as u32;
        out.push(progress_line(
            "Walltime",
            job.elapsed_secs().unwrap_or(0) as usize,
            job.limit_secs().unwrap_or(1) as usize,
            color,
            format!("{pct}%  {used} / {limit}"),
        ));
    }

    if let Some(array_id) = &job.array_job_id {
        let (mut running, mut pending) = (0u64, 0u64);
        for j in &app.job_list.jobs {
            if j.array_job_id.as_deref() != Some(array_id.as_str()) {
                continue;
            }
            let n = crate::models::array_task_count(&j.job_id);
            match j.state {
                crate::models::JobState::Running => running += n,
                crate::models::JobState::Pending => pending += n,
                _ => {}
            }
        }
        let total = running + pending;
        if total > 0 {
            out.push(progress_line(
                "Array",
                running as usize,
                total as usize,
                theme::RUNNING,
                format!("{running} run · {pending} pend"),
            ));
        }
    }

    if app.activity_job_id.as_deref() == Some(job.job_id.as_str()) && app.activity.len() >= 2 {
        let sizes: Vec<u64> = app.activity.iter().copied().collect();
        let deltas: Vec<u64> = sizes.windows(2).map(|w| w[1].saturating_sub(w[0])).collect();
        let flowing = deltas.iter().rev().take(3).any(|&d| d > 0);
        let (note, color) = if flowing {
            (" writing", theme::RUNNING)
        } else {
            (" quiet", theme::MUTED)
        };
        out.push(Line::from(vec![
            Span::styled("Activity   ", Style::default().fg(theme::MUTED)),
            Span::styled(sparkline(&deltas), Style::default().fg(color)),
            Span::styled(note, Style::default().fg(theme::MUTED)),
        ]));
    }

    if !out.is_empty() {
        out.push(Line::from(""));
    }
    out
}

fn job_detail_lines(app: &App, job: &Job) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(theme::state_badge(&job.state)),
        Line::from(""),
    ];
    lines.extend(job_progress_lines(app, job));
    lines.push(kv("User", job.user.clone()));
    lines.push(kv("Partition", job.partition.clone()));

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
        lines.push(kv(
            label,
            start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
        ));
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
        FocusPanel::Jobs => &[
            ("esc", "back"),
            ("↑↓", "select"),
            ("←→", "columns"),
            ("q", "quit"),
        ],
        FocusPanel::Details => &[("esc", "back"), ("↑↓", "scroll"), ("q", "quit")],
        FocusPanel::Logs => &[
            ("esc", "back"),
            ("↑↓", "scroll"),
            ("G", "follow"),
            ("y", "raw"),
            ("q", "quit"),
        ],
    };

    frame.render_widget(Paragraph::new(fullscreen_header(app)), rows[0]);

    match app.fullscreen_panel {
        FocusPanel::Jobs => render_jobs_table(frame, app, rows[1]),
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
        Some(job) => job_detail_lines(app, job),
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
            let block = logs_panel(&path, true, area.width);
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
        LogRead::Empty(_) => render_placeholder(
            frame,
            theme::panel("Logs", true),
            area,
            "This job's log is empty",
        ),
        LogRead::Missing(_) => {
            render_placeholder(frame, theme::panel("Logs", true), area, "No log output yet")
        }
    }
}

/// Plain, borderless, no-wrap log view so a terminal selection stays clean
/// (one screen row per log line). Mouse capture is released in the event loop.
fn render_raw_log(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let mut header = vec![Span::styled(
        " RAW ",
        Style::default()
            .bg(theme::ACCENT_PINK)
            .fg(theme::BADGE_FG)
            .add_modifier(Modifier::BOLD),
    )];

    match read_tail_for_paths(app.raw_log_paths.clone(), TAIL_BYTES) {
        LogRead::Lines { path, text } => {
            header.push(Span::styled(
                format!("  {path}"),
                Style::default().fg(theme::MUTED),
            ));
            header.push(Span::styled(
                "   esc to exit",
                Style::default().fg(theme::DIM_BORDER),
            ));
            frame.render_widget(Paragraph::new(Line::from(header)), rows[0]);

            let viewport = rows[1].height;
            let total = text.lines().count();
            let offset = if app.log_follow {
                (total as u16).saturating_sub(viewport)
            } else {
                clamp_scroll(app.fullscreen_scroll, total, viewport)
            };
            frame.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(theme::FG))
                    .scroll((offset, 0)),
                rows[1],
            );
        }
        LogRead::Empty(_) => {
            frame.render_widget(Paragraph::new(Line::from(header)), rows[0]);
            render_placeholder(
                frame,
                theme::panel("Logs", true),
                rows[1],
                "This job's log is empty",
            );
        }
        LogRead::Missing(_) => {
            frame.render_widget(Paragraph::new(Line::from(header)), rows[0]);
            render_placeholder(
                frame,
                theme::panel("Logs", true),
                rows[1],
                "No log output yet",
            );
        }
    }
}

/// A titled panel with a column header and a selectable, scrolling list.
/// `message` replaces the list for loading / error / empty.
fn render_cluster_list(
    frame: &mut Frame,
    title: &str,
    header: &str,
    items: Vec<ListItem<'static>>,
    selected: usize,
    message: Option<&str>,
    area: Rect,
) {
    let block = theme::panel(title, true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(msg) = message {
        let body = vec![
            Line::from(""),
            Line::styled(
                msg.to_string(),
                Style::default()
                    .fg(theme::MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
        ];
        frame.render_widget(Paragraph::new(body).alignment(Alignment::Center), inner);
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            header.to_string(),
            Style::default().fg(theme::MUTED),
        ))),
        rows[0],
    );

    // A ListState carries the selection so the list scrolls to keep the
    // highlighted row on screen; the row's own styling draws the highlight.
    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(List::new(items), rows[1], &mut state);
}

/// The loading / error / empty message for a cluster list, or `None` if it has rows.
fn cluster_message<'a>(
    loading: bool,
    error: &'a Option<String>,
    empty: bool,
    empty_msg: &'a str,
) -> Option<&'a str> {
    if let Some(err) = error {
        Some(err.as_str())
    } else if empty && loading {
        Some("Loading…")
    } else if empty {
        Some(empty_msg)
    } else {
        None
    }
}

/// The leading selection rail plus the base row style, shared by every list.
fn row_base(selected: bool) -> (Span<'static>, Style) {
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
    (rail, base)
}

/// A short braille bar showing `filled` of `total` in `color`.
fn mini_bar(
    filled: usize,
    total: usize,
    width: usize,
    color: ratatui::style::Color,
) -> Vec<Span<'static>> {
    let cells = if total == 0 {
        0
    } else {
        ((filled as f32 / total as f32) * width as f32).round() as usize
    }
    .min(width);

    vec![
        Span::styled(BAR_FILL.repeat(cells), Style::default().fg(color)),
        Span::styled(
            BAR_TRACK.repeat(width - cells),
            Style::default().fg(theme::DIM_BORDER),
        ),
    ]
}

/// Megabytes to a compact whole-GB string, e.g. `245G`. `None` becomes `-`.
fn fmt_gb(mb: Option<u64>) -> String {
    match mb {
        Some(mb) => format!("{}G", mb / 1024),
        None => "-".to_string(),
    }
}

fn node_state_color(node: &Node) -> ratatui::style::Color {
    if node.is_unavailable() {
        return theme::FAILED;
    }
    let s = node.state.to_lowercase();
    if s.contains("idle") {
        theme::RUNNING
    } else if s.contains("alloc") || s.contains("mix") {
        theme::COMPLETED
    } else {
        theme::MUTED
    }
}

fn render_nodes_tab(frame: &mut Frame, app: &App, area: Rect) {
    // These two columns hold numbers, so they can't be truncated like the
    // text columns. Size them to the widest value (with the old widths as
    // minimums) so big CPU/RAM figures keep a space before the next column
    // instead of merging into it, e.g. "128/1282766G/1511G".
    let cpu_w = app
        .nodes
        .iter()
        .map(|n| format!("{}/{}", n.cpus_alloc, n.cpus_total).len() + 1)
        .max()
        .unwrap_or(0)
        .max(7);
    let mem_w = app
        .nodes
        .iter()
        .map(|n| format!("{}/{}", fmt_gb(n.free_mem_mb), fmt_gb(n.memory_mb)).len() + 1)
        .max()
        .unwrap_or(0)
        .max(12);
    // The CPUS header spans the 6-cell bar plus the value column.
    let cpu_col = 7 + cpu_w;

    let header = format!(
        "  {:<18}{:<10}{:<cpu_col$}{:<mem_w$}{:<20}PART",
        "NODE", "STATE", "CPUS", "MEM f/t", "GPU"
    );

    let items: Vec<ListItem> = app
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let (rail, base) = row_base(i == app.selected_node_index);
            let color = node_state_color(node);

            let mut spans = vec![
                rail,
                Span::styled(
                    format!("{:<18}", truncate(&node.name, 17)),
                    base.fg(theme::FG),
                ),
                Span::styled(format!("{:<10}", truncate(&node.state, 9)), base.fg(color)),
            ];
            spans.extend(mini_bar(
                node.cpus_alloc as usize,
                node.cpus_total as usize,
                6,
                color,
            ));
            spans.push(Span::styled(
                format!(
                    " {:<cpu_w$}",
                    format!("{}/{}", node.cpus_alloc, node.cpus_total)
                ),
                base.fg(theme::MUTED),
            ));
            spans.push(Span::styled(
                format!(
                    "{:<mem_w$}",
                    format!("{}/{}", fmt_gb(node.free_mem_mb), fmt_gb(node.memory_mb))
                ),
                base.fg(theme::MUTED),
            ));
            spans.push(Span::styled(
                format!("{:<20}", truncate(node.gres.as_deref().unwrap_or("-"), 19)),
                base.fg(theme::FG),
            ));
            spans.push(Span::styled(
                truncate(&node.partition, 12),
                base.fg(theme::MUTED),
            ));

            ListItem::new(Line::from(spans)).style(base)
        })
        .collect();

    let title = format!("Nodes ({})", app.nodes.len());
    let message = cluster_message(
        app.nodes_loading,
        &app.nodes_error,
        app.nodes.is_empty(),
        "No nodes reported",
    );
    render_cluster_list(
        frame,
        &title,
        &header,
        items,
        app.selected_node_index,
        message,
        area,
    );
}

fn render_partitions_tab(frame: &mut Frame, app: &App, area: Rect) {
    let header = format!(
        "  {:<18}{:<8}{:<14}{:<10}TIMELIMIT",
        "PARTITION", "AVAIL", "NODES", "i/t"
    );

    let items: Vec<ListItem> = app
        .partitions
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let (rail, base) = row_base(i == app.selected_partition_index);
            let up = part.is_up();
            let name = if part.is_default {
                format!("{}*", part.name)
            } else {
                part.name.clone()
            };

            let mut spans = vec![
                rail,
                Span::styled(format!("{:<18}", truncate(&name, 17)), base.fg(theme::FG)),
                Span::styled(
                    format!("{:<8}", part.availability),
                    base.fg(if up { theme::RUNNING } else { theme::FAILED }),
                ),
            ];
            spans.extend(mini_bar(
                part.nodes_idle as usize,
                part.nodes_total as usize,
                6,
                theme::RUNNING,
            ));
            spans.push(Span::styled(
                format!(
                    " {:<7}",
                    format!("{}/{}", part.nodes_idle, part.nodes_total)
                ),
                base.fg(theme::MUTED),
            ));
            spans.push(Span::styled(format!("{:<10}", ""), base.fg(theme::MUTED)));
            spans.push(Span::styled(part.time_limit.clone(), base.fg(theme::FG)));

            ListItem::new(Line::from(spans)).style(base)
        })
        .collect();

    let title = format!("Partitions ({})", app.partitions.len());
    let message = cluster_message(
        app.partitions_loading,
        &app.partitions_error,
        app.partitions.is_empty(),
        "No partitions reported",
    );
    render_cluster_list(
        frame,
        &title,
        &header,
        items,
        app.selected_partition_index,
        message,
        area,
    );
}

fn fairshare_color(band: Option<FairShareBand>) -> ratatui::style::Color {
    match band {
        Some(FairShareBand::Penalised) => theme::PENDING,
        Some(FairShareBand::Boosted) => theme::RUNNING,
        _ => theme::MUTED,
    }
}

/// A plain-language reading of the invoking user's fairshare factor, with the
/// value and the verdict word emphasised.
fn fairshare_reading(row: Option<&FairShareEntry>) -> Line<'static> {
    let muted = Style::default()
        .fg(theme::MUTED)
        .add_modifier(Modifier::ITALIC);
    let strong = Style::default()
        .fg(theme::FG)
        .add_modifier(Modifier::BOLD | Modifier::ITALIC);

    let Some((row, f)) = row.and_then(|r| r.fair_share.map(|f| (r, f))) else {
        return Line::styled("No fairshare factor reported for your account.", muted);
    };

    let (verdict, tail) = match row.band() {
        Some(FairShareBand::Penalised) => (
            "below",
            " the 0.5 midpoint. Recent usage has run ahead of your target share, so new jobs get a mild priority penalty. It decays over time and recovers as you idle.",
        ),
        Some(FairShareBand::Boosted) => (
            "above",
            " the 0.5 midpoint. You are under your target share, so new jobs get a small priority boost.",
        ),
        _ => (
            "around",
            " the neutral 0.5 midpoint. Your usage is close to your target share.",
        ),
    };

    Line::from(vec![
        Span::styled("FairShare ", muted),
        Span::styled(format!("{f:.4}"), strong.fg(fairshare_color(row.band()))),
        Span::styled(" sits ", muted),
        Span::styled(verdict, strong),
        Span::styled(tail, muted),
    ])
}

fn render_usage_tab(frame: &mut Frame, app: &App, area: Rect) {
    let block = theme::panel("Usage", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(msg) = cluster_message(
        app.fairshare_loading,
        &app.fairshare_error,
        app.fairshare.is_empty(),
        "sshare returned no rows",
    ) {
        let body = vec![
            Line::from(""),
            Line::styled(
                msg.to_string(),
                Style::default()
                    .fg(theme::MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
        ];
        frame.render_widget(Paragraph::new(body).alignment(Alignment::Center), inner);
        return;
    }

    let heading = |text: &str| {
        Line::styled(
            text.to_string(),
            Style::default().fg(theme::FG).add_modifier(Modifier::BOLD),
        )
    };
    let dash = "-".to_string();

    let mut lines: Vec<Line> = vec![
        heading("Your fairshare standing"),
        Line::from(""),
        Line::styled(
            format!(
                "  {:<12}{:<12}{:>12}{:>14}{:>12}",
                "USER", "ACCOUNT", "RAWUSAGE", "EFFECTVUSAGE", "FAIRSHARE"
            ),
            Style::default().fg(theme::MUTED),
        ),
    ];

    for e in &app.fairshare {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<12}", truncate(&e.user, 11)), Style::default().fg(theme::FG)),
            Span::styled(
                format!("{:<12}", truncate(&e.account, 11)),
                Style::default().fg(theme::MUTED),
            ),
            Span::styled(
                format!("{:>12}", e.raw_usage.map(|u| u.to_string()).unwrap_or_else(|| dash.clone())),
                Style::default().fg(theme::MUTED),
            ),
            Span::styled(
                format!("{:>14}", e.effectv_usage.map(|u| format!("{u:.6}")).unwrap_or_else(|| dash.clone())),
                Style::default().fg(theme::MUTED),
            ),
            Span::styled(
                format!("{:>12}", e.fair_share.map(|u| format!("{u:.4}")).unwrap_or_else(|| dash.clone())),
                Style::default()
                    .fg(fairshare_color(e.band()))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    // Read the invoking user's row (their own, even while showing all users).
    let me = app.my_user.as_deref().or(app.current_user.as_deref());
    let reading_row = me
        .and_then(|u| app.fairshare.iter().find(|e| e.user == u))
        .or_else(|| app.fairshare.first());

    lines.push(Line::from(""));
    lines.push(fairshare_reading(reading_row));

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }),
        inner,
    );
}

fn render_history_tab(frame: &mut Frame, app: &App, area: Rect) {
    let header = format!(
        "  {:<12}{:<18}{:<12}{:<8}{:<12}ENDED",
        "JOBID", "NAME", "STATE", "EXIT", "ELAPSED"
    );

    let items: Vec<ListItem> = app
        .history
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let (rail, base) = row_base(i == app.selected_history_index);
            let color = history_color(entry);

            ListItem::new(Line::from(vec![
                rail,
                Span::styled(
                    format!("{:<12}", truncate(&entry.job_id, 11)),
                    base.fg(theme::FG),
                ),
                Span::styled(
                    format!("{:<18}", truncate(&entry.name, 17)),
                    base.fg(theme::FG),
                ),
                Span::styled(
                    format!("{:<12}", truncate(&entry.state, 11)),
                    base.fg(color),
                ),
                Span::styled(
                    format!("{:<8}", entry.exit_code.clone()),
                    base.fg(theme::MUTED),
                ),
                Span::styled(
                    format!("{:<12}", entry.elapsed.clone()),
                    base.fg(theme::MUTED),
                ),
                Span::styled(truncate(&entry.end, 19), base.fg(theme::MUTED)),
            ]))
            .style(base)
        })
        .collect();

    // sacct erroring almost always means slurmdbd accounting isn't set up;
    // say so plainly rather than leaving an empty pane.
    let message = if app.history_error.is_some() {
        Some("Accounting not available (slurmdbd not configured)")
    } else {
        cluster_message(
            app.history_loading,
            &None,
            app.history.is_empty(),
            "No recent jobs",
        )
    };
    let title = format!("History ({})", app.history.len());
    render_cluster_list(
        frame,
        &title,
        &header,
        items,
        app.selected_history_index,
        message,
        area,
    );
}

fn history_color(entry: &AcctEntry) -> ratatui::style::Color {
    let s = entry.state.to_uppercase();
    if s.starts_with("RUNNING") || s.starts_with("PENDING") {
        theme::RUNNING
    } else if entry.succeeded() {
        theme::COMPLETED
    } else {
        theme::FAILED
    }
}

fn acct_state_color(state: &str, exit_code: &str) -> ratatui::style::Color {
    let s = state.to_uppercase();
    if s.starts_with("RUNNING") || s.starts_with("PENDING") {
        theme::RUNNING
    } else if exit_code == "0:0" && s.starts_with("COMPLETED") {
        theme::COMPLETED
    } else {
        theme::FAILED
    }
}

/// The fullscreen History detail: sacct fields up top, best-effort log below.
fn render_history_detail(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Length(17), // details
            Constraint::Min(0),     // logs
            Constraint::Length(1),  // hints
        ])
        .split(area);

    frame.render_widget(Paragraph::new(history_detail_header(app)), rows[0]);

    let details_block = theme::panel("Details", true);
    match &app.history_detail {
        Some(detail) => {
            let body = acct_detail_lines(detail);
            let offset = clamp_scroll(0, body.len(), details_block.inner(rows[1]).height);
            frame.render_widget(
                Paragraph::new(body)
                    .block(details_block)
                    .wrap(Wrap { trim: false })
                    .scroll((offset, 0)),
                rows[1],
            );
            render_history_detail_logs(frame, app, detail, rows[2]);
        }
        None => {
            let msg = app
                .history_detail_error
                .as_deref()
                .unwrap_or("Loading job detail…");
            render_placeholder(frame, details_block, rows[1], msg);
            frame.render_widget(theme::panel("Logs", true), rows[2]);
        }
    }

    let hints: &[(&str, &str)] = &[
        ("esc", "back"),
        ("↑↓", "scroll"),
        ("y", "raw"),
        ("q", "quit"),
    ];
    frame.render_widget(Paragraph::new(hint_line(hints)), rows[3]);
}

fn history_detail_header(app: &App) -> Line<'static> {
    let mut spans = vec![Span::styled(
        " History ",
        Style::default()
            .bg(theme::ACCENT)
            .fg(theme::BADGE_FG)
            .add_modifier(Modifier::BOLD),
    )];

    let id = app
        .history_detail
        .as_ref()
        .map(|d| d.job_id.clone())
        .or_else(|| app.history_detail_id.clone())
        .unwrap_or_default();

    if let Some(detail) = &app.history_detail {
        spans.push(Span::styled(
            format!("  {}  ", detail.name),
            Style::default().fg(theme::FG).add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::styled(
        format!("job {id}"),
        Style::default().fg(theme::MUTED),
    ));

    if app.history_detail_loading {
        spans.push(Span::styled(
            format!("   {} ", theme::spinner_frame(app.tick)),
            Style::default().fg(theme::ACCENT),
        ));
    }

    Line::from(spans)
}

fn acct_detail_lines(d: &AcctDetail) -> Vec<Line<'static>> {
    let badge = Span::styled(
        format!(" ● {} ", d.state),
        Style::default()
            .bg(acct_state_color(&d.state, &d.exit_code))
            .fg(theme::BADGE_FG)
            .add_modifier(Modifier::BOLD),
    );

    let used = d.max_rss.as_deref().unwrap_or("--");
    let req = if d.req_mem.is_empty() {
        "--"
    } else {
        &d.req_mem
    };

    let mut lines = vec![Line::from(badge), Line::from("")];
    lines.push(kv("User", d.user.clone()));
    if !d.account.is_empty() {
        lines.push(kv("Account", d.account.clone()));
    }
    lines.push(kv("Partition", d.partition.clone()));
    if !d.node_list.is_empty() {
        lines.push(kv("Nodes", d.node_list.clone()));
    }
    lines.push(kv("CPUs", d.alloc_cpus.clone()));
    lines.push(kv("Memory", format!("req {req}   used {used}")));
    if !d.total_cpu.is_empty() {
        lines.push(kv("CPU time", d.total_cpu.clone()));
    }
    lines.push(kv("Submitted", d.submit.clone()));
    lines.push(kv("Started", d.start.clone()));
    lines.push(kv("Ended", d.end.clone()));
    lines.push(kv("Elapsed", d.elapsed.clone()));
    lines.push(kv("Exit code", d.exit_code.clone()));
    if !d.work_dir.is_empty() {
        lines.push(kv("Work dir", d.work_dir.clone()));
    }

    lines
}

fn render_history_detail_logs(frame: &mut Frame, app: &App, detail: &AcctDetail, area: Rect) {
    let paths = SlurmParser::get_acct_log_paths(&detail.work_dir, &detail.job_id);
    let block = theme::panel("Logs", true);

    match read_tail_for_paths(paths, TAIL_BYTES) {
        LogRead::Lines { path, text } => {
            let content = format!("{path}\n{}\n{text}", "─".repeat(40));
            let total = content.lines().count();
            let offset = clamp_scroll(app.history_detail_scroll, total, block.inner(area).height);
            frame.render_widget(
                Paragraph::new(content)
                    .style(Style::default().fg(theme::FG))
                    .block(block)
                    .wrap(Wrap { trim: false })
                    .scroll((offset, 0)),
                area,
            );
        }
        LogRead::Empty(_) => render_placeholder(frame, block, area, "This job's log is empty"),
        LogRead::Missing(_) => {
            render_placeholder(frame, block, area, "No log file found for this job")
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

/// Truncate keeping the tail, so a shortened path still shows its filename.
fn truncate_left(s: &str, max_len: usize) -> String {
    let count = s.chars().count();
    if count <= max_len {
        return s.to_string();
    }
    let keep = max_len.saturating_sub(1);
    let tail: String = s.chars().skip(count - keep).collect();
    format!("…{tail}")
}

/// A Logs panel with the file path carried in the border, muted and tail-first
/// so the filename survives when the path is long.
fn logs_panel(path: &str, focused: bool, width: u16) -> Block<'static> {
    // Leave room for the " Logs · " label, the borders, and a trailing gap.
    let budget = (width as usize).saturating_sub(12).max(8);
    let shown = truncate_left(path, budget);
    theme::panel("Logs", focused).title(Span::styled(
        format!("· {shown} "),
        Style::default().fg(theme::MUTED),
    ))
}

/// Center a rect of fixed cell dimensions, clamped to the available area.
/// Use this for small popups whose content has a known size, so they don't
/// collapse on short terminals the way percentage sizing does.
fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let w = width.min(r.width);
    let h = height.min(r.height);
    Rect {
        x: r.x + r.width.saturating_sub(w) / 2,
        y: r.y + r.height.saturating_sub(h) / 2,
        width: w,
        height: h,
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
