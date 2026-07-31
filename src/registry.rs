use crate::config::{Config, TargetConfig};

/// Postgres role every infractl connection authenticates as. Read-only by
/// design (only `pg_read_all_data` is granted), so this is the actual safety
/// boundary — not a CLI flag, which a caller could otherwise use to connect
/// as an arbitrary, possibly writable, role.
pub const READER_ROLE: &str = "infractl_reader";

/// Name of the `kubernetes.io/basic-auth` Secret backing `READER_ROLE`'s password.
pub const READER_SECRET_NAME: &str = "cnpg-role-infractl-reader";

/// A fully resolved connection profile for a named target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    pub context: Option<String>,
    pub namespace: String,
    pub cluster: String,
    pub database: String,
    pub user: String,
    pub secret_name: String,
}

impl Target {
    fn from_config(name: &str, config: &TargetConfig) -> Self {
        Self {
            name: name.to_string(),
            context: config.context.clone(),
            namespace: config.namespace.clone(),
            cluster: config.cluster.clone(),
            database: config.database.clone(),
            user: READER_ROLE.to_string(),
            secret_name: READER_SECRET_NAME.to_string(),
        }
    }
}

/// Resolve a single target by name.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "not called until `infractl db query` is implemented"
    )
)]
pub fn resolve(config: &Config, name: &str) -> Option<Target> {
    config
        .targets
        .get(name)
        .map(|target_config| Target::from_config(name, target_config))
}

/// List all registered targets, sorted by name for stable output.
pub fn list(config: &Config) -> Vec<Target> {
    let mut targets: Vec<Target> = config
        .targets
        .iter()
        .map(|(name, target_config)| Target::from_config(name, target_config))
        .collect();
    targets.sort_by(|a, b| a.name.cmp(&b.name));
    targets
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn target_config(
        context: Option<&str>,
        namespace: &str,
        cluster: &str,
        database: &str,
    ) -> TargetConfig {
        TargetConfig {
            context: context.map(String::from),
            namespace: namespace.to_string(),
            cluster: cluster.to_string(),
            database: database.to_string(),
        }
    }

    #[test]
    fn resolve_returns_target_with_derived_reader_fields() {
        let config = Config {
            targets: HashMap::from([(
                "tq".to_string(),
                target_config(None, "databases", "main", "tq"),
            )]),
        };

        assert_eq!(
            resolve(&config, "tq"),
            Some(Target {
                name: "tq".to_string(),
                context: None,
                namespace: "databases".to_string(),
                cluster: "main".to_string(),
                database: "tq".to_string(),
                user: "infractl_reader".to_string(),
                secret_name: "cnpg-role-infractl-reader".to_string(),
            })
        );
    }

    #[test]
    fn resolve_returns_none_for_unknown_target() {
        let config = Config::default();

        assert_eq!(resolve(&config, "unknown"), None);
    }

    #[test]
    fn list_sorts_targets_by_name() {
        let config = Config {
            targets: HashMap::from([
                (
                    "mastodon".to_string(),
                    target_config(Some("home-k8s"), "databases", "main", "mastodon"),
                ),
                (
                    "tq".to_string(),
                    target_config(None, "databases", "main", "tq"),
                ),
                (
                    "meshi".to_string(),
                    target_config(None, "databases", "main", "meshi"),
                ),
            ]),
        };

        assert_eq!(
            list(&config),
            vec![
                Target {
                    name: "mastodon".to_string(),
                    context: Some("home-k8s".to_string()),
                    namespace: "databases".to_string(),
                    cluster: "main".to_string(),
                    database: "mastodon".to_string(),
                    user: READER_ROLE.to_string(),
                    secret_name: READER_SECRET_NAME.to_string(),
                },
                Target {
                    name: "meshi".to_string(),
                    context: None,
                    namespace: "databases".to_string(),
                    cluster: "main".to_string(),
                    database: "meshi".to_string(),
                    user: READER_ROLE.to_string(),
                    secret_name: READER_SECRET_NAME.to_string(),
                },
                Target {
                    name: "tq".to_string(),
                    context: None,
                    namespace: "databases".to_string(),
                    cluster: "main".to_string(),
                    database: "tq".to_string(),
                    user: READER_ROLE.to_string(),
                    secret_name: READER_SECRET_NAME.to_string(),
                },
            ]
        );
    }

    #[test]
    fn list_returns_empty_for_empty_config() {
        let config = Config::default();

        assert_eq!(list(&config), Vec::new());
    }
}
