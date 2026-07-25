//! Startup update check. Asks crates.io for the latest published version and,
//! if it's newer than the running build, surfaces a badge in the status bar.
//!
//! Every failure path (no cache, no curl/wget, offline, timeout, bad JSON) is a
//! silent `None`. A version check must never disrupt or slow down the TUI, and
//! on a locked-down login node it simply finds nothing and moves on.

use std::path::PathBuf;
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

/// Best-effort open of `url` in the platform browser. Fire-and-forget: on a
/// headless login node there may be no opener, and that is fine.
pub fn open_url(url: &str) {
    let (cmd, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[])
    } else if cfg!(target_os = "windows") {
        ("cmd", &["/C", "start", ""])
    } else {
        ("xdg-open", &[])
    };
    let _ = std::process::Command::new(cmd).args(args).arg(url).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

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
