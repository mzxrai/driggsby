use std::cmp;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Align {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub struct Column<'a> {
    pub name: &'a str,
    pub align: Align,
}

const INDENT: usize = 2;
const COLUMN_GAP: usize = 2;
const MIN_TABLE_COLUMN_WIDTH: usize = 8;

pub fn terminal_width() -> usize {
    let from_env = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(120);
    cmp::max(from_env, 40)
}

pub fn key_value_rows(entries: &[(&str, String)], indent: usize) -> Vec<String> {
    if entries.is_empty() {
        return Vec::new();
    }

    let label_width = entries
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(0);
    let padding = " ".repeat(indent);

    entries
        .iter()
        .map(|(label, value)| format!("{padding}{label:<label_width$}  {value}"))
        .collect()
}

pub fn render_table_or_blocks(
    columns: &[Column<'_>],
    rows: &[Vec<String>],
    max_width: usize,
    block_label: &str,
) -> Vec<String> {
    if columns.is_empty() {
        return Vec::new();
    }

    if should_fallback_to_blocks(columns.len(), max_width) {
        return render_blocks(columns, rows, block_label);
    }

    let natural = natural_column_widths(columns, rows);
    let minimums = columns
        .iter()
        .map(|column| cmp::max(column.name.len(), MIN_TABLE_COLUMN_WIDTH))
        .collect::<Vec<usize>>();
    let available = max_width.saturating_sub(INDENT);
    let gap_total = COLUMN_GAP * columns.len().saturating_sub(1);
    let budget = available.saturating_sub(gap_total);

    let Some(widths) = fit_widths_to_budget(&natural, &minimums, budget) else {
        return render_blocks(columns, rows, block_label);
    };

    let mut output = Vec::new();
    output.push(format_row(
        columns,
        &columns
            .iter()
            .map(|c| c.name.to_string())
            .collect::<Vec<_>>(),
        &widths,
    ));

    for row in rows {
        let wrapped = wrap_row(row, &widths);
        let max_lines = wrapped.iter().map(Vec::len).max().unwrap_or(1);

        for line_index in 0..max_lines {
            let mut line_cells = Vec::with_capacity(columns.len());
            for (column_index, _column) in columns.iter().enumerate() {
                let cell_value = wrapped
                    .get(column_index)
                    .and_then(|chunks| chunks.get(line_index))
                    .cloned()
                    .unwrap_or_default();
                line_cells.push(cell_value);
            }
            output.push(format_row(columns, &line_cells, &widths));
        }
    }

    output
}

fn should_fallback_to_blocks(column_count: usize, max_width: usize) -> bool {
    let minimum = INDENT
        + (MIN_TABLE_COLUMN_WIDTH * column_count)
        + (COLUMN_GAP * column_count.saturating_sub(1));
    max_width < minimum
}

fn natural_column_widths(columns: &[Column<'_>], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths = columns
        .iter()
        .map(|column| column.name.len())
        .collect::<Vec<usize>>();

    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if let Some(slot) = widths.get_mut(index) {
                *slot = cmp::max(*slot, value.len());
            }
        }
    }

    widths
}

fn fit_widths_to_budget(
    natural: &[usize],
    minimums: &[usize],
    budget: usize,
) -> Option<Vec<usize>> {
    if natural.len() != minimums.len() {
        return None;
    }

    let min_total = minimums.iter().sum::<usize>();
    if min_total > budget {
        return None;
    }

    let mut widths = natural.to_vec();
    let mut total = widths.iter().sum::<usize>();
    if total <= budget {
        return Some(widths);
    }

    while total > budget {
        let mut reduced = false;

        for (index, width) in widths.iter_mut().enumerate() {
            if total <= budget {
                break;
            }

            let floor = *minimums.get(index).unwrap_or(&0);
            if *width > floor {
                *width -= 1;
                total -= 1;
                reduced = true;
            }
        }

        if !reduced {
            return None;
        }
    }

    Some(widths)
}

fn wrap_row(row: &[String], widths: &[usize]) -> Vec<Vec<String>> {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let value = row.get(index).map(String::as_str).unwrap_or("");
            wrap_text(value, *width)
        })
        .collect()
}

fn format_row(columns: &[Column<'_>], cells: &[String], widths: &[usize]) -> String {
    let mut pieces = Vec::with_capacity(columns.len());
    for (index, column) in columns.iter().enumerate() {
        let width = *widths.get(index).unwrap_or(&MIN_TABLE_COLUMN_WIDTH);
        let value = cells.get(index).cloned().unwrap_or_default();

        let piece = match column.align {
            Align::Left => format!("{value:<width$}"),
            Align::Right => format!("{value:>width$}"),
        };
        pieces.push(piece);
    }

    format!("{}{}", " ".repeat(INDENT), pieces.join("  "))
}

fn wrap_text(value: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![value.to_string()];
    }
    if value.len() <= width {
        return vec![value.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in value.split_whitespace() {
        if current.is_empty() {
            if word.len() <= width {
                current.push_str(word);
            } else {
                lines.extend(split_long_token(word, width));
            }
            continue;
        }

        let candidate_len = current.len() + 1 + word.len();
        if candidate_len <= width {
            current.push(' ');
            current.push_str(word);
            continue;
        }

        lines.push(current);
        current = String::new();

        if word.len() <= width {
            current.push_str(word);
        } else {
            lines.extend(split_long_token(word, width));
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        return split_long_token(value, width);
    }

    lines
}

fn split_long_token(token: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![token.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;

    for ch in token.chars() {
        current.push(ch);
        current_len += 1;

        if current_len == width {
            chunks.push(std::mem::take(&mut current));
            current_len = 0;
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

fn render_blocks(columns: &[Column<'_>], rows: &[Vec<String>], block_label: &str) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }

    let labels = columns
        .iter()
        .map(|column| format!("{}:", column.name))
        .collect::<Vec<String>>();
    let label_width = labels.iter().map(|label| label.len()).max().unwrap_or(0);

    let mut output = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        output.push(format!("  {block_label} {}:", row_index + 1));

        for (column_index, label) in labels.iter().enumerate() {
            let value = row.get(column_index).cloned().unwrap_or_default();
            output.push(format!("    {label:<label_width$}  {value}"));
        }

        if row_index + 1 < rows.len() {
            output.push(String::new());
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{
        Align, Column, fit_widths_to_budget, key_value_rows, render_table_or_blocks,
        split_long_token,
    };

    #[test]
    fn key_value_rows_align_labels() {
        let rows = key_value_rows(
            &[
                ("Rows read:", "100".to_string()),
                ("Rows invalid:", "0".to_string()),
            ],
            2,
        );

        assert_eq!(rows[0], "  Rows read:     100");
        assert_eq!(rows[1], "  Rows invalid:  0");
    }

    #[test]
    fn table_renderer_renders_expected_values_when_width_is_sufficient() {
        let columns = [
            Column {
                name: "Merchant",
                align: Align::Left,
            },
            Column {
                name: "Amount",
                align: Align::Right,
            },
        ];
        let rows = vec![vec![
            "VERY LONG MERCHANT NAME THAT MUST WRAP".to_string(),
            "-1234.56 USD".to_string(),
        ]];

        let rendered = render_table_or_blocks(&columns, &rows, 80, "Row");
        assert!(rendered[0].contains("Merchant"));
        assert!(rendered[0].contains("Amount"));
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("VERY LONG MERCHANT NAME THAT MUST WRAP"))
        );
        assert!(rendered.iter().any(|line| line.contains("-1234.56 USD")));
    }

    #[test]
    fn table_renderer_wraps_without_truncating() {
        let columns = [
            Column {
                name: "Merchant",
                align: Align::Left,
            },
            Column {
                name: "Amount",
                align: Align::Right,
            },
        ];
        let rows = vec![vec![
            "VERY LONG MERCHANT NAME THAT MUST WRAP".to_string(),
            "-1234.56 USD".to_string(),
        ]];

        let rendered = render_table_or_blocks(&columns, &rows, 44, "Row");
        assert!(rendered[0].contains("Merchant"));
        assert!(rendered[0].contains("Amount"));
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("VERY LONG MERCHANT"))
        );
        assert!(rendered.iter().any(|line| line.contains("THAT")));
        assert!(rendered.iter().any(|line| line.contains("WRAP")));
        assert!(rendered.iter().any(|line| line.contains("-1234.56")));
        assert!(rendered.iter().any(|line| line.contains("USD")));
    }

    #[test]
    fn narrow_width_falls_back_to_blocks() {
        let columns = [
            Column {
                name: "Merchant",
                align: Align::Left,
            },
            Column {
                name: "Amount",
                align: Align::Right,
            },
            Column {
                name: "Reason",
                align: Align::Left,
            },
        ];
        let rows = vec![vec![
            "Coffee".to_string(),
            "-5.00 USD".to_string(),
            "small purchase".to_string(),
        ]];

        let rendered = render_table_or_blocks(&columns, &rows, 20, "Finding");
        assert_eq!(rendered[0], "  Finding 1:");
        assert!(rendered[1].contains("Merchant:"));
        assert!(rendered[2].contains("Amount:"));
        assert!(rendered[3].contains("Reason:"));
    }

    #[test]
    fn fit_widths_respects_column_name_minimums() {
        let natural = [20, 12];
        let minimums = [8, 10];

        let fitted = fit_widths_to_budget(&natural, &minimums, 19);
        assert!(fitted.is_some());
        if let Some(widths) = fitted {
            assert_eq!(widths, vec![9, 10]);
        }
    }

    #[test]
    fn table_renderer_falls_back_when_headers_cannot_fit() {
        let columns = [
            Column {
                name: "import_id",
                align: Align::Left,
            },
            Column {
                name: "committed_at",
                align: Align::Left,
            },
            Column {
                name: "rows_invalid",
                align: Align::Left,
            },
            Column {
                name: "source_kind",
                align: Align::Left,
            },
            Column {
                name: "source_ref",
                align: Align::Left,
            },
        ];
        let rows = vec![vec![
            "imp_01KJDDSDBMREJ6F5TG6D3H5PZN".to_string(),
            "1772124681".to_string(),
            "0".to_string(),
            "file".to_string(),
            "tmp/plan13-orchestrated-e2e/runs/run-plan13-rerun-after-plan14/scenarios/U-02/inputs/u02.json".to_string(),
        ]];

        let rendered = render_table_or_blocks(&columns, &rows, 54, "Row");
        assert_eq!(rendered[0], "  Row 1:");
        assert!(rendered.iter().any(|line| line.contains("source_ref:")));
    }

    #[test]
    fn split_long_token_handles_unicode_without_panicking() {
        let chunks = split_long_token("éééé", 3);
        assert_eq!(chunks, vec!["ééé".to_string(), "é".to_string()]);
    }
}
