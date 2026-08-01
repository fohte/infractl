use super::pg::ResultSet;
use crate::commands::db::table;

const NULL_DISPLAY: &str = "NULL";

pub fn format_json(result_sets: &[ResultSet]) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(result_sets)?)
}

/// Renders each result set as a left-aligned table, one after another. A
/// statement that returned no columns (e.g. because it matched nothing) is
/// unreachable in practice: `collect_result_sets` already drops
/// column-less, row-less statements, so an empty `result_sets` here means
/// the query produced no result set at all.
pub fn format_table(result_sets: &[ResultSet]) -> String {
    if result_sets.is_empty() {
        return "OK\n".to_string();
    }

    result_sets.iter().map(format_result_set).collect()
}

fn format_result_set(result_set: &ResultSet) -> String {
    if result_set.rows.is_empty() {
        return "(0 rows)\n".to_string();
    }

    let rows: Vec<Vec<String>> = result_set
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| v.clone().unwrap_or_else(|| NULL_DISPLAY.to_string()))
                .collect()
        })
        .collect();

    table::render(&result_set.columns, &rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use rstest::rstest;

    fn result_set(columns: &[&str], rows: Vec<Vec<Option<&str>>>) -> ResultSet {
        ResultSet {
            columns: columns.iter().map(|c| c.to_string()).collect(),
            rows: rows
                .into_iter()
                .map(|row| row.into_iter().map(|v| v.map(str::to_string)).collect())
                .collect(),
        }
    }

    #[rstest]
    #[case::empty(vec![], "OK\n")]
    #[case::single_result_set(
        vec![result_set(
            &["id", "name"],
            vec![vec![Some("1"), Some("alice")], vec![Some("2"), Some("bob")]],
        )],
        indoc! {"
            id  name
            1   alice
            2   bob
        "}
    )]
    #[case::null_and_zero_rows(
        vec![
            result_set(&["id", "name"], vec![vec![Some("1"), None]]),
            result_set(&["id"], vec![]),
        ],
        indoc! {"
            id  name
            1   NULL
            (0 rows)
        "}
    )]
    #[case::multibyte(
        vec![result_set(&["cluster"], vec![vec![Some("本番環境")]])],
        indoc! {"
            cluster
            本番環境
        "}
    )]
    fn test_format_table(#[case] result_sets: Vec<ResultSet>, #[case] expected: &str) {
        assert_eq!(format_table(&result_sets), expected);
    }

    #[test]
    fn format_json_renders_columns_rows_and_null() {
        let result_sets = vec![result_set(
            &["id", "name"],
            vec![vec![Some("1"), Some("alice")], vec![Some("2"), None]],
        )];

        assert_eq!(
            format_json(&result_sets).unwrap(),
            indoc! {r#"
                [
                  {
                    "columns": [
                      "id",
                      "name"
                    ],
                    "rows": [
                      [
                        "1",
                        "alice"
                      ],
                      [
                        "2",
                        null
                      ]
                    ]
                  }
                ]"#}
        );
    }
}
