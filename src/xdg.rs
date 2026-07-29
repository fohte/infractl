use std::path::PathBuf;

/// Returns the user's home directory from `$HOME`. Empty values are treated as unset.
fn home_dir() -> Option<PathBuf> {
    non_empty_env("HOME").map(PathBuf::from)
}

/// Returns the XDG config directory (`$XDG_CONFIG_HOME`, falling back to `$HOME/.config`).
/// Empty or relative values are treated as unset per the XDG Base Directory
/// Specification, which requires these paths to be absolute.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(xdg) = non_empty_env("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg);
        if path.is_absolute() {
            return Some(path);
        }
    }
    home_dir().map(|home| home.join(".config"))
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
}
