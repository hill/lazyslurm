use crate::app::{App, AppState};
use crate::render_app;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
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
    }
}

async fn event_normal_state(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _)
        | (KeyCode::Char('Q'), _)
        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            return Ok(Some(()));
        }
        (KeyCode::Char('r'), _) => {
            app.refresh_jobs().await?;
        }
        (KeyCode::Up, _) => {
            app.select_previous_job();
        }
        (KeyCode::Down, _) => {
            app.select_next_job();
        }
        (KeyCode::Char('u'), _) => {
            app.state = AppState::UserSearchPopup;
        }
        (KeyCode::Char('c'), _) if app.selected_job.is_some() => {
            app.confirm_action = false;
            app.state = AppState::CancelJobPopup;
        }
        _ => {}
    }
    Ok(None)
}

async fn event_user_search_popup(
    app: &mut App,
    key: KeyEvent,
) -> Result<Option<()>, Box<dyn Error>> {
    match key.code {
        KeyCode::Enter => {
            if app.input.is_empty() {
                app.current_user = None;
            } else {
                app.current_user = Some(app.input.clone());
            }

            app.input.clear();
            app.state = AppState::Normal;

            app.refresh_jobs().await?;
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
    Ok(None)
}

async fn event_cancel_popup(app: &mut App, key: KeyEvent) -> Result<Option<()>, Box<dyn Error>> {
    match key.code {
        KeyCode::Char('y') => {
            app.confirm_action = true;
            app.state = AppState::Normal;
            app.refresh_jobs().await?;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.confirm_action = false;
            app.state = AppState::Normal;
            app.refresh_jobs().await?;
        }
        _ => {}
    }
    app.handle_cancel_popup().await?;
    Ok(None)
}

pub async fn run_event_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| render_app(frame, app))?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
            && let Ok(Some(())) = handle_key_event(app, key).await
        {
            return Ok(());
        }

        if app.should_refresh() {
            app.refresh_jobs().await?;
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
