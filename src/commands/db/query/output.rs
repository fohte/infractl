use super::pg::ResultSet;

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

    let num_cols = result_set.columns.len();
    let rows: Vec<Vec<String>> = result_set
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| v.clone().unwrap_or_else(|| NULL_DISPLAY.to_string()))
                .collect()
        })
        .collect();

    let widths: Vec<usize> = (0..num_cols)
        .map(|i| {
            rows.iter()
                .map(|r| r[i].chars().count())
                .chain(std::iter::once(result_set.columns[i].chars().count()))
                .max()
                .unwrap_or(0)
        })
        .collect();

    let mut out = format_row(&result_set.columns, &widths);
    out.push('\n');
    for row in &rows {
        out.push_str(&format_row(row, &widths));
        out.push('\n');
    }
    out
}

/// Join columns with two-space gaps, padding every column but the last
/// (which is left ragged to avoid trailing whitespace).
fn format_row(cols: &[String], widths: &[usize]) -> String {
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

    fn result_set(columns: &[&str], rows: Vec<Vec<Option<&str>>>) -> ResultSet {
        ResultSet {
            columns: columns.iter().map(|c| c.to_string()).collect(),
            rows: rows
                .into_iter()
                .map(|row| row.into_iter().map(|v| v.map(str::to_string)).collect())
                .collect(),
        }
    }

    #[test]
    fn format_table_empty_reports_ok() {
        assert_eq!(format_table(&[]), "OK\n");
    }

    #[test]
    fn format_table_renders_single_result_set() {
        let result_sets = vec![result_set(
            &["id", "name"],
            vec![vec![Some("1"), Some("alice")], vec![Some("2"), Some("bob")]],
        )];

        assert_eq!(
            format_table(&result_sets),
            indoc! {"
                id  name
                1   alice
                2   bob
            "}
        );
    }

    #[test]
    fn format_table_renders_null_as_literal_and_no_rows_as_message() {
        let result_sets = vec![
            result_set(&["id", "name"], vec![vec![Some("1"), None]]),
            result_set(&["id"], vec![]),
        ];

        assert_eq!(
            format_table(&result_sets),
            indoc! {"
                id  name
                1   NULL
                (0 rows)
            "}
        );
    }

    #[test]
    fn format_table_aligns_columns_for_multibyte_values() {
        let result_sets = vec![result_set(&["cluster"], vec![vec![Some("本番環境")]])];

        assert_eq!(
            format_table(&result_sets),
            indoc! {"
                cluster
                本番環境
            "}
        );
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
