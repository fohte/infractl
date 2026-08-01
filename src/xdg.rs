use std::path::PathBuf;

/// Returns the user's home directory from `$HOME`. Empty values are treated as unset.
fn home_dir() -> Option<PathBuf> {
    non_empty_env("HOME").map(PathBuf::from)
}

/// Resolves an XDG base directory: `$<env_key>` if set to an absolute path,
/// otherwise `$HOME/<fallback_subdir>`. Empty or relative values are treated
/// as unset per the XDG Base Directory Specification, which requires these
/// paths to be absolute.
fn xdg_dir(env_key: &str, fallback_subdir: &str) -> Option<PathBuf> {
    if let Some(xdg) = non_empty_env(env_key) {
        let path = PathBuf::from(xdg);
        if path.is_absolute() {
            return Some(path);
        }
    }
    home_dir().map(|home| home.join(fallback_subdir))
}

/// Returns the XDG config directory (`$XDG_CONFIG_HOME`, falling back to `$HOME/.config`).
pub fn config_dir() -> Option<PathBuf> {
    xdg_dir("XDG_CONFIG_HOME", ".config")
}

/// Returns the XDG cache directory (`$XDG_CACHE_HOME`, falling back to `$HOME/.cache`).
pub fn cache_dir() -> Option<PathBuf> {
    xdg_dir("XDG_CACHE_HOME", ".cache")
}

/// Returns the value of an environment variable, treating empty strings as unset.
fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::xdg_config_home_set(
        vec![("XDG_CONFIG_HOME", Some("/custom/config"))],
        Some("/custom/config")
    )]
    #[case::falls_back_to_home_dot_config(
        vec![("XDG_CONFIG_HOME", None), ("HOME", Some("/test/home"))],
        Some("/test/home/.config")
    )]
    #[case::empty_xdg_treated_as_unset(
        vec![("XDG_CONFIG_HOME", Some("")), ("HOME", Some("/test/home"))],
        Some("/test/home/.config")
    )]
    #[case::relative_xdg_treated_as_unset(
        vec![("XDG_CONFIG_HOME", Some("relative/config")), ("HOME", Some("/test/home"))],
        Some("/test/home/.config")
    )]
    #[case::returns_none_when_home_unset(
        vec![("XDG_CONFIG_HOME", None), ("HOME", None)],
        None
    )]
    fn test_config_dir(#[case] vars: Vec<(&str, Option<&str>)>, #[case] expected: Option<&str>) {
        temp_env::with_vars(vars, || {
            assert_eq!(config_dir(), expected.map(PathBuf::from));
        });
    }

    #[rstest]
    #[case::xdg_cache_home_set(
        vec![("XDG_CACHE_HOME", Some("/custom/cache"))],
        Some("/custom/cache")
    )]
    #[case::falls_back_to_home_dot_cache(
        vec![("XDG_CACHE_HOME", None), ("HOME", Some("/test/home"))],
        Some("/test/home/.cache")
    )]
    #[case::empty_xdg_treated_as_unset(
        vec![("XDG_CACHE_HOME", Some("")), ("HOME", Some("/test/home"))],
        Some("/test/home/.cache")
    )]
    #[case::relative_xdg_treated_as_unset(
        vec![("XDG_CACHE_HOME", Some("relative/cache")), ("HOME", Some("/test/home"))],
        Some("/test/home/.cache")
    )]
    #[case::returns_none_when_home_unset(
        vec![("XDG_CACHE_HOME", None), ("HOME", None)],
        None
    )]
    fn test_cache_dir(#[case] vars: Vec<(&str, Option<&str>)>, #[case] expected: Option<&str>) {
        temp_env::with_vars(vars, || {
            assert_eq!(cache_dir(), expected.map(PathBuf::from));
        });
    }
}
