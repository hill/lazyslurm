//! Startup update check. Asks crates.io for the latest published version and,
//! if it's newer than the running build, surfaces a badge in the status bar.
//!
//! Every failure path (no cache, no curl/wget, offline, timeout, bad JSON) is a
//! silent `None`. A version check must never disrupt or slow down the TUI, and
//! on a locked-down login node it simply finds nothing and moves on.

use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tokio::process::Command;

/// The crates.io page, opened when the update badge is clicked.
pub const CRATES_URL: &str = "https://crates.io/crates/lazyslurm";

const API_URL: &str = "https://crates.io/api/v1/crates/lazyslurm";
const CACHE_TTL_SECS: u64 = 24 * 60 * 60;

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The latest stable version, but only when it's newer than what's running.
/// A fresh disk cache short-circuits the network so most launches touch nothing.
pub async fn check_for_update() -> Option<String> {
    let latest = match cached_latest() {
        Some(v) => v,
        None => {
            let latest = parse_crates_io_latest(&fetch_body().await?)?;
            write_cache(&latest);
            latest
        }
    };
    is_newer(&latest, current_version()).then_some(latest)
}

/// Fetch the crates.io metadata by shelling out, curl first then wget. Keeping
/// this a subprocess avoids pulling an HTTP + TLS stack into the binary.
async fn fetch_body() -> Option<String> {
    // crates.io rejects requests without a User-Agent.
    let ua = format!("lazyslurm/{}", current_version());

    if let Ok(out) = Command::new("curl")
        .args(["-fsSL", "--max-time", "3", "-A", &ua, API_URL])
        .output()
        .await
        && out.status.success()
    {
        return Some(String::from_utf8_lossy(&out.stdout).into_owned());
    }

    let wget = Command::new("wget")
        .args(["-qO-", "--timeout=3", "-U", &ua, API_URL])
        .output()
        .await
        .ok()?;
    wget.status
        .success()
        .then(|| String::from_utf8_lossy(&wget.stdout).into_owned())
}

fn parse_crates_io_latest(json: &str) -> Option<String> {
    let value: Value = serde_json::from_str(json).ok()?;
    value["crate"]["max_stable_version"]
        .as_str()
        .map(str::to_string)
}

/// Parse `X.Y.Z` (tolerating a leading `v` and any pre-release suffix on the
/// patch) into a comparable tuple. Returns `None` on anything unexpected.
fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let mut parts = s.trim().trim_start_matches('v').split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts
        .next()?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

fn cache_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("lazyslurm").join("update_check.json"))
}

fn now_secs() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

fn within_ttl(checked_at: u64, now: u64) -> bool {
    now.saturating_sub(checked_at) < CACHE_TTL_SECS
}

/// The cached latest version, but only while the entry is within the TTL.
fn cached_latest() -> Option<String> {
    let value: Value = serde_json::from_str(&std::fs::read_to_string(cache_path()?).ok()?).ok()?;
    let checked_at = value["checked_at"].as_u64()?;
    if !within_ttl(checked_at, now_secs()?) {
        return None;
    }
    value["latest"].as_str().map(str::to_string)
}

fn write_cache(latest: &str) {
    let (Some(path), Some(now)) = (cache_path(), now_secs()) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body = serde_json::json!({ "checked_at": now, "latest": latest });
    let _ = std::fs::write(path, body.to_string());
}

/// What opening a URL managed to do.
pub enum OpenOutcome {
    /// A browser was launched on this machine.
    Opened,
    /// No browser worth launching, so the URL went to the terminal's clipboard.
    Copied,
}

/// Open `url` in the platform browser, or hand it to the terminal instead.
///
/// On a login node there is no browser, and `xdg-open` says so by printing a
/// "command not found" line per candidate. Those go to the tty the TUI draws
/// on, so they land in the middle of the frame. The child gets closed stdio to
/// stop that, and a headless host skips the opener entirely.
pub fn open_url(url: &str) -> OpenOutcome {
    if has_display() && spawn_opener(url) {
        return OpenOutcome::Opened;
    }
    copy_to_terminal_clipboard(url);
    OpenOutcome::Copied
}

/// Whether a browser launched here would appear in front of whoever is
/// watching. Over SSH it would open on the cluster, so this is false unless X
/// is forwarded.
fn has_display() -> bool {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        return std::env::var_os("SSH_CONNECTION").is_none();
    }
    std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn spawn_opener(url: &str) -> bool {
    let (cmd, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[])
    } else if cfg!(target_os = "windows") {
        ("cmd", &["/C", "start", ""])
    } else {
        ("xdg-open", &[])
    };
    std::process::Command::new(cmd)
        .args(args)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

/// Copy via OSC 52, which asks the terminal emulator to set the clipboard. The
/// escape travels back down the SSH connection, so the text lands on the
/// machine you are sitting at rather than the login node. Ghostty, kitty,
/// WezTerm, iTerm2 and Alacritty honour it. tmux and screen need passthrough
/// turned on, and there is no reply to tell us either way.
fn copy_to_terminal_clipboard(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = write!(out, "\x1b]52;c;{}\x07", base64(text.as_bytes()));
    let _ = out.flush();
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let padded = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let word = u32::from(padded[0]) << 16 | u32::from(padded[1]) << 8 | u32::from(padded[2]);
        for i in 0..4 {
            match i <= chunk.len() {
                true => out.push(ALPHABET[(word >> (18 - 6 * i)) as usize & 63] as char),
                false => out.push('='),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_pads_every_remainder() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(
            base64(CRATES_URL.as_bytes()),
            "aHR0cHM6Ly9jcmF0ZXMuaW8vY3JhdGVzL2xhenlzbHVybQ=="
        );
    }

    #[test]
    fn parses_plain_and_prefixed_versions() {
        assert_eq!(parse_version("0.3.1"), Some((0, 3, 1)));
        assert_eq!(parse_version("v1.20.4"), Some((1, 20, 4)));
        assert_eq!(parse_version("2.0.0-rc1"), Some((2, 0, 0)));
    }

    #[test]
    fn rejects_garbage_versions() {
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("0.3"), None);
        assert_eq!(parse_version("not.a.version"), None);
    }

    #[test]
    fn newer_only_when_strictly_greater() {
        assert!(is_newer("0.4.0", "0.3.1"));
        assert!(is_newer("0.3.2", "0.3.1"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.3.1", "0.3.1"));
        assert!(!is_newer("0.3.0", "0.3.1"));
        assert!(!is_newer("garbage", "0.3.1"));
    }

    #[test]
    fn extracts_max_stable_version() {
        let body = r#"{"crate":{"name":"lazyslurm","max_stable_version":"0.4.0"},"versions":[]}"#;
        assert_eq!(parse_crates_io_latest(body), Some("0.4.0".to_string()));
    }

    #[test]
    fn malformed_json_yields_none() {
        assert_eq!(parse_crates_io_latest("{ not json"), None);
        assert_eq!(parse_crates_io_latest("{}"), None);
    }

    #[test]
    fn ttl_boundary() {
        assert!(within_ttl(1000, 1000));
        assert!(within_ttl(1000, 1000 + CACHE_TTL_SECS - 1));
        assert!(!within_ttl(1000, 1000 + CACHE_TTL_SECS));
    }
}
