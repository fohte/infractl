/// Renders a header and rows as a left-aligned table. Columns are separated
/// by two-space gaps; every column but the last is padded, so the last is
/// left ragged to avoid trailing whitespace. Widths are computed in chars,
/// not bytes, so multibyte values still line up.
pub fn render(header: &[String], rows: &[Vec<String>]) -> String {
    let widths: Vec<usize> = (0..header.len())
        .map(|i| {
            rows.iter()
                .map(|r| r[i].chars().count())
                .chain(std::iter::once(header[i].chars().count()))
                .max()
                .unwrap_or(0)
        })
        .collect();

    let mut out = format_row(header, &widths);
    out.push('\n');
    for row in rows {
        out.push_str(&format_row(row, &widths));
        out.push('\n');
    }
    out
}

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

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn render_aligns_columns_across_multiple_rows() {
        let header = strings(&["NAME", "CONTEXT"]);
        let rows = vec![
            strings(&["mastodon", "home-k8s"]),
            strings(&["tq", "(current)"]),
        ];

        assert_eq!(
            render(&header, &rows),
            indoc! {"
                NAME      CONTEXT
                mastodon  home-k8s
                tq        (current)
            "}
        );
    }

    #[test]
    fn render_aligns_columns_for_multibyte_values() {
        let header = strings(&["CLUSTER"]);
        let rows = vec![strings(&["本番環境"])];

        assert_eq!(
            render(&header, &rows),
            indoc! {"
                CLUSTER
                本番環境
            "}
        );
    }

    #[test]
    fn render_header_only_for_no_rows() {
        let header = strings(&["NAME"]);

        assert_eq!(render(&header, &[]), "NAME\n");
    }
}
