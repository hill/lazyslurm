/// Resolve the user whose jobs we should show by default.
///
/// `$USER` is the fast path, but it isn't always exported (`docker exec`, cron,
/// bare login shells), so we fall back to `$LOGNAME` and finally ask the OS for
/// the effective login name. Returning `None` here would mean "show every user",
/// which is rarely what someone starting the TUI actually wants.
pub fn invoking_user() -> Option<String> {
    non_empty(std::env::var("USER").ok())
        .or_else(|| non_empty(std::env::var("LOGNAME").ok()))
        .or_else(|| non_empty(Some(whoami::username())))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_values_are_dropped() {
        assert_eq!(non_empty(Some("".into())), None);
        assert_eq!(non_empty(Some("   ".into())), None);
        assert_eq!(non_empty(None), None);
        assert_eq!(non_empty(Some("alice".into())), Some("alice".into()));
    }

    #[test]
    fn always_resolves_someone() {
        // Even with $USER/$LOGNAME unset, the OS lookup should still name a user,
        // so the default filter never silently falls back to "all".
        let user = invoking_user();
        assert!(user.is_some_and(|u| !u.trim().is_empty()));
    }
}
