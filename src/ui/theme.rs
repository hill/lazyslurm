use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding},
};

use crate::models::JobState;

// Accents
pub const ACCENT: Color = Color::Rgb(0x7C, 0x6B, 0xFF);
pub const ACCENT_PINK: Color = Color::Rgb(0xFF, 0x4F, 0xBF);

// Semantic state hues
pub const RUNNING: Color = Color::Rgb(0x12, 0xC7, 0x8F);
pub const PENDING: Color = Color::Rgb(0xE8, 0xC5, 0x47);
pub const COMPLETED: Color = Color::Rgb(0x00, 0xA4, 0xFF);
pub const FAILED: Color = Color::Rgb(0xFF, 0x57, 0x7D);
pub const CANCELLED: Color = Color::Rgb(0xD4, 0x6E, 0xFF);

// Neutrals
pub const FG: Color = Color::Rgb(0xD6, 0xD3, 0xDC); // primary text
pub const MUTED: Color = Color::Rgb(0x85, 0x83, 0x92); // secondary labels
pub const DIM_BORDER: Color = Color::Rgb(0x4D, 0x4C, 0x57); // unfocused borders
pub const BADGE_FG: Color = Color::Rgb(0x20, 0x1F, 0x26); // dark text on a filled badge
pub const SELECT_BG: Color = Color::Rgb(0x2D, 0x2C, 0x36); // selected row background

/// MiniDot braille spinner, a Charm staple. Stepped at half tick-rate so it
/// reads as a smooth ~5fps rather than a blur.
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn spinner_frame(tick: u64) -> &'static str {
    SPINNER[(tick as usize / 2) % SPINNER.len()]
}

pub fn state_color(state: &JobState) -> Color {
    match state {
        JobState::Running => RUNNING,
        JobState::Pending => PENDING,
        JobState::Completed => COMPLETED,
        JobState::Failed | JobState::Timeout | JobState::NodeFail => FAILED,
        JobState::Cancelled | JobState::Preempted => CANCELLED,
        JobState::Unknown(_) => MUTED,
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

/// Filled status pill, dark text on the state hue, with a leading dot.
pub fn state_badge(state: &JobState) -> Span<'static> {
    Span::styled(
        format!(" ● {} ", state_label(state)),
        Style::default()
            .bg(state_color(state))
            .fg(BADGE_FG)
            .add_modifier(Modifier::BOLD),
    )
}

/// A rounded panel. Focused panels glow in the accent, the rest sit back in a
/// dim grey so the eye always knows where it is (the lazygit move).
pub fn panel(title: &str, focused: bool) -> Block<'static> {
    let border = if focused { ACCENT } else { DIM_BORDER };
    let title_style = if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED).add_modifier(Modifier::BOLD)
    };

    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .padding(Padding::horizontal(1))
        .title(Span::styled(format!(" {title} "), title_style))
}

/// A `key label` hint with the key in accent and the label muted.
pub fn key_hint(key: &str, label: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            key.to_string(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {label}"), Style::default().fg(MUTED)),
    ]
}

fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::Rgb(mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

/// Sweep text from the accent purple to the accent pink, char by char. Used
/// for the empty-state logo.
pub fn gradient_line(text: &str) -> Line<'static> {
    let start = (0x7C, 0x6B, 0xFF);
    let end = (0xFF, 0x4F, 0xBF);
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
