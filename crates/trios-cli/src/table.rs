//! table.rs — ASCII table rendering for tri report / tri dash
//! No external deps: pure Rust formatting
//! Used to render RINGS dashboard, experiment results, agent roster

/// A simple ASCII table with header row and data rows.
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(headers: Vec<&str>) -> Self {
        Self {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    pub fn add_row(&mut self, row: Vec<&str>) {
        self.rows.push(row.iter().map(|s| s.to_string()).collect());
    }

    /// Render table to String with column auto-sizing
    pub fn render(&self) -> String {
        let ncols = self.headers.len();
        let mut widths = vec![0usize; ncols];

        for (i, h) in self.headers.iter().enumerate() {
            widths[i] = widths[i].max(h.len());
        }
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < ncols {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        let sep = Self::sep_line(&widths);
        let mut out = String::new();
        out.push_str(&sep);
        out.push_str(&Self::fmt_row(&self.headers, &widths));
        out.push_str(&sep);
        for row in &self.rows {
            out.push_str(&Self::fmt_row(row, &widths));
        }
        out.push_str(&sep);
        out
    }

    fn sep_line(widths: &[usize]) -> String {
        let inner: Vec<String> = widths.iter().map(|w| "-".repeat(w + 2)).collect();
        format!("+{}+\n", inner.join("+"))
    }

    fn fmt_row(cells: &[String], widths: &[usize]) -> String {
        let parts: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(i, c)| format!(" {:<width$} ", c, width = widths.get(i).copied().unwrap_or(0)))
            .collect();
        format!("|{}|\n", parts.join("|"))
    }
}

/// Render a BPB progress line: e.g. "IGLA 1.82 ──►── 1.15 target"
pub fn bpb_progress(label: &str, current: f64, target: f64) -> String {
    let pct = ((current - target) / (1.2244 - target)).clamp(0.0, 1.0);
    let filled = (pct * 20.0) as usize;
    let bar: String = (0..20)
        .map(|i| if i < filled { '█' } else { '░' })
        .collect();
    format!("{label}: [{bar}] {current:.4} → {target:.4}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_renders_header() {
        let mut t = Table::new(vec!["Agent", "BPB", "Status"]);
        t.add_row(vec!["GOLF", "1.15", "🟡 IN PROGRESS"]);
        let out = t.render();
        assert!(out.contains("Agent"));
        assert!(out.contains("GOLF"));
    }

    #[test]
    fn bpb_progress_smoke() {
        let line = bpb_progress("IGLA", 1.82, 1.10);
        assert!(line.contains("IGLA"));
        assert!(line.contains("1.1000"));
    }
}
