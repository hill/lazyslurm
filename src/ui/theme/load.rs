use std::str::FromStr;

use ratatui::style::Color;
use serde::{Deserialize, Deserializer, de};

use super::{Theme, builtin};
use crate::utils::config;

/// A theme file as written by a user. Every slot is optional so a file can
/// override two colours and inherit the rest.
///
/// Unknown keys are rejected here, unlike in `Config`: a typo'd `acent` that
/// silently does nothing is a worse failure than a loud one, and adding a slot
/// in a later release never breaks an older file.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ThemeFile {
    /// A built-in name to start from. Absent means the default theme.
    pub extends: Option<String>,

    #[serde(deserialize_with = "de_color")]
    pub accent: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub accent_alt: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub running: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub pending: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub completed: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub failed: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub cancelled: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub fg: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub muted: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub border: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub badge_fg: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub select_bg: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub column_bg: Option<Color>,
    #[serde(deserialize_with = "de_color")]
    pub bg: Option<Color>,
}

/// Reads a colour as a plain string so a bad value can name itself. Ratatui's
/// own `Deserialize` routes through an untagged enum, whose error says only
/// that nothing matched.
fn de_color<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Color>, D::Error> {
    let Some(raw) = Option::<String>::deserialize(deserializer)? else {
        return Ok(None);
    };
    Color::from_str(&raw).map(Some).map_err(|_| {
        de::Error::custom(format!(
            "unknown colour {raw:?} (expected #RRGGBB, a name like \"light-blue\", or 0-255)"
        ))
    })
}

impl ThemeFile {
    /// Fill the gaps from `extends`, or from the default theme.
    pub fn resolve(&self) -> Result<Theme, String> {
        let base = match &self.extends {
            Some(name) => builtin::get(name)
                .ok_or_else(|| format!("extends: no built-in theme called {name:?}"))?,
            None => builtin::LAZYSLURM,
        };

        Ok(Theme {
            accent: self.accent.unwrap_or(base.accent),
            accent_alt: self.accent_alt.unwrap_or(base.accent_alt),
            running: self.running.unwrap_or(base.running),
            pending: self.pending.unwrap_or(base.pending),
            completed: self.completed.unwrap_or(base.completed),
            failed: self.failed.unwrap_or(base.failed),
            cancelled: self.cancelled.unwrap_or(base.cancelled),
            fg: self.fg.unwrap_or(base.fg),
            muted: self.muted.unwrap_or(base.muted),
            border: self.border.unwrap_or(base.border),
            badge_fg: self.badge_fg.unwrap_or(base.badge_fg),
            select_bg: self.select_bg.unwrap_or(base.select_bg),
            column_bg: self.column_bg.unwrap_or(base.column_bg),
            // `bg = "reset"` is how a file asks for terminal transparency,
            // since an absent key means "inherit" rather than "unset".
            bg: match self.bg {
                Some(Color::Reset) => None,
                Some(color) => Some(color),
                None => base.bg,
            },
        })
    }
}

pub fn parse_theme(text: &str) -> Result<Theme, String> {
    toml::from_str::<ThemeFile>(text)
        .map_err(|e| e.message().to_string())?
        .resolve()
}

pub struct ThemeEntry {
    pub name: String,
    pub theme: Theme,
    /// True when it came from the user's themes directory, including when it
    /// shadows a built-in of the same name.
    pub user: bool,
}

/// Every theme the picker can offer: the built-ins, plus anything in the
/// user's themes directory.
pub struct ThemeRegistry {
    entries: Vec<ThemeEntry>,
}

impl ThemeRegistry {
    /// Built-ins only, touching no filesystem. This is what `App::new` uses so
    /// tests and headless mode stay hermetic.
    pub fn builtin_only() -> Self {
        Self {
            entries: builtin::BUILTINS
                .iter()
                .map(|(name, theme)| ThemeEntry {
                    name: (*name).to_string(),
                    theme: *theme,
                    user: false,
                })
                .collect(),
        }
    }

    /// Built-ins plus `~/.config/lazyslurm/themes/*.toml`.
    pub fn load() -> (Self, Vec<String>) {
        match config::themes_dir() {
            Some(dir) => Self::load_from(&dir),
            None => (Self::builtin_only(), Vec::new()),
        }
    }

    /// Built-ins plus every `*.toml` in `dir`. A user file whose stem matches a
    /// built-in replaces it, so you can retune `nord` in place. Files that fail
    /// to load come back as warnings and are skipped, never fatal.
    pub fn load_from(dir: &std::path::Path) -> (Self, Vec<String>) {
        let mut registry = Self::builtin_only();
        let mut warnings = Vec::new();

        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return (registry, warnings);
        };

        let mut paths: Vec<_> = read_dir
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
            .collect();
        paths.sort();

        for path in paths {
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let theme = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| parse_theme(&text));

            match theme {
                Ok(theme) => registry.insert(name, theme),
                Err(err) => warnings.push(format!("{}: {err}", path.display())),
            }
        }

        (registry, warnings)
    }

    fn insert(&mut self, name: &str, theme: Theme) {
        match self.entries.iter_mut().find(|e| e.name == name) {
            Some(existing) => {
                existing.theme = theme;
                existing.user = true;
            }
            None => self.entries.push(ThemeEntry {
                name: name.to_string(),
                theme,
                user: true,
            }),
        }
    }

    pub fn get(&self, name: &str) -> Option<Theme> {
        self.entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.theme)
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.name == name)
    }

    pub fn entries(&self) -> &[ThemeEntry] {
        &self.entries
    }
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::builtin_only()
    }
}

/// The name to use, in precedence order. Kept free of env lookups so it stays
/// directly testable.
pub fn resolve_theme_name<'a>(
    cli: Option<&'a str>,
    env: Option<&'a str>,
    config: Option<&'a str>,
) -> &'a str {
    cli.or(env).or(config).unwrap_or(DEFAULT_THEME)
}

pub const DEFAULT_THEME: &str = "lazyslurm";
