use anyhow::Result;

/// Markdown table parser and renderer for #143
#[derive(Debug)]
pub struct Table {
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub task: String,
    pub agent: String,
    pub status: String,
    pub bpb: Option<f64>,
    pub ref_issue: String,
}

impl Table {
    pub fn parse(markdown: &str) -> Result<Self> {
        let rows = markdown
            .lines()
            .skip_while(|l| !l.contains("Task"))
            .skip(1) // header
            .skip_while(|l| l.contains("---"))
            .take_while(|l| l.starts_with("|"))
            .filter_map(|l| Self::parse_row(l))
            .collect();
        
        Ok(Table { rows })
    }

    fn parse_row(line: &str) -> Option<Row> {
        let cells: Vec<&str> = line
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(|s| s.trim())
            .collect();
        
        if cells.len() < 5 {
            return None;
        }

        Some(Row {
            task: cells[0].to_string(),
            agent: cells[1].to_string(),
            status: cells[2].to_string(),
            bpb: cells[3].parse().ok(),
            ref_issue: cells.get(4).unwrap_or(&"").to_string(),
        })
    }

    pub fn update_row(&mut self, agent: &str, status: &str, bpb: Option<f64>) -> bool {
        for row in &mut self.rows {
            if row.agent == agent {
                row.status = status.to_string();
                row.bpb = bpb;
                return true;
            }
        }
        false
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("| Task                                    | Agent   | Status       | BPB    | Ref        |\n");
        out.push_str("|-----------------------------------------|---------|--------------|--------|------------|\n");
        
        for row in &self.rows {
            out.push_str(&format!(
                "| {:39} | {:7} | {:12} | {:6} | {:10} |\n",
                row.task,
                row.agent,
                row.status,
                row.bpb.map(|b| b.to_string()).unwrap_or_else(|| "—".to_string()),
                row.ref_issue
            ));
        }
        
        out
    }
}
