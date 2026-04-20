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
    fn parse_empty_xdc() {
        let xdc = XdcConstraints::from_raw("");
        assert_eq!(xdc.port_count(), 0);
    }

    #[test]
    fn parse_clock_pin() {
        let raw =
            "create_clock -add -name sys_clk -period 83.333 -waveform {0 41.666} [get_ports clk]";
        let xdc = XdcConstraints::from_raw(raw);
        assert_eq!(
            xdc.port_count(),
            0,
            "create_clock has no PACKAGE_PIN, so it produces no entry"
        );
    }

    #[test]
    fn clock_ports_method() {
        let raw = "set_property -dict { PACKAGE_PIN E3 IOSTANDARD LVCMOS33 } [get_ports clk]\nset_property -dict { PACKAGE_PIN C18 IOSTANDARD LVCMOS33 } [get_ports rst_n]\n";
        let xdc = XdcConstraints::from_raw(raw);
        let clocks = xdc.clock_ports();
        assert_eq!(clocks.len(), 0, "no create_clock entries have PACKAGE_PIN");
    }

    #[test]
    fn parse_single_pin() {
        let raw = "set_property -dict { PACKAGE_PIN E3 IOSTANDARD LVCMOS33 } [get_ports clk]";
        let xdc = XdcConstraints::from_raw(raw);
        assert_eq!(xdc.port_count(), 1);
        assert_eq!(xdc.entries[0].package_pin, "E3");
        assert_eq!(xdc.entries[0].iostandard, "LVCMOS33");
        assert_eq!(xdc.entries[0].port, "clk");
        assert!(!xdc.entries[0].is_clock);
    }

    #[test]
    fn parse_port_with_braces() {
        let raw =
            "set_property -dict { PACKAGE_PIN H17 IOSTANDARD LVCMOS33 } [get_ports { led[0] }]";
        let xdc = XdcConstraints::from_raw(raw);
        assert_eq!(xdc.port_count(), 1);
        assert_eq!(xdc.entries[0].package_pin, "H17");
        assert!(
            xdc.entries[0].port.contains("led"),
            "port should contain led, got {}",
            xdc.entries[0].port
        );
    }

    #[test]
    fn parse_non_pin_lines_skipped() {
        let raw = "some_random_line\nanother line\n";
        let xdc = XdcConstraints::from_raw(raw);
        assert_eq!(xdc.port_count(), 0);
    }

    #[test]
    fn parse_mixed_valid_and_invalid() {
        let raw = "# header\nset_property -dict { PACKAGE_PIN E3 IOSTANDARD LVCMOS33 } [get_ports clk]\nnot a pin line\nset_property -dict { PACKAGE_PIN C18 IOSTANDARD LVCMOS33 } [get_ports rst_n]\n";
        let xdc = XdcConstraints::from_raw(raw);
        assert_eq!(xdc.port_count(), 2);
    }

    #[test]
    fn source_preserved() {
        let raw = "set_property -dict { PACKAGE_PIN E3 IOSTANDARD LVCMOS33 } [get_ports clk]";
        let xdc = XdcConstraints::from_raw(raw);
        assert_eq!(xdc.source, raw);
    }

    #[test]
    fn serialize_deserialize_xdc_entry() {
        let entry = XdcEntry {
            package_pin: "E3".into(),
            iostandard: "LVCMOS33".into(),
            port: "clk".into(),
            is_clock: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: XdcEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.package_pin, "E3");
        assert_eq!(back.port, "clk");
        assert!(!back.is_clock);
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

    #[test]
    fn preprocess_strips_current_design() {
        let raw = "set_property something [current_design]\nkeep_me\n";
        let processed = preprocess_for_nextpnr(raw);
        assert!(!processed.contains("current_design"));
        assert!(processed.contains("keep_me"));
    }

    #[test]
    fn preprocess_strips_double_slash_comments() {
        let raw = "// C++ style comment\nkeep\n";
        let processed = preprocess_for_nextpnr(raw);
        assert!(!processed.contains("// C++"));
        assert!(processed.contains("keep"));
    }

    #[test]
    fn preprocess_strips_blank_lines() {
        let raw = "line1\n\n\nline2\n";
        let processed = preprocess_for_nextpnr(raw);
        assert!(!processed.contains("\n\n"));
    }

    #[test]
    fn preprocess_removes_pullup() {
        let raw = "set_property -dict { PACKAGE_PIN E3 IOSTANDARD LVCMOS33 PULLUP true } [get_ports clk]\n";
        let processed = preprocess_for_nextpnr(raw);
        assert!(!processed.contains("PULLUP"));
        assert!(processed.contains("E3"));
    }

    #[test]
    fn preprocess_normalizes_braces() {
        let raw = "set_property -dict { PACKAGE_PIN E3 IOSTANDARD LVCMOS33 } [get_ports { clk }]\n";
        let processed = preprocess_for_nextpnr(raw);
        assert!(
            processed.contains("[get_ports clk]"),
            "should normalize braces, got: {}",
            processed
        );
    }

    #[test]
    fn preprocess_empty_input() {
        let processed = preprocess_for_nextpnr("");
        assert!(processed.is_empty() || processed.trim().is_empty());
    }

    #[test]
    fn minimal_qmtech_xdc_has_required_pins() {
        let xdc = XdcConstraints::from_raw(minimal_qmtech_xdc());
        let ports: Vec<&str> = xdc.entries.iter().map(|e| e.port.as_str()).collect();
        assert!(ports.contains(&"clk"), "missing clk");
        assert!(ports.contains(&"rst_n"), "missing rst_n");
        assert!(ports.contains(&"uart_rx"), "missing uart_rx");
        assert!(ports.contains(&"uart_tx"), "missing uart_tx");
    }

    #[test]
    fn minimal_qmtech_xdc_all_lvcmos33() {
        let xdc = XdcConstraints::from_raw(minimal_qmtech_xdc());
        for entry in &xdc.entries {
            assert_eq!(
                entry.iostandard, "LVCMOS33",
                "pin {} should be LVCMOS33",
                entry.package_pin
            );
        }
    }

    #[test]
    fn extract_port_no_get_ports_returns_none() {
        assert!(extract_port("no get_ports here").is_none());
    }

    #[test]
    fn extract_value_missing_key_returns_none() {
        assert!(extract_value("nothing here", "PACKAGE_PIN").is_none());
    }
}
