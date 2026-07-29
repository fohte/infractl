use std::path::PathBuf;

/// Returns the user's home directory from `$HOME`. Empty values are treated as unset.
fn home_dir() -> Option<PathBuf> {
    non_empty_env("HOME").map(PathBuf::from)
}

/// Returns the XDG config directory (`$XDG_CONFIG_HOME`, falling back to `$HOME/.config`).
/// Empty values are treated as unset per the XDG Base Directory Specification.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(xdg) = non_empty_env("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg));
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

    #[test]
    fn config_dir_uses_xdg_config_home_when_set() {
        temp_env::with_vars([("XDG_CONFIG_HOME", Some("/custom/config"))], || {
            assert_eq!(config_dir(), Some(PathBuf::from("/custom/config")));
        });
    }

    #[test]
    fn config_dir_falls_back_to_home_dot_config() {
        temp_env::with_vars(
            [
                ("XDG_CONFIG_HOME", None::<&str>),
                ("HOME", Some("/test/home")),
            ],
            || {
                assert_eq!(config_dir(), Some(PathBuf::from("/test/home/.config")));
            },
        );
    }

    #[test]
    fn config_dir_treats_empty_xdg_as_unset() {
        temp_env::with_vars(
            [("XDG_CONFIG_HOME", Some("")), ("HOME", Some("/test/home"))],
            || {
                assert_eq!(config_dir(), Some(PathBuf::from("/test/home/.config")));
            },
        );
    }

    #[test]
    fn config_dir_returns_none_when_home_unset() {
        temp_env::with_vars(
            [("XDG_CONFIG_HOME", None::<&str>), ("HOME", None::<&str>)],
            || {
                assert_eq!(config_dir(), None);
            },
        );
    }
}
