use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{error::Error, io, time::Duration};
use tokio::time::sleep;

use lazyslurm::slurm::SlurmCommands;
use lazyslurm::ui::{App, components::render_app};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "A terminal UI for monitoring and managing Slurm jobs.",
    long_about = "A terminal UI for monitoring and managing Slurm jobs.",
    before_help = r#"

░██                                             ░██                                     
░██                                             ░██                                     
░██  ░██████   ░█████████ ░██    ░██  ░███████  ░██ ░██    ░██ ░██░████ ░█████████████  
░██       ░██       ░███  ░██    ░██ ░██        ░██ ░██    ░██ ░███     ░██   ░██   ░██ 
░██  ░███████     ░███    ░██    ░██  ░███████  ░██ ░██    ░██ ░██      ░██   ░██   ░██ 
░██ ░██   ░██   ░███      ░██   ░███        ░██ ░██ ░██   ░███ ░██      ░██   ░██   ░██ 
░██  ░█████░██ ░█████████  ░█████░██  ░███████  ░██  ░█████░██ ░██      ░██   ░██   ░██ 
                                 ░██                                                    
                           ░███████                                                     
                                                                                        

"#,
    after_help = r#"Keyboard shortcuts:
  q: quit
  ↑/↓ or j/k: navigate jobs
  r: refresh jobs
  c: cancel selected job

Notes:
  - SLURM tools required for normal operation: squeue, scontrol, scancel.
"#
)]
struct Cli {
    #[arg(
        short = 'u',
        long = "user",
        help = "Filter to a specific user (default: $USER)"
    )]
    user: Option<String>,

    #[arg(
        short = 'p',
        long = "partition",
        help = "Filter to a specific partition (e.g., gpu)"
    )]
    partition: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse CLI first so --version/-V and --help exit early
    let cli = Cli::parse();

    // Check if SLURM is available
    if !SlurmCommands::check_slurm_available() {
        eprintln!(
            "Error: slurm commands not found. Please make sure slurm is installed and available in PATH."
        );
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
    let mut app = App::with_cli(cli.user, cli.partition);
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
        if crossterm::event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            // Only handle KeyEventKind::Press to avoid duplicate events
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    return Ok(());
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
                (KeyCode::Char('c'), _) => {
                    if app.selected_job.is_some() {
                        app.show_confirm_popup = true;
                        app.confirm_action = false;
                    }
                }
                (KeyCode::Char('y'), _) if app.show_confirm_popup => {
                    app.confirm_action = true;
                    app.show_confirm_popup = false;
                }
                (KeyCode::Char('n'), _) | (KeyCode::Esc, _) if app.show_confirm_popup => {
                    app.show_confirm_popup = false;
                    app.confirm_action = false;
                }

                _ => {}
            }
        }

        app.handle_confirm_action().await?;

        // Auto refresh if needed
        if app.should_refresh() {
            app.refresh_jobs().await?;
        }

        // Small delay to prevent excessive CPU usage
        sleep(Duration::from_millis(50)).await;
    }
}
