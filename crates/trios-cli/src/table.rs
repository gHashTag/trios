//! Markdown table parser and renderer for #143

use anyhow::Result;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TableRow {
    pub cells: Vec<String>,
}

impl TableRow {
    pub fn agent(&self) -> Option<&str> {
        self.cells
            .iter()
            .find(|c| {
                let c = c.trim();
                [
                    "FOXTROT", "HOTEL", "ALFA", "BRAVO", "CHARLIE", "DELTA", "ECHO", "GOLF",
                    "INDIA", "JULIETT", "KILO", "LIMA", "MIKE", "NOVEMBER", "OSCAR",
                ]
                .iter()
                .any(|nato| c.contains(nato))
            })
            .map(String::as_str)
    }

    pub fn contains(&self, keyword: &str) -> bool {
        self.cells.iter().any(|c| c.contains(keyword))
    }

    pub fn find_bpb_cell_index(&self) -> Option<usize> {
        for (i, cell) in self.cells.iter().enumerate() {
            let trimmed = cell.trim();
            if trimmed.parse::<f64>().is_ok()
                && trimmed.parse::<f64>().unwrap() > 0.0
                && trimmed.parse::<f64>().unwrap() < 20.0
            {
                return Some(i);
            }
        }
        None
    }
}

pub fn parse_table(markdown: &str, header_hint: &str) -> Result<Vec<TableRow>> {
    let mut found_header = false;
    let mut past_separator = false;
    let mut rows = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if !found_header {
            if trimmed.starts_with('|') && trimmed.contains(header_hint) {
                found_header = true;
            }
            continue;
        }
        if !past_separator {
            if trimmed.starts_with('|') && trimmed.contains('-') {
                past_separator = true;
            }
            continue;
        }
        if !trimmed.starts_with('|') {
            break;
        }
        if trimmed.contains('-') && !trimmed.contains(|c: char| c.is_alphanumeric()) {
            continue;
        }
        if let Some(row) = parse_row(line) {
            rows.push(row);
        }
    }

    Ok(rows)
}

fn parse_row(line: &str) -> Option<TableRow> {
    let cells: Vec<String> = line
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|s| s.trim().to_string())
        .collect();

    if cells.len() < 2 {
        return None;
    }

    Some(TableRow { cells })
}

fn render_row_from_cells(cells: &[String]) -> String {
    let rendered: Vec<String> = cells.iter().map(|c| format!(" {} ", c)).collect();
    format!("|{}|", rendered.join("|"))
}

pub fn update_table(markdown: &str, agent: &str, status: &str, bpb: Option<f64>) -> Result<String> {
    let mut lines: Vec<String> = markdown.lines().map(String::from).collect();
    let mut modified = false;

    for line in lines.iter_mut() {
        if !line.trim().starts_with('|') || line.contains("---") {
            continue;
        }

        if let Some(mut row) = parse_row(line) {
            if let Some(cell) = row
                .cells
                .iter()
                .find(|c| c.trim() == agent || c.contains(agent))
            {
                if cell.trim() == agent || cell.trim().starts_with("**") && cell.contains(agent) {
                    if let Some(bpb_idx) = row.find_bpb_cell_index() {
                        if let Some(bpb_val) = bpb {
                            row.cells[bpb_idx] = format!("{:.4}", bpb_val);
                        }
                    }

                    for cell in row.cells.iter_mut() {
                        let trimmed = cell.trim().to_string();
                        if ["running", "pending", "in-progress", "active"]
                            .contains(&trimmed.as_str())
                        {
                            *cell = format!(" {}", status);
                            break;
                        }
                        if ["complete", "done", "baseline", "finished"].contains(&trimmed.as_str())
                        {
                            *cell = format!(" {}", status);
                            break;
                        }
                    }

                    *line = render_row_from_cells(&row.cells);
                    modified = true;
                }
            }
        }
    }

    if !modified {
        if let Some(pos) = lines
            .iter()
            .position(|l| l.contains("---") && l.trim().starts_with('|'))
        {
            if lines[..pos].iter().any(|l| l.trim().starts_with('|')) {
                let new_row = if let Some(bpb_val) = bpb {
                    format!("| {} | {:.4} | {} |", agent, bpb_val, status)
                } else {
                    format!("| {} | {} |", agent, status)
                };
                lines.insert(pos + 1, new_row);
            }
        }
    }

    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_igla_table() {
        let md = "| Agent | Experiment | BPB | Status |\n|---|---|---|---|\n| FOXTROT | IGLA-STACK-501 | 5.8711 | baseline |\n| HOTEL | IGLA-MUON-502 | 5.8736 | baseline |";
        let rows = parse_table(md, "Agent").unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].contains("FOXTROT"));
        assert!(rows[1].contains("HOTEL"));
    }

    #[test]
    fn test_update_agent_bpb() {
        let md = "| Agent | Experiment | BPB | Status |\n|---|---|---|---|\n| FOXTROT | IGLA-STACK-501 | 5.8711 | baseline |";
        let updated = update_table(md, "FOXTROT", "complete", Some(1.13)).unwrap();
        assert!(updated.contains("1.1300"));
    }

    #[test]
    fn test_parse_empty_table() {
        let md = "| Agent | BPB |\n|---|---|";
        let rows = parse_table(md, "Agent").unwrap();
        assert!(rows.is_empty());
    }
}
