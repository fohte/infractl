use std::time::Duration;

use anyhow::Context;
use serde::Serialize;
use tokio_postgres::{Config, NoTls, SimpleQueryMessage};

use crate::registry::Target;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const STATEMENT_TIMEOUT: &str = "30s";

/// One statement's results from a (possibly multi-statement) query, with
/// every value already rendered as text by the simple query protocol.
#[derive(Debug, PartialEq, Serialize)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

/// Connect as the target's reader role, apply the read-only / statement
/// timeout session guards, and run `sql` via the simple query protocol.
///
/// The simple query protocol returns every column as text and allows
/// multiple `;`-separated statements. Both traits are fine here:
/// `infractl_reader` only has `pg_read_all_data`, so there's nothing an
/// extra statement could write, and returning text avoids having to map
/// every Postgres type to a display string ourselves.
pub async fn run_query(
    target: &Target,
    password: &str,
    local_port: u16,
    sql: &str,
) -> anyhow::Result<Vec<ResultSet>> {
    let mut config = Config::new();
    config
        .host("127.0.0.1")
        .port(local_port)
        .user(target.user.as_str())
        .password(password)
        .dbname(target.database.as_str())
        .connect_timeout(CONNECT_TIMEOUT);

    let (client, connection) = config
        .connect(NoTls)
        .await
        .with_context(|| format!("failed to connect to target {:?}", target.name))?;

    tokio::spawn(async move {
        if let Err(err) = connection.await {
            eprintln!("postgres connection error: {err}");
        }
    });

    client
        .batch_execute(&format!(
            "SET default_transaction_read_only = on; SET statement_timeout = '{STATEMENT_TIMEOUT}';"
        ))
        .await
        .context("failed to apply session guards")?;

    let messages = client.simple_query(sql).await.context("query failed")?;

    Ok(collect_result_sets(messages.into_iter().filter_map(
        |message| {
            match message {
                SimpleQueryMessage::RowDescription(columns) => Some(SimpleMessage::Columns(
                    columns.iter().map(|c| c.name().to_string()).collect(),
                )),
                SimpleQueryMessage::Row(row) => Some(SimpleMessage::Row(
                    (0..row.len())
                        .map(|i| row.get(i).map(str::to_string))
                        .collect(),
                )),
                SimpleQueryMessage::CommandComplete(_) => Some(SimpleMessage::CommandComplete),
                _ => None,
            }
        },
    )))
}

/// An owned, test-constructible projection of the bits of
/// [`SimpleQueryMessage`] that [`collect_result_sets`] needs. `tokio_postgres`
/// doesn't expose a public constructor for `SimpleQueryRow`/`SimpleColumn`,
/// so this indirection is what makes the grouping logic unit-testable.
enum SimpleMessage {
    Columns(Vec<String>),
    Row(Vec<Option<String>>),
    CommandComplete,
}

/// Groups a flat message stream into one [`ResultSet`] per statement,
/// splitting on `CommandComplete`. A statement that returns no columns and
/// no rows (e.g. `SET ...`) is dropped rather than producing an empty set.
fn collect_result_sets(messages: impl IntoIterator<Item = SimpleMessage>) -> Vec<ResultSet> {
    let mut result_sets = Vec::new();
    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Option<String>>> = Vec::new();

    for message in messages {
        match message {
            SimpleMessage::Columns(cols) => columns = cols,
            SimpleMessage::Row(row) => rows.push(row),
            SimpleMessage::CommandComplete => {
                if !columns.is_empty() || !rows.is_empty() {
                    result_sets.push(ResultSet {
                        columns: std::mem::take(&mut columns),
                        rows: std::mem::take(&mut rows),
                    });
                }
            }
        }
    }

    result_sets
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::groups_one_set_per_statement(
        vec![
            SimpleMessage::Columns(vec!["id".to_string(), "name".to_string()]),
            SimpleMessage::Row(vec![Some("1".to_string()), Some("alice".to_string())]),
            SimpleMessage::Row(vec![Some("2".to_string()), None]),
            SimpleMessage::CommandComplete,
            SimpleMessage::Columns(vec!["total".to_string()]),
            SimpleMessage::Row(vec![Some("2".to_string())]),
            SimpleMessage::CommandComplete,
        ],
        vec![
            ResultSet {
                columns: vec!["id".to_string(), "name".to_string()],
                rows: vec![
                    vec![Some("1".to_string()), Some("alice".to_string())],
                    vec![Some("2".to_string()), None],
                ],
            },
            ResultSet {
                columns: vec!["total".to_string()],
                rows: vec![vec![Some("2".to_string())]],
            },
        ]
    )]
    #[case::keeps_empty_row_description_as_empty_result_set(
        vec![
            SimpleMessage::Columns(vec!["id".to_string()]),
            SimpleMessage::CommandComplete,
        ],
        vec![ResultSet {
            columns: vec!["id".to_string()],
            rows: vec![],
        }]
    )]
    #[case::drops_statements_without_columns_or_rows(
        vec![SimpleMessage::CommandComplete],
        Vec::new()
    )]
    #[case::returns_empty_for_no_messages(Vec::new(), Vec::new())]
    fn test_collect_result_sets(
        #[case] messages: Vec<SimpleMessage>,
        #[case] expected: Vec<ResultSet>,
    ) {
        assert_eq!(collect_result_sets(messages), expected);
    }
}
