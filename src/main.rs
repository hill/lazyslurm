use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{error::Error, io};

use lazyslurm::slurm::check_slurm_available;
use lazyslurm::ui::{App, events};

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

    #[arg(
        long = "json",
        help = "Fetch jobs once, print as JSON to stdout, and exit (headless mode)"
    )]
    json: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse CLI first so --version/-V and --help exit early
    let cli = Cli::parse();

    // Check if SLURM is available
    if !check_slurm_available() {
        eprintln!(
            "Error: slurm commands not found. Please make sure slurm is installed and available in PATH."
        );
        eprintln!("Required commands: squeue, scontrol, scancel");
        std::process::exit(1);
    }

    if cli.json {
        return run_headless(cli.user, cli.partition).await;
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

async fn run_headless(
    user: Option<String>,
    partition: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let mut app = App::with_cli(user, partition);
    app.refresh_jobs().await?;

    if let Some(err) = &app.error_message {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }

    let json = serde_json::to_string_pretty(&app.job_list)?;
    println!("{json}");
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    // Initial refresh
    app.refresh_jobs().await?;

    events::run_event_loop(app, terminal).await?;

    Ok(())
}
