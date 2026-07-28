pub mod builtin;
pub mod load;

use std::sync::{PoisonError, RwLock};

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding},
};

use crate::models::JobState;

/// A complete palette. Every colour the UI can draw lives here, so swapping a
/// theme is replacing one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Theme {
    /// Focused borders, brand badge, key hints, cursors.
    pub accent: Color,
    /// Job-name pill, update badge, pinned star.
    pub accent_alt: Color,

    pub running: Color,
    pub pending: Color,
    pub completed: Color,
    pub failed: Color,
    pub cancelled: Color,

    /// Primary text.
    pub fg: Color,
    /// Secondary labels, placeholders, help labels.
    pub muted: Color,
    /// Unfocused borders and hint separators.
    pub border: Color,
    /// Text drawn *on* a filled badge, so it wants to be the theme's own
    /// background tone: dark on dark themes, light on light ones.
    pub badge_fg: Color,
    /// Selected row background.
    pub select_bg: Color,
    /// Focused-column background.
    pub column_bg: Color,

    /// The canvas. `None` leaves the terminal's own background showing
    /// through, which is what the default theme wants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<Color>,
}

static CURRENT: RwLock<Theme> = RwLock::new(builtin::LAZYSLURM);

/// The active palette. One uncontended read lock and a ~56 byte copy, cheap
/// enough to call per span.
#[inline]
pub fn current() -> Theme {
    // Poisoning needs a panic while the lock is held, and we never hold it
    // across anything fallible. Recovering beats a second panic on the unwind
    // path with the alternate screen still up.
    *CURRENT.read().unwrap_or_else(PoisonError::into_inner)
}

/// Swap the active palette. Takes effect on the next draw.
pub fn set(theme: Theme) {
    *CURRENT.write().unwrap_or_else(PoisonError::into_inner) = theme;
}

/// MiniDot braille spinner, a Charm staple. Stepped at half tick-rate so it
/// reads as a smooth ~5fps rather than a blur.
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn spinner_frame(tick: u64) -> &'static str {
    SPINNER[(tick as usize / 2) % SPINNER.len()]
}

pub fn state_color(state: &JobState) -> Color {
    let t = current();
    match state {
        JobState::Running => t.running,
        JobState::Pending => t.pending,
        JobState::Completed => t.completed,
        JobState::Failed | JobState::Timeout | JobState::NodeFail => t.failed,
        JobState::Cancelled | JobState::Preempted => t.cancelled,
        JobState::Unknown(_) => t.muted,
    }
}

pub fn state_label(state: &JobState) -> String {
    match state {
        JobState::Running => "RUNNING".into(),
        JobState::Pending => "PENDING".into(),
        JobState::Completed => "DONE".into(),
        JobState::Cancelled => "CANCELLED".into(),
        JobState::Failed => "FAILED".into(),
        JobState::Timeout => "TIMEOUT".into(),
        JobState::NodeFail => "NODEFAIL".into(),
        JobState::Preempted => "PREEMPT".into(),
        JobState::Unknown(s) => s.to_uppercase(),
    }
}

/// Filled status pill, badge text on the state hue, with a leading dot.
pub fn state_badge(state: &JobState) -> Span<'static> {
    Span::styled(
        format!(" ● {} ", state_label(state)),
        Style::default()
            .bg(state_color(state))
            .fg(current().badge_fg)
            .add_modifier(Modifier::BOLD),
    )
}

/// A rounded panel. Focused panels glow in the accent, the rest sit back in a
/// dim grey so the eye always knows where it is (the lazygit move).
pub fn panel(title: &str, focused: bool) -> Block<'static> {
    let t = current();
    let border = if focused { t.accent } else { t.border };
    let title_fg = if focused { t.accent } else { t.muted };

    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .padding(Padding::horizontal(1))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(title_fg).add_modifier(Modifier::BOLD),
        ))
}

/// A titled popup frame, always accented since a popup is always what has
/// focus.
pub fn popup_block(title: &str) -> Block<'static> {
    let accent = current().accent;
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
}

/// An untitled rounded frame for the chrome that never takes focus: the help
/// bar and the right-column header.
pub fn bar_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(current().border))
}

/// A `key label` hint with the key in accent and the label muted.
pub fn key_hint(key: &str, label: &str) -> Vec<Span<'static>> {
    let t = current();
    vec![
        Span::styled(
            key.to_string(),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {label}"), Style::default().fg(t.muted)),
    ]
}

/// True RGB for a colour, where one exists. Named colours and the first 16
/// indexed slots are whatever the terminal says they are, so they have no
/// answer here.
pub fn rgb_of(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        // The 6x6x6 colour cube.
        Color::Indexed(i @ 16..=231) => {
            const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
            let i = i - 16;
            Some((
                LEVELS[(i / 36) as usize],
                LEVELS[(i % 36 / 6) as usize],
                LEVELS[(i % 6) as usize],
            ))
        }
        // The greyscale ramp.
        Color::Indexed(i @ 232..=255) => {
            let v = 8 + 10 * (i - 232);
            Some((v, v, v))
        }
        _ => None,
    }
}

fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::Rgb(mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

/// Sweep text from the accent to the alternate accent, char by char. Used for
/// the empty-state logo.
///
/// A theme built from named colours has no RGB to interpolate between, so it
/// falls back to flat accent rather than guessing what the terminal means by
/// "blue".
pub fn gradient_line(text: &str) -> Line<'static> {
    let t = current();
    let (Some(start), Some(end)) = (rgb_of(t.accent), rgb_of(t.accent_alt)) else {
        return Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ));
    };

    let chars: Vec<char> = text.chars().collect();
    let last = chars.len().saturating_sub(1).max(1) as f32;

    let spans = chars
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            Span::styled(
                c.to_string(),
                Style::default()
                    .fg(lerp(start, end, i as f32 / last))
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect::<Vec<_>>();

    Line::from(spans)
}
