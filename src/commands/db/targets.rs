use clap::Args;

use crate::{config, registry};
use registry::Target;

#[derive(Args)]
pub struct TargetsArgs {}

pub fn run(_args: &TargetsArgs) -> anyhow::Result<()> {
    let config = config::load_config()?;
    let targets = registry::list(&config);
    print!("{}", format_targets(&targets));
    Ok(())
}

const COLUMN_HEADERS: [&str; 5] = ["NAME", "CONTEXT", "NAMESPACE", "CLUSTER", "DATABASE"];
const CURRENT_CONTEXT_LABEL: &str = "(current)";

/// Render targets as a left-aligned table, without touching the current
/// kubectl context or cluster, so this is a pure function of `targets`.
fn format_targets(targets: &[Target]) -> String {
    if targets.is_empty() {
        return "No targets configured.\n".to_string();
    }

    let rows: Vec<[String; 5]> = targets
        .iter()
        .map(|t| {
            [
                t.name.clone(),
                t.context
                    .clone()
                    .unwrap_or_else(|| CURRENT_CONTEXT_LABEL.to_string()),
                t.namespace.clone(),
                t.cluster.clone(),
                t.database.clone(),
            ]
        })
        .collect();

    let widths: [usize; 5] = std::array::from_fn(|i| {
        rows.iter()
            .map(|r| r[i].len())
            .chain(std::iter::once(COLUMN_HEADERS[i].len()))
            .max()
            .unwrap_or(0)
    });

    let mut out = format_row(&COLUMN_HEADERS.map(String::from), &widths);
    out.push('\n');
    for row in &rows {
        out.push_str(&format_row(row, &widths));
        out.push('\n');
    }
    out
}

/// Join columns with two-space gaps, padding every column but the last (which
/// is left ragged to avoid trailing whitespace).
fn format_row(cols: &[String; 5], widths: &[usize; 5]) -> String {
    cols.iter()
        .enumerate()
        .map(|(i, c)| {
            if i + 1 == cols.len() {
                c.clone()
            } else {
                format!("{c:<width$}", width = widths[i])
            }
        })
        .collect::<Vec<_>>()
        .join("  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn target(
        name: &str,
        context: Option<&str>,
        namespace: &str,
        cluster: &str,
        database: &str,
    ) -> Target {
        Target {
            name: name.to_string(),
            context: context.map(String::from),
            namespace: namespace.to_string(),
            cluster: cluster.to_string(),
            database: database.to_string(),
            user: registry::READER_ROLE.to_string(),
            secret_name: registry::READER_SECRET_NAME.to_string(),
        }
    }

    #[test]
    fn format_targets_empty_reports_no_targets() {
        assert_eq!(format_targets(&[]), "No targets configured.\n");
    }

    #[test]
    fn format_targets_single_row_shows_current_context_placeholder() {
        let targets = vec![target("tq", None, "databases", "main", "tq")];

        assert_eq!(
            format_targets(&targets),
            indoc! {"
                NAME  CONTEXT    NAMESPACE  CLUSTER  DATABASE
                tq    (current)  databases  main     tq
            "}
        );
    }

    #[test]
    fn format_targets_aligns_columns_across_multiple_rows() {
        let targets = vec![
            target(
                "mastodon",
                Some("home-k8s"),
                "databases",
                "main",
                "mastodon",
            ),
            target("tq", None, "databases", "main", "tq"),
        ];

        assert_eq!(
            format_targets(&targets),
            indoc! {"
                NAME      CONTEXT    NAMESPACE  CLUSTER  DATABASE
                mastodon  home-k8s   databases  main     mastodon
                tq        (current)  databases  main     tq
            "}
        );
    }
}
