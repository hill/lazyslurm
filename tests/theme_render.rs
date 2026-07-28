//! Rendering under a non-default theme. These mutate the global palette, so
//! they live in their own integration binary: every `tests/*.rs` file is a
//! separate process, which keeps them from racing the threaded suites. Within
//! this binary they take a lock, since the tests here still share a process.

use std::sync::{Mutex, PoisonError};

use lazyslurm::models::{Job, JobState};
use lazyslurm::ui::theme::{self, Theme, builtin};
use lazyslurm::ui::{ActiveTab, App, AppState, render_app};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, style::Color};

static PALETTE: Mutex<()> = Mutex::new(());

/// Draw under `theme`, then put the default back. A failing test poisons the
/// lock but leaves the palette restored, so the rest still run honestly.
fn with_theme<T>(theme: Theme, body: impl FnOnce() -> T) -> T {
    let _guard = PALETTE.lock().unwrap_or_else(PoisonError::into_inner);
    theme::set(theme);
    let result = body();
    theme::set(builtin::LAZYSLURM);
    result
}

fn draw(app: &App) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|frame| render_app(frame, app)).unwrap();
    terminal.backend().buffer().clone()
}

fn app_with_a_job() -> App {
    let mut app = App::new();
    let job = Job::new(
        "48201".into(),
        "train".into(),
        "alice".into(),
        JobState::Running,
    );
    app.job_list.jobs = vec![job.clone()];
    app.selected_job = Some(job);
    app
}

/// A layout or unicode-width panic under one palette but not another would be
/// surprising, but it costs almost nothing to rule out.
#[test]
fn every_builtin_renders_every_tab() {
    for (name, palette) in builtin::BUILTINS {
        with_theme(*palette, || {
            for tab in ActiveTab::ALL {
                let mut app = app_with_a_job();
                app.active_tab = tab;
                assert!(
                    !draw(&app).content().is_empty(),
                    "{name} drew nothing on {tab:?}"
                );
            }
        });
    }
}

/// The rest of the suite asserts on glyphs, so this is the one place we prove
/// a theme's colours actually reach the buffer.
fn brand_badge_cell(buffer: &Buffer) -> (Color, Color) {
    let cell = buffer.cell((1, 0)).expect("status bar should be drawn");
    (cell.fg, cell.bg)
}

#[test]
fn the_brand_badge_is_filled_with_the_active_accent() {
    let (fg, bg) = with_theme(builtin::NORD, || brand_badge_cell(&draw(&app_with_a_job())));
    assert_eq!(bg, builtin::NORD.accent);
    assert_eq!(fg, builtin::NORD.badge_fg);
}

/// The root canvas is painted above the fullscreen early returns, so a light
/// theme has to be opaque in every view, not just the dashboard.
#[test]
fn a_light_theme_paints_the_root_background_in_every_view() {
    let light = builtin::get("gruvbox-light").unwrap();
    let expected = light.bg.unwrap();

    with_theme(light, || {
        for state in [
            AppState::Normal,
            AppState::Fullscreen,
            AppState::HistoryDetail,
            AppState::RawLog,
        ] {
            let mut app = app_with_a_job();
            app.state = state;
            // The last row's last cell sits outside every widget.
            let bg = draw(&app).cell((119, 39)).unwrap().bg;
            assert_eq!(bg, expected, "{state:?} left the canvas unpainted");
        }
    });
}

/// `Clear` resets cells to terminal default, so without a repaint every popup
/// would punch a hole through a themed background.
#[test]
fn popups_are_opaque_under_a_light_theme() {
    let light = builtin::get("catppuccin-latte").unwrap();
    let expected = light.bg.unwrap();

    with_theme(light, || {
        let mut app = app_with_a_job();
        app.open_cancel_popup();
        assert_eq!(app.state, AppState::CancelJobPopup);

        // Inside the 44x7 popup centred on a 120x40 screen, clear of its border.
        let bg = draw(&app).cell((60, 20)).unwrap().bg;
        assert_eq!(bg, expected, "the popup showed the terminal through");
    });
}

/// The default stays transparent so it sits over whatever the terminal is
/// doing, including a translucent one.
#[test]
fn the_default_theme_leaves_the_terminal_background_alone() {
    let bg = with_theme(builtin::LAZYSLURM, || {
        draw(&app_with_a_job()).cell((119, 39)).unwrap().bg
    });
    assert_eq!(bg, Color::Reset);
}

/// Browsing the picker is the preview: the highlighted theme is applied at
/// once, and dismissing has to put the old one back.
///
/// Nothing here calls `commit_theme`, which would write the real config file.
#[test]
fn browsing_the_picker_previews_and_dismissing_reverts() {
    with_theme(builtin::LAZYSLURM, || {
        let mut app = app_with_a_job();
        app.open_theme_picker();
        assert_eq!(app.state, AppState::ThemePicker);
        assert_eq!(theme::current(), builtin::LAZYSLURM);

        app.theme_picker_next();
        let previewed = builtin::BUILTINS[1].1;
        assert_eq!(theme::current(), previewed, "moving down should apply");
        assert_eq!(app.theme_name, "lazyslurm", "preview is not a commitment");

        // The list is drawn with the previewed palette behind it.
        let text: String = draw(&app)
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains(builtin::BUILTINS[1].0), "{text}");

        app.cancel_theme();
        assert_eq!(app.state, AppState::Normal);
        assert_eq!(theme::current(), builtin::LAZYSLURM, "esc should revert");
    });
}

/// Selection is clamped at both ends rather than wrapping, matching every other
/// list in the app.
#[test]
fn the_picker_selection_stops_at_both_ends() {
    with_theme(builtin::LAZYSLURM, || {
        let mut app = app_with_a_job();
        app.open_theme_picker();

        app.theme_picker_prev();
        assert_eq!(app.theme_picker_index, 0);

        for _ in 0..builtin::BUILTINS.len() + 5 {
            app.theme_picker_next();
        }
        assert_eq!(app.theme_picker_index, builtin::BUILTINS.len() - 1);

        app.cancel_theme();
    });
}

/// A theme built from named colours cannot be interpolated, so the empty-state
/// logo falls back to flat accent rather than guessing.
#[test]
fn a_named_colour_theme_still_renders_the_empty_state() {
    let named = Theme {
        accent: Color::Blue,
        accent_alt: Color::Magenta,
        ..builtin::LAZYSLURM
    };

    let text = with_theme(named, || {
        draw(&App::new())
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    });
    assert!(text.contains("L A Z Y S L U R M"), "logo should still draw");
}
