use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io, time::Duration};
use tokio::time::sleep;

use lazyslurm::ui::{components::render_app, App};
use lazyslurm::slurm::SlurmCommands;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Check if SLURM is available
    if !SlurmCommands::check_slurm_available() {
        eprintln!("Error: SLURM commands not found. Please make sure SLURM is installed and available in PATH.");
        eprintln!("Required commands: squeue, scontrol, scancel");
        std::process::exit(1);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("Application error: {err:?}");
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    // Initial refresh
    app.refresh_jobs().await?;

    loop {
        // Draw UI
        terminal.draw(|frame| render_app(frame, app))?;

        // Handle events with timeout
        let timeout = Duration::from_millis(250);
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Only handle KeyEventKind::Press to avoid duplicate events
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('r') => {
                        app.refresh_jobs().await?;
                    }
                    KeyCode::Up => {
                        app.select_previous_job();
                    }
                    KeyCode::Down => {
                        app.select_next_job();
                    }
                    KeyCode::Char('c') => {
                        if let Err(e) = app.cancel_selected_job().await {
                            app.error_message = Some(format!("Failed to cancel job: {}", e));
                        }
                    }
                    _ => {}
                }
            }
        }

        // Auto refresh if needed
        if app.should_refresh() {
            app.refresh_jobs().await?;
        }

        // Small delay to prevent excessive CPU usage
        sleep(Duration::from_millis(50)).await;
    }
}
