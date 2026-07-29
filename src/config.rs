use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Top-level configuration for infractl, loaded from `~/.config/infractl/config.yml`.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Database targets, keyed by target name.
    #[serde(default)]
    pub targets: HashMap<String, TargetConfig>,
}

/// A single target's connection profile, as declared in the config file.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TargetConfig {
    /// kubectl context to use. `None` means the current context.
    #[serde(default)]
    pub context: Option<String>,

    /// Kubernetes namespace the CNPG cluster runs in.
    pub namespace: String,

    /// CNPG Cluster name.
    pub cluster: String,

    /// Logical database name to connect to.
    pub database: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("invalid config file {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_yaml::Error,
    },
}

/// Load the config from `~/.config/infractl/config.yml` (XDG-aware).
/// Returns `Config::default()` if no config directory can be resolved.
pub fn load_config() -> Result<Config, ConfigError> {
    let Some(dir) = crate::xdg::config_dir() else {
        return Ok(Config::default());
    };
    load_config_from_path(&dir.join("infractl").join("config.yml"))
}

/// Load config from an explicit path. Returns `Config::default()` if the file doesn't exist.
pub fn load_config_from_path(path: &Path) -> Result<Config, ConfigError> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => {
            return Err(ConfigError::Read {
                path: path.to_path_buf(),
                source: e,
            });
        }
    };

    serde_yaml::from_str(&content).map_err(|e| ConfigError::Parse {
        path: path.to_path_buf(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_config_from_path_returns_default_for_missing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yml");

        assert_eq!(load_config_from_path(&path).unwrap(), Config::default());
    }

    #[test]
    fn load_config_from_path_parses_targets() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yml");
        fs::write(
            &path,
            indoc! {"
                targets:
                  tq:
                    namespace: databases
                    cluster: main
                    database: tq
                  mastodon:
                    context: home-k8s
                    namespace: databases
                    cluster: main
                    database: mastodon
            "},
        )
        .unwrap();

        let config = load_config_from_path(&path).unwrap();

        assert_eq!(
            config,
            Config {
                targets: HashMap::from([
                    (
                        "tq".to_string(),
                        TargetConfig {
                            context: None,
                            namespace: "databases".to_string(),
                            cluster: "main".to_string(),
                            database: "tq".to_string(),
                        }
                    ),
                    (
                        "mastodon".to_string(),
                        TargetConfig {
                            context: Some("home-k8s".to_string()),
                            namespace: "databases".to_string(),
                            cluster: "main".to_string(),
                            database: "mastodon".to_string(),
                        }
                    ),
                ]),
            }
        );
    }

    #[test]
    fn load_config_from_path_rejects_unknown_fields() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yml");
        fs::write(
            &path,
            indoc! {"
                targets:
                  tq:
                    namespace: databases
                    cluster: main
                    database: tq
                    user: superuser
            "},
        )
        .unwrap();

        let err = load_config_from_path(&path).unwrap_err();

        assert!(matches!(err, ConfigError::Parse { path: err_path, .. } if err_path == path));
    }

    #[test]
    fn load_config_from_path_reports_read_error_for_unreadable_file() {
        let dir = TempDir::new().unwrap();
        // A directory can't be read as a file, forcing an io::Error that isn't NotFound.
        let path = dir.path().join("config.yml");
        fs::create_dir(&path).unwrap();

        let err = load_config_from_path(&path).unwrap_err();

        assert!(matches!(err, ConfigError::Read { path: err_path, .. } if err_path == path));
    }
}
