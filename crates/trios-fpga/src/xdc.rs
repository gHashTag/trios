use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XdcEntry {
    pub package_pin: String,
    pub iostandard: String,
    pub port: String,
    pub is_clock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XdcConstraints {
    pub entries: Vec<XdcEntry>,
    pub source: String,
}

impl XdcConstraints {
    pub fn from_raw(raw: &str) -> Self {
        let mut entries = Vec::new();
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }
            if let Some(pin) = parse_pin_line(trimmed) {
                entries.push(pin);
            }
        }
        XdcConstraints {
            entries,
            source: raw.to_string(),
        }
    }

    pub fn port_count(&self) -> usize {
        self.entries.len()
    }

    pub fn clock_ports(&self) -> Vec<&XdcEntry> {
        self.entries.iter().filter(|e| e.is_clock).collect()
    }
}

fn parse_pin_line(line: &str) -> Option<XdcEntry> {
    let pin = extract_value(line, "PACKAGE_PIN")?;
    let iostandard = extract_value(line, "IOSTANDARD").unwrap_or_else(|| "LVCMOS33".into());
    let port = extract_port(line)?;
    let is_clock = line.contains("create_clock");

    Some(XdcEntry {
        package_pin: pin,
        iostandard,
        port,
        is_clock,
    })
}

fn extract_value(line: &str, key: &str) -> Option<String> {
    let pattern = format!("{} ", key);
    let start = line.find(&pattern)?;
    let rest = &line[start + pattern.len()..];
    let val = rest.split_whitespace().next()?;
    Some(
        val.trim_matches(|c: char| c == '{' || c == '}' || c == '"')
            .to_string(),
    )
}

fn extract_port(line: &str) -> Option<String> {
    let start = line.find("[get_ports")?;
    let rest = &line[start + "[get_ports".len()..];
    let port = rest
        .trim_start_matches(|c| c == ' ' || c == '{')
        .split(|c: char| c == '}' || c == ']')
        .next()?;
    Some(port.trim().to_string())
}

pub fn preprocess_for_nextpnr(raw: &str) -> String {
    let mut out = String::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("set_false_path") || trimmed.contains("[current_design]") {
            continue;
        }
        let l = if trimmed.contains("PULLUP") {
            trimmed.replace("PULLUP true", "").replace("  ", " ")
        } else {
            trimmed.to_string()
        };
        let l = l
            .replace("[get_ports { ", "[get_ports ")
            .replace(" }]", "]");
        out.push_str(&l);
        out.push('\n');
    }
    out
}

pub fn minimal_qmtech_xdc() -> &'static str {
    r#"# nextpnr-compatible XDC for minimal design (prjxray-verified pins)
 set_property -dict { PACKAGE_PIN E3    IOSTANDARD LVCMOS33 } [get_ports clk]
 create_clock -add -name sys_clk -period 83.333 -waveform {0 41.666} [get_ports clk]
 set_property -dict { PACKAGE_PIN C18   IOSTANDARD LVCMOS33 } [get_ports rst_n]
set_property -dict { PACKAGE_PIN T14   IOSTANDARD LVCMOS33 } [get_ports uart_rx]
set_property -dict { PACKAGE_PIN T15   IOSTANDARD LVCMOS33 } [get_ports uart_tx]
set_property -dict { PACKAGE_PIN H17   IOSTANDARD LVCMOS33 } [get_ports led[0]]
set_property -dict { PACKAGE_PIN K15   IOSTANDARD LVCMOS33 } [get_ports led[1]]
set_property -dict { PACKAGE_PIN J13   IOSTANDARD LVCMOS33 } [get_ports led[2]]
set_property -dict { PACKAGE_PIN N14   IOSTANDARD LVCMOS33 } [get_ports led[3]]
set_property -dict { PACKAGE_PIN R18   IOSTANDARD LVCMOS33 } [get_ports led[4]]
set_property -dict { PACKAGE_PIN U18   IOSTANDARD LVCMOS33 } [get_ports led[5]]
set_property -dict { PACKAGE_PIN T13   IOSTANDARD LVCMOS33 } [get_ports led[6]]
set_property -dict { PACKAGE_PIN T11   IOSTANDARD LVCMOS33 } [get_ports led[7]]
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_xdc() {
        let xdc = XdcConstraints::from_raw(minimal_qmtech_xdc());
        assert!(
            xdc.port_count() >= 10,
            "should have at least 10 pins, got {}",
            xdc.port_count()
        );
        let clk = xdc.entries.iter().find(|e| e.port == "clk");
        assert!(clk.is_some(), "should have clk port");
        assert_eq!(clk.unwrap().package_pin, "E3");
    }

    #[test]
    fn preprocess_strips_comments() {
        let raw = "# comment\nset_property -dict { PACKAGE_PIN E3 IOSTANDARD LVCMOS33 } [get_ports clk]\n";
        let processed = preprocess_for_nextpnr(raw);
        assert!(!processed.contains("# comment"));
        assert!(processed.contains("E3"));
    }

    #[test]
    fn preprocess_strips_false_paths() {
        let raw = "set_false_path -from [get_pins clk]\nsomething_else\n";
        let processed = preprocess_for_nextpnr(raw);
        assert!(!processed.contains("set_false_path"));
        assert!(processed.contains("something_else"));
    }
}
