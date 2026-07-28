//! Theme parsing, merging and name resolution. Nothing here touches the
//! global palette, so these can share a process with anything.

use lazyslurm::ui::theme::{
    Theme, builtin,
    load::{DEFAULT_THEME, ThemeRegistry, parse_theme, resolve_theme_name},
    rgb_of,
};
use lazyslurm::utils::config;
use ratatui::style::Color;

/// The guarantee that the shipped themes are written in exactly the format
/// users write. If `Theme` ever gains a slot that `ThemeFile` does not know
/// about, this fails immediately.
#[test]
fn builtins_round_trip_through_the_user_theme_format() {
    for (name, theme) in builtin::BUILTINS {
        let text = toml::to_string_pretty(theme).expect("theme should serialise");
        let parsed = parse_theme(&text).unwrap_or_else(|e| panic!("{name}: {e}\n{text}"));
        assert_eq!(*theme, parsed, "{name} did not survive a round trip");
    }
}

#[test]
fn builtin_names_are_unique_and_the_default_comes_first() {
    let names: Vec<_> = builtin::BUILTINS.iter().map(|(n, _)| *n).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "duplicate built-in theme name");
    assert_eq!(names[0], DEFAULT_THEME);
    assert!(builtin::get(DEFAULT_THEME).is_some());
    assert!(builtin::get("no-such-theme").is_none());
}

/// The default is transparent on purpose so it sits over whatever the terminal
/// is doing. Light themes cannot be, or their text lands on a dark terminal.
#[test]
fn light_themes_set_a_background_and_the_default_does_not() {
    assert!(builtin::LAZYSLURM.bg.is_none());
    for name in ["gruvbox-light", "catppuccin-latte", "tokyonight-day"] {
        let theme = builtin::get(name).unwrap();
        assert!(theme.bg.is_some(), "{name} needs an opaque background");
    }
}

#[test]
fn a_partial_theme_file_inherits_the_rest_from_extends() {
    let theme = parse_theme(
        r##"
        extends = "nord"
        accent = "#268BD2"
        "##,
    )
    .unwrap();

    assert_eq!(theme.accent, Color::Rgb(0x26, 0x8B, 0xD2));
    assert_eq!(theme.muted, builtin::NORD.muted);
    assert_eq!(theme.bg, builtin::NORD.bg);
}

#[test]
fn a_theme_file_without_extends_inherits_the_default() {
    let theme = parse_theme(r#"accent = "red""#).unwrap();
    assert_eq!(theme.accent, Color::Red);
    assert_eq!(theme.fg, builtin::LAZYSLURM.fg);
}

/// An absent `bg` inherits, so there has to be a way to ask for transparency
/// explicitly.
#[test]
fn bg_reset_clears_an_inherited_background() {
    let inherited = parse_theme(r#"extends = "nord""#).unwrap();
    assert!(inherited.bg.is_some());

    let cleared = parse_theme(
        r#"
        extends = "nord"
        bg = "reset"
        "#,
    )
    .unwrap();
    assert_eq!(cleared.bg, None);
}

#[test]
fn extends_naming_something_unknown_is_an_error() {
    let err = parse_theme(r#"extends = "solarised""#).unwrap_err();
    assert!(
        err.contains("solarised"),
        "error should name the value: {err}"
    );
}

/// A typo that silently does nothing is worse than a loud failure.
#[test]
fn an_unknown_key_is_rejected() {
    let err = parse_theme(r##"acent = "#ffffff""##).unwrap_err();
    assert!(err.contains("acent"), "error should name the key: {err}");
}

#[test]
fn a_bad_colour_error_names_the_offending_value() {
    let err = parse_theme(r#"accent = "burnt-sienna""#).unwrap_err();
    assert!(
        err.contains("burnt-sienna"),
        "error should name the value: {err}"
    );
}

#[test]
fn every_colour_syntax_ratatui_accepts_is_accepted() {
    let theme = parse_theme(
        r##"
        accent = "#268BD2"
        accent_alt = "light-blue"
        fg = "42"
        "##,
    )
    .unwrap();

    assert_eq!(theme.accent, Color::Rgb(0x26, 0x8B, 0xD2));
    assert_eq!(theme.accent_alt, Color::LightBlue);
    assert_eq!(theme.fg, Color::Indexed(42));
}

/// Confirming a theme in the picker rewrites the config, so a hand-written one
/// has to survive it intact.
#[test]
fn saving_a_theme_keeps_the_rest_of_the_config() {
    let existing = "# my settings\ntheme = \"nord\"\n";
    let updated = config::with_theme_set(existing, "gruvbox-dark");

    assert!(updated.contains("# my settings"), "{updated}");
    assert!(updated.contains(r#"theme = "gruvbox-dark""#), "{updated}");
    assert!(!updated.contains("nord"), "{updated}");
}

#[test]
fn saving_a_theme_into_an_empty_or_broken_config_still_works() {
    assert_eq!(
        config::with_theme_set("", "nord"),
        "theme = \"nord\"\n",
        "an absent config should just be created"
    );
    assert!(
        config::with_theme_set("this is not = = toml", "nord").contains("nord"),
        "an unparseable config should not block the save"
    );
}

#[test]
fn theme_name_precedence_runs_cli_then_env_then_config() {
    assert_eq!(resolve_theme_name(Some("a"), Some("b"), Some("c")), "a");
    assert_eq!(resolve_theme_name(None, Some("b"), Some("c")), "b");
    assert_eq!(resolve_theme_name(None, None, Some("c")), "c");
    assert_eq!(resolve_theme_name(None, None, None), DEFAULT_THEME);
}

/// A directory of user themes: one good, one with a bad colour, one with a
/// typo'd key. The good one has to land and the other two have to be reported
/// without taking the registry down with them.
#[test]
fn a_themes_directory_loads_the_good_files_and_reports_the_rest() {
    let dir = std::env::temp_dir().join("lazyslurm-theme-dir-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("solarised.toml"),
        "extends = \"nord\"\naccent = \"#268BD2\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("broken.toml"), "accent = \"burnt-sienna\"\n").unwrap();
    std::fs::write(dir.join("nord.toml"), "accent = \"red\"\n").unwrap();
    std::fs::write(dir.join("notes.txt"), "ignored\n").unwrap();

    let (registry, warnings) = ThemeRegistry::load_from(&dir);

    // Named by its filename, inheriting the rest from nord.
    let solarised = registry.get("solarised").expect("custom theme should load");
    assert_eq!(solarised.accent, Color::Rgb(0x26, 0x8B, 0xD2));
    assert_eq!(solarised.muted, builtin::NORD.muted);

    // A file named after a built-in retunes it in place rather than duplicating.
    assert_eq!(registry.get("nord").unwrap().accent, Color::Red);
    assert_eq!(
        registry
            .entries()
            .iter()
            .filter(|e| e.name == "nord")
            .count(),
        1
    );
    assert!(
        registry
            .entries()
            .iter()
            .find(|e| e.name == "nord")
            .unwrap()
            .user
    );

    assert_eq!(warnings.len(), 1, "got: {warnings:?}");
    assert!(warnings[0].contains("burnt-sienna"), "{warnings:?}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_themes_directory_is_not_an_error() {
    let (registry, warnings) =
        ThemeRegistry::load_from(std::path::Path::new("/no/such/lazyslurm/themes"));
    assert_eq!(registry.entries().len(), builtin::BUILTINS.len());
    assert!(warnings.is_empty());
}

#[test]
fn the_builtin_registry_offers_every_shipped_theme() {
    let registry = ThemeRegistry::builtin_only();
    assert_eq!(registry.entries().len(), builtin::BUILTINS.len());
    assert_eq!(registry.index_of(DEFAULT_THEME), Some(0));
    assert_eq!(registry.get("nord"), Some(builtin::NORD));
    assert!(registry.get("no-such-theme").is_none());
    assert!(registry.entries().iter().all(|e| !e.user));
}

/// Named colours are whatever the terminal decides, so there is no RGB to
/// report and the gradient has to fall back rather than guess.
#[test]
fn rgb_of_answers_for_true_colour_and_the_cube_but_not_names() {
    assert_eq!(rgb_of(Color::Rgb(1, 2, 3)), Some((1, 2, 3)));
    assert_eq!(rgb_of(Color::Indexed(16)), Some((0, 0, 0)));
    assert_eq!(rgb_of(Color::Indexed(231)), Some((255, 255, 255)));
    assert_eq!(rgb_of(Color::Indexed(232)), Some((8, 8, 8)));
    assert_eq!(rgb_of(Color::Indexed(255)), Some((238, 238, 238)));
    assert_eq!(rgb_of(Color::Blue), None);
    assert_eq!(rgb_of(Color::Indexed(4)), None);
    assert_eq!(rgb_of(Color::Reset), None);
}

/// A serialised theme is what `--print-theme` hands the user, so it has to be
/// readable rather than a debug dump.
#[test]
fn a_serialised_theme_uses_hex_strings() {
    let text = toml::to_string_pretty(&builtin::NORD).unwrap();
    assert!(text.contains(r##"accent = "#88C0D0""##), "{text}");

    let transparent: Theme = builtin::LAZYSLURM;
    let text = toml::to_string_pretty(&transparent).unwrap();
    assert!(
        !text.lines().any(|l| l.starts_with("bg ")),
        "an unset background should be omitted, got:\n{text}"
    );
}
