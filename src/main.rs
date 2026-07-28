use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{error::Error, io};

use lazyslurm::slurm::check_slurm_available;
use lazyslurm::ui::theme::{self, load, load::ThemeRegistry};
use lazyslurm::ui::{App, events};
use lazyslurm::utils::config;

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
  Tab / 1-5: switch between Jobs, Nodes, Partitions, History, Usage
  ↑/↓ or j/k: navigate the current list
  r: refresh
  c: cancel selected job (Jobs tab)
  T: pick a colour theme

Notes:
  - Required tools: squeue, scontrol, scancel.
  - Optional tools power the extra tabs: sinfo (Nodes, Partitions) and
    sacct (History). The History tab needs slurmdbd accounting enabled.
  - Config lives at ~/.config/lazyslurm/config.toml, custom themes at
    ~/.config/lazyslurm/themes/*.toml.
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
        short = 'a',
        long = "all",
        help = "Show jobs for all users (overrides --user)"
    )]
    all: bool,

    #[arg(
        long = "json",
        help = "Fetch jobs once, print as JSON to stdout, and exit (headless mode)"
    )]
    json: bool,

    #[arg(
        long = "no-update-check",
        help = "Skip the startup check for a newer release (also LAZYSLURM_NO_UPDATE_CHECK)"
    )]
    no_update_check: bool,

    #[arg(
        long = "theme",
        value_name = "NAME",
        help = "Colour theme (also LAZYSLURM_THEME; see --list-themes)"
    )]
    theme: Option<String>,

    #[arg(long = "list-themes", help = "List the available themes and exit")]
    list_themes: bool,

    #[arg(
        long = "print-theme",
        help = "Print the active theme as TOML and exit (a starting point for your own)"
    )]
    print_theme: bool,
}

/// Everything the theme system decided at startup, resolved before the
/// terminal is touched so a bad theme can never strand the alternate screen.
struct ThemeChoice {
    registry: ThemeRegistry,
    name: String,
    warning: Option<String>,
}

fn resolve_theme(cli: Option<String>) -> ThemeChoice {
    let (registry, mut warnings) = ThemeRegistry::load();
    let (config, config_warning) = config::load();
    warnings.extend(config_warning);

    let env = std::env::var("LAZYSLURM_THEME").ok();
    let requested =
        load::resolve_theme_name(cli.as_deref(), env.as_deref(), config.theme.as_deref())
            .to_string();

    let name = match registry.get(&requested) {
        Some(theme) => {
            theme::set(theme);
            requested
        }
        None => {
            // An explicit --theme is a typo worth stopping for. A stale name in
            // the config or environment is not worth refusing to start over.
            if cli.is_some() {
                eprintln!("Error: no theme called '{requested}'.");
                eprintln!("Available: {}", theme_names(&registry).join(", "));
                std::process::exit(2);
            }
            warnings.push(format!("no theme called '{requested}', using the default"));
            load::DEFAULT_THEME.to_string()
        }
    };

    ThemeChoice {
        registry,
        name,
        warning: (!warnings.is_empty()).then(|| warnings.join("; ")),
    }
}

fn theme_names(registry: &ThemeRegistry) -> Vec<String> {
    registry
        .entries()
        .iter()
        .map(|entry| entry.name.clone())
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse CLI first so --version/-V and --help exit early
    let cli = Cli::parse();

    // Resolve the theme before the terminal is touched, so an unknown name
    // exits plainly instead of from inside the alternate screen.
    let theme_choice = resolve_theme(cli.theme);

    if cli.list_themes {
        for entry in theme_choice.registry.entries() {
            let marker = if entry.name == theme_choice.name {
                "*"
            } else {
                " "
            };
            let origin = if entry.user { "  (custom)" } else { "" };
            println!("{marker} {}{origin}", entry.name);
        }
        return Ok(());
    }

    if cli.print_theme {
        let theme = theme_choice
            .registry
            .get(&theme_choice.name)
            .unwrap_or_else(theme::current);
        println!("# lazyslurm theme '{}'", theme_choice.name);
        print!("{}", toml::to_string_pretty(&theme)?);
        return Ok(());
    }

    // Check if SLURM is available
    if !check_slurm_available() {
        eprintln!(
            "Error: slurm commands not found. Please make sure slurm is installed and available in PATH."
        );
        eprintln!("Required commands: squeue, scontrol, scancel");
        std::process::exit(1);
    }

    if cli.json {
        return run_headless(cli.user, cli.partition, cli.all).await;
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::with_cli(cli.user, cli.partition, cli.all);
    app.set_themes(theme_choice.registry, theme_choice.name);
    app.theme_warning = theme_choice.warning;

    let update_check_disabled =
        cli.no_update_check || std::env::var_os("LAZYSLURM_NO_UPDATE_CHECK").is_some();
    if !update_check_disabled {
        app.start_update_check();
    }

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
    all: bool,
) -> Result<(), Box<dyn Error>> {
    let mut app = App::with_cli(user, partition, all);
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
