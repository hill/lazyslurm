use ratatui::style::Color;

use super::Theme;

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Every built-in theme, in picker order. The first entry is the default.
pub const BUILTINS: &[(&str, Theme)] = &[
    ("lazyslurm", LAZYSLURM),
    ("gruvbox-dark", GRUVBOX_DARK),
    ("gruvbox-light", GRUVBOX_LIGHT),
    ("catppuccin-mocha", CATPPUCCIN_MOCHA),
    ("catppuccin-latte", CATPPUCCIN_LATTE),
    ("nord", NORD),
    ("dracula", DRACULA),
    ("tokyonight", TOKYONIGHT),
    ("tokyonight-day", TOKYONIGHT_DAY),
];

pub fn get(name: &str) -> Option<Theme> {
    BUILTINS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, theme)| *theme)
}

/// The original scheme. Leaves `bg` unset so the app stays transparent over
/// whatever the terminal is doing.
pub const LAZYSLURM: Theme = Theme {
    accent: rgb(0x7C6BFF),
    accent_alt: rgb(0xFF4FBF),
    running: rgb(0x12C78F),
    pending: rgb(0xE8C547),
    completed: rgb(0x00A4FF),
    failed: rgb(0xFF577D),
    cancelled: rgb(0xD46EFF),
    fg: rgb(0xD6D3DC),
    muted: rgb(0x858392),
    border: rgb(0x4D4C57),
    badge_fg: rgb(0x201F26),
    select_bg: rgb(0x2D2C36),
    column_bg: rgb(0x332E52),
    bg: None,
};

pub const GRUVBOX_DARK: Theme = Theme {
    accent: rgb(0xFE8019),
    accent_alt: rgb(0xD3869B),
    running: rgb(0xB8BB26),
    pending: rgb(0xFABD2F),
    completed: rgb(0x83A598),
    failed: rgb(0xFB4934),
    cancelled: rgb(0xD3869B),
    fg: rgb(0xEBDBB2),
    muted: rgb(0x928374),
    border: rgb(0x504945),
    badge_fg: rgb(0x282828),
    select_bg: rgb(0x3C3836),
    column_bg: rgb(0x504945),
    bg: Some(rgb(0x282828)),
};

pub const GRUVBOX_LIGHT: Theme = Theme {
    accent: rgb(0xAF3A03),
    accent_alt: rgb(0x8F3F71),
    running: rgb(0x79740E),
    pending: rgb(0xB57614),
    completed: rgb(0x076678),
    failed: rgb(0x9D0006),
    cancelled: rgb(0x8F3F71),
    fg: rgb(0x3C3836),
    muted: rgb(0x7C6F64),
    border: rgb(0xD5C4A1),
    badge_fg: rgb(0xFBF1C7),
    select_bg: rgb(0xEBDBB2),
    column_bg: rgb(0xD5C4A1),
    bg: Some(rgb(0xFBF1C7)),
};

pub const CATPPUCCIN_MOCHA: Theme = Theme {
    accent: rgb(0xCBA6F7),
    accent_alt: rgb(0xF5C2E7),
    running: rgb(0xA6E3A1),
    pending: rgb(0xF9E2AF),
    completed: rgb(0x89B4FA),
    failed: rgb(0xF38BA8),
    cancelled: rgb(0xB4BEFE),
    fg: rgb(0xCDD6F4),
    muted: rgb(0x6C7086),
    border: rgb(0x45475A),
    badge_fg: rgb(0x1E1E2E),
    select_bg: rgb(0x313244),
    column_bg: rgb(0x45475A),
    bg: Some(rgb(0x1E1E2E)),
};

pub const CATPPUCCIN_LATTE: Theme = Theme {
    accent: rgb(0x8839EF),
    accent_alt: rgb(0xEA76CB),
    running: rgb(0x40A02B),
    pending: rgb(0xDF8E1D),
    completed: rgb(0x1E66F5),
    failed: rgb(0xD20F39),
    cancelled: rgb(0x7287FD),
    fg: rgb(0x4C4F69),
    muted: rgb(0x6C6F85),
    border: rgb(0xBCC0CC),
    badge_fg: rgb(0xEFF1F5),
    select_bg: rgb(0xCCD0DA),
    column_bg: rgb(0xBCC0CC),
    bg: Some(rgb(0xEFF1F5)),
};

pub const NORD: Theme = Theme {
    accent: rgb(0x88C0D0),
    accent_alt: rgb(0xB48EAD),
    running: rgb(0xA3BE8C),
    pending: rgb(0xEBCB8B),
    completed: rgb(0x81A1C1),
    failed: rgb(0xBF616A),
    cancelled: rgb(0xB48EAD),
    fg: rgb(0xD8DEE9),
    muted: rgb(0x7B88A1),
    border: rgb(0x434C5E),
    badge_fg: rgb(0x2E3440),
    select_bg: rgb(0x3B4252),
    column_bg: rgb(0x434C5E),
    bg: Some(rgb(0x2E3440)),
};

pub const DRACULA: Theme = Theme {
    accent: rgb(0xBD93F9),
    accent_alt: rgb(0xFF79C6),
    running: rgb(0x50FA7B),
    pending: rgb(0xF1FA8C),
    completed: rgb(0x8BE9FD),
    failed: rgb(0xFF5555),
    cancelled: rgb(0xFFB86C),
    fg: rgb(0xF8F8F2),
    muted: rgb(0x6272A4),
    border: rgb(0x44475A),
    badge_fg: rgb(0x282A36),
    select_bg: rgb(0x44475A),
    column_bg: rgb(0x3C3F58),
    bg: Some(rgb(0x282A36)),
};

pub const TOKYONIGHT: Theme = Theme {
    accent: rgb(0x7AA2F7),
    accent_alt: rgb(0xBB9AF7),
    running: rgb(0x9ECE6A),
    pending: rgb(0xE0AF68),
    completed: rgb(0x7DCFFF),
    failed: rgb(0xF7768E),
    cancelled: rgb(0xBB9AF7),
    fg: rgb(0xC0CAF5),
    muted: rgb(0x565F89),
    border: rgb(0x292E42),
    badge_fg: rgb(0x1A1B26),
    select_bg: rgb(0x292E42),
    column_bg: rgb(0x343A55),
    bg: Some(rgb(0x1A1B26)),
};

pub const TOKYONIGHT_DAY: Theme = Theme {
    accent: rgb(0x2E7DE9),
    accent_alt: rgb(0x9854F1),
    running: rgb(0x587539),
    pending: rgb(0x8C6C3E),
    completed: rgb(0x007197),
    failed: rgb(0xF52A65),
    cancelled: rgb(0x9854F1),
    fg: rgb(0x3760BF),
    muted: rgb(0x848CB5),
    border: rgb(0xC4C8DA),
    badge_fg: rgb(0xE1E2E7),
    select_bg: rgb(0xC4C8DA),
    column_bg: rgb(0xCDD1E3),
    bg: Some(rgb(0xE1E2E7)),
};
