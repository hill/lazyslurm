use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

/// `$XDG_CONFIG_HOME/lazyslurm`, else `$HOME/.config/lazyslurm`.
///
/// Hand-rolled rather than pulled from a crate so the layout matches the
/// `~/.cache/lazyslurm` convention `update.rs` already uses, on macOS as well
/// as Linux. People bounce between a laptop and a login node and expect the
/// same path on both.
pub fn config_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("lazyslurm"))
}

pub fn config_path() -> Option<PathBuf> {
    Some(config_dir()?.join("config.toml"))
}

pub fn themes_dir() -> Option<PathBuf> {
    Some(config_dir()?.join("themes"))
}

/// Unknown keys are ignored on purpose: an older binary should not refuse to
/// start because a newer one wrote a setting it does not understand.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: Option<String>,
}

/// Best effort. A broken config yields defaults plus a warning, because it
/// must never stop you looking at your jobs.
pub fn load() -> (Config, Option<String>) {
    let Some(path) = config_path() else {
        return (Config::default(), None);
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return (Config::default(), None);
    };

    match toml::from_str::<Config>(&text) {
        Ok(config) => (config, None),
        Err(err) => {
            let detail = err.to_string();
            let detail = detail.lines().next().unwrap_or("invalid").to_string();
            (
                Config::default(),
                Some(format!("{}: {detail}", path.display())),
            )
        }
    }
}

/// Write the chosen theme back, leaving every other key, comment and ordering
/// in the file untouched.
pub fn persist_theme(name: &str) -> Result<()> {
    let path = config_path().ok_or_else(|| anyhow!("no config directory ($HOME is unset)"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }

    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = with_theme_set(&existing, name);

    // Write and rename so an interrupted save cannot truncate the real file.
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, updated).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, &path).with_context(|| format!("saving {}", path.display()))?;
    Ok(())
}

/// Set `theme` in an existing config, keeping every other key, comment and
/// ordering as the user left them. A file too broken to parse is replaced;
/// `load` has already warned about it by this point.
pub fn with_theme_set(existing: &str, name: &str) -> String {
    let mut doc = existing
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_default();
    doc["theme"] = toml_edit::value(name);
    doc.to_string()
}
