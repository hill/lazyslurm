use crate::app::{ActiveTab, App, AppState, FocusPanel};
use crate::{panel_rects, render_app, tab_rects};
use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

pub async fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    match app.state {
        AppState::Normal => event_normal_state(app, key).await,
        AppState::UserSearchPopup => event_user_search_popup(app, key).await,
        AppState::CancelJobPopup => event_cancel_popup(app, key).await,
        AppState::PartitionSearchPopup => event_partition_search_popup(app, key).await,
        AppState::Fullscreen => event_fullscreen(app, key),
        AppState::HistoryDetail => event_history_detail(app, key),
        AppState::FilterInput => event_filter_input(app, key),
    }
}

/// Live filter typing. Every keystroke re-filters the in-memory list, so it is
/// instant. Esc drops the filter; Enter keeps it and returns to navigation.
fn event_filter_input(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(Some(())),
        (KeyCode::Esc, _) => app.clear_filter(),
        (KeyCode::Enter, _) => app.commit_filter(),
        (KeyCode::Backspace, _) => app.filter_backspace(),
        (KeyCode::Char(c), _) => app.filter_push(c),
        _ => {}
    }
    Ok(None)
}

fn event_history_detail(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    const PAGE: u16 = 10;
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(Some(())),
        (KeyCode::Esc, _) | (KeyCode::Char('q'), _) | (KeyCode::Char('Q'), _) => {
            app.close_history_detail();
        }
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.history_detail_scroll_up(1),
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.history_detail_scroll_down(1),
        (KeyCode::PageUp, _) => app.history_detail_scroll_up(PAGE),
        (KeyCode::PageDown, _) => app.history_detail_scroll_down(PAGE),
        _ => {}
    }
    Ok(None)
}

pub async fn handle_text_event(app: &mut App, key: KeyEvent) -> Option<Option<String>> {
    match key.code {
        KeyCode::Enter => {
            if app.input.is_empty() {
                return Some(None);
            } else {
                return Some(Some(app.input.clone()));
            }
        }
        KeyCode::Esc => {
            app.input.clear();
            app.state = AppState::Normal;
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        _ => {}
    }
    None
}

pub fn reset_popup_state_to_normal(app: &mut App) {
    app.input.clear();
    app.state = AppState::Normal;
    app.invalidate_and_refresh();
    // History is user-scoped, so a filter change should reflect there too.
    app.refresh_active_tab();
}

async fn event_normal_state(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    const PAGE: u16 = 10;
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _)
        | (KeyCode::Char('Q'), _)
        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            return Ok(Some(()));
        }
        (KeyCode::Char('r'), _) => {
            app.start_refresh();
            app.refresh_active_tab();
        }
        (KeyCode::Tab, _) => app.next_tab(),
        (KeyCode::BackTab, _) => app.prev_tab(),
        (KeyCode::Char('1'), _) => app.switch_tab(ActiveTab::Jobs),
        (KeyCode::Char('2'), _) => app.switch_tab(ActiveTab::Nodes),
        (KeyCode::Char('3'), _) => app.switch_tab(ActiveTab::Partitions),
        (KeyCode::Char('4'), _) => app.switch_tab(ActiveTab::History),
        (KeyCode::Left, _) | (KeyCode::Char('h'), _) if app.active_tab == ActiveTab::Jobs => {
            app.focus_left();
        }
        (KeyCode::Right, _) | (KeyCode::Char('l'), _) if app.active_tab == ActiveTab::Jobs => {
            app.focus_right();
        }
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
            if app.active_tab != ActiveTab::Jobs {
                app.list_prev();
            } else if app.focus == FocusPanel::Jobs {
                app.select_previous_job();
            } else {
                app.focus_up();
            }
        }
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
            if app.active_tab != ActiveTab::Jobs {
                app.list_next();
            } else if app.focus == FocusPanel::Jobs {
                app.select_next_job();
            } else {
                app.focus_down();
            }
        }
        // Inline scroll for the right-hand panes; arrows are reserved for focus.
        (KeyCode::PageUp, _) => app.scroll_focused_up(PAGE),
        (KeyCode::PageDown, _) => app.scroll_focused_down(PAGE),
        (KeyCode::Enter, _) if app.active_tab == ActiveTab::Jobs => {
            app.open_fullscreen();
        }
        (KeyCode::Enter, _) if app.active_tab == ActiveTab::History => {
            app.open_history_detail();
        }
        (KeyCode::Char('u'), _) => {
            app.state = AppState::UserSearchPopup;
        }
        (KeyCode::Char('p'), _) => {
            app.state = AppState::PartitionSearchPopup;
        }
        (KeyCode::Char('c'), _) if app.active_tab == ActiveTab::Jobs => {
            app.open_cancel_popup();
        }
        (KeyCode::Char('/'), _) if app.active_tab == ActiveTab::Jobs => {
            app.open_filter();
        }
        // Esc clears an active filter without entering filter mode first.
        (KeyCode::Esc, _) if app.active_tab == ActiveTab::Jobs && !app.filter_query.is_empty() => {
            app.clear_filter();
        }
        (KeyCode::Char('P'), _) if app.active_tab == ActiveTab::Jobs => {
            app.toggle_pin();
        }
        _ => {}
    }
    Ok(None)
}

fn event_fullscreen(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    const PAGE: u16 = 10;
    let on_jobs = app.fullscreen_panel == FocusPanel::Jobs;
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(Some(())),
        (KeyCode::Esc, _) | (KeyCode::Char('q'), _) | (KeyCode::Char('Q'), _) => {
            app.close_fullscreen();
        }
        // On the Jobs pane the arrows move the selection; elsewhere they scroll.
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) if on_jobs => app.select_previous_job(),
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) if on_jobs => app.select_next_job(),
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.fullscreen_scroll_up(1),
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.fullscreen_scroll_down(1),
        (KeyCode::PageUp, _) => app.fullscreen_scroll_up(PAGE),
        (KeyCode::PageDown, _) => app.fullscreen_scroll_down(PAGE),
        (KeyCode::Char('G'), _) | (KeyCode::Char('g'), _) => app.fullscreen_follow(),
        _ => {}
    }
    Ok(None)
}

async fn event_user_search_popup(
    app: &mut App,
    key: KeyEvent,
) -> Result<Option<()>, Box<dyn Error>> {
    let user_search = handle_text_event(app, key).await;
    if let Some(user) = user_search {
        app.current_user = user;
        reset_popup_state_to_normal(app);
    }
    Ok(None)
}

async fn event_partition_search_popup(
    app: &mut App,
    key: KeyEvent,
) -> Result<Option<()>, Box<dyn Error>> {
    let partition_search = handle_text_event(app, key).await;
    if let Some(partition) = partition_search {
        app.current_partition = partition;
        reset_popup_state_to_normal(app);
    }
    Ok(None)
}

async fn event_cancel_popup(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    match key.code {
        KeyCode::Char('y') => {
            app.confirm_cancel().await?;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.dismiss_cancel_popup();
        }
        _ => {}
    }
    Ok(None)
}

pub async fn run_event_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        app.drain_events();
        terminal.draw(|frame| render_app(frame, app))?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if let Ok(Some(())) = handle_key_event(app, key).await {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    let size = terminal.size()?;
                    handle_mouse_event(app, mouse, Rect::new(0, 0, size.width, size.height));
                }
                _ => {}
            }
        }

        if app.should_refresh() {
            app.start_refresh();
            app.refresh_active_tab();
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            app.tick = app.tick.wrapping_add(1);
        }
    }
}

/// Click a tab to switch to it, click a panel to focus it, wheel to scroll the
/// focused panel. Reuses `tab_rects`/`panel_rects` so the hit-testing can't
/// drift from what's drawn.
fn handle_mouse_event(app: &mut App, mouse: MouseEvent, area: Rect) {
    if app.state != AppState::Normal {
        return;
    }

    // Clicking a tab switches to it, on every tab.
    if let MouseEventKind::Down(_) = mouse.kind
        && let Some(tab) = tab_rects(area).hit(mouse.column, mouse.row)
    {
        app.switch_tab(tab);
        return;
    }

    // Panel hit-testing and the focus model only exist on the Jobs dashboard.
    // On the cluster tabs the wheel just moves the list selection.
    if app.active_tab != ActiveTab::Jobs {
        match mouse.kind {
            MouseEventKind::ScrollDown => app.list_next(),
            MouseEventKind::ScrollUp => app.list_prev(),
            _ => {}
        }
        return;
    }

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // status bar (title + tabs)
            Constraint::Min(0),    // panels
            Constraint::Length(3), // help bar
        ])
        .split(area)[1];

    match mouse.kind {
        MouseEventKind::Down(_) => {
            if let Some(panel) = panel_rects(main).hit(mouse.column, mouse.row) {
                app.focus = panel;
            }
        }
        MouseEventKind::ScrollDown if app.focus == FocusPanel::Jobs => app.select_next_job(),
        MouseEventKind::ScrollUp if app.focus == FocusPanel::Jobs => app.select_previous_job(),
        MouseEventKind::ScrollDown => app.scroll_focused_down(1),
        MouseEventKind::ScrollUp => app.scroll_focused_up(1),
        _ => {}
    }
}
