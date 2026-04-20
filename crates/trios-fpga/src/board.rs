use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownBoard {
    QmtechA100t,
    ArtyA7_100t,
    ArtyA7_35t,
}

impl KnownBoard {
    pub fn fpga_part(&self) -> &'static str {
        match self {
            Self::QmtechA100t => "xc7a100tcsg324",
            Self::ArtyA7_100t => "xc7a100tcsg324",
            Self::ArtyA7_35t => "xc7a35tcsg324",
        }
    }

    pub fn openfpgaloader_board(&self) -> &'static str {
        match self {
            Self::QmtechA100t => "qmtech_xc7a100t",
            Self::ArtyA7_100t => "arty_a7_100t",
            Self::ArtyA7_35t => "arty_a7_35t",
        }
    }

    pub fn clock_mhz(&self) -> u32 {
        match self {
            Self::QmtechA100t => 12,
            Self::ArtyA7_100t => 100,
            Self::ArtyA7_35t => 100,
        }
    }

    pub fn default_xdc(&self) -> &'static str {
        match self {
            Self::QmtechA100t => "specs/fpga/constraints/qmtech_a100t.xdc",
            Self::ArtyA7_100t | Self::ArtyA7_35t => "specs/fpga/constraints/arty_a7.xdc",
        }
    }

    pub fn chipdb_name(&self) -> &'static str {
        match self {
            Self::QmtechA100t => "xc7a100tcsg324-1",
            Self::ArtyA7_100t => "xc7a100tcsg324-1",
            Self::ArtyA7_35t => "xc7a35tcsg324-1",
        }
    }
}

impl fmt::Display for KnownBoard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QmtechA100t => write!(f, "qmtech-a100t"),
            Self::ArtyA7_100t => write!(f, "arty-a7-100t"),
            Self::ArtyA7_35t => write!(f, "arty-a7-35t"),
        }
    }
}

impl std::str::FromStr for KnownBoard {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "qmtech-a100t" => Ok(Self::QmtechA100t),
            "arty-a7-100t" => Ok(Self::ArtyA7_100t),
            "arty-a7-35t" => Ok(Self::ArtyA7_35t),
            "arty-a7" => Ok(Self::ArtyA7_35t),
            _ => Err(format!("unknown board: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardProfile {
    pub board: KnownBoard,
    pub fpga_part: String,
    pub package: String,
    pub speedgrade: u8,
    pub clock_hz: u64,
    pub uart_baud: u32,
    pub led_count: usize,
    pub has_spi: bool,
    pub has_mac_debug: bool,
}

impl BoardProfile {
    pub fn qmtech_a100t() -> Self {
        Self {
            board: KnownBoard::QmtechA100t,
            fpga_part: "xc7a100tcsg324".into(),
            package: "csg324".into(),
            speedgrade: 1,
            clock_hz: 12_000_000,
            uart_baud: 115200,
            led_count: 8,
            has_spi: true,
            has_mac_debug: true,
        }
    }

    pub fn arty_a7_100t() -> Self {
        Self {
            board: KnownBoard::ArtyA7_100t,
            fpga_part: "xc7a100tcsg324".into(),
            package: "csg324".into(),
            speedgrade: 1,
            clock_hz: 100_000_000,
            uart_baud: 115200,
            led_count: 4,
            has_spi: true,
            has_mac_debug: false,
        }
    }

    pub fn arty_a7_35t() -> Self {
        Self {
            board: KnownBoard::ArtyA7_35t,
            fpga_part: "xc7a35tcsg324".into(),
            package: "csg324".into(),
            speedgrade: 1,
            clock_hz: 100_000_000,
            uart_baud: 115200,
            led_count: 4,
            has_spi: true,
            has_mac_debug: false,
        }
    }

    pub fn from_known(board: KnownBoard) -> Self {
        match board {
            KnownBoard::QmtechA100t => Self::qmtech_a100t(),
            KnownBoard::ArtyA7_100t => Self::arty_a7_100t(),
            KnownBoard::ArtyA7_35t => Self::arty_a7_35t(),
        }
    }

    pub fn heartbeat_divider(&self) -> u32 {
        let target_hz = 1.0;
        (self.clock_hz as f64 / target_hz) as u32
    }

    pub fn heartbeat_counter_width(&self) -> u32 {
        let divider = self.heartbeat_divider();
        32 - divider.leading_zeros()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qmtech_profile() {
        let p = BoardProfile::qmtech_a100t();
        assert_eq!(p.fpga_part, "xc7a100tcsg324");
        assert_eq!(p.clock_hz, 12_000_000);
        assert_eq!(p.led_count, 8);
        assert!(p.has_spi);
        assert!(p.has_mac_debug);
    }

    #[test]
    fn qmtech_known_board_accessors() {
        let b = KnownBoard::QmtechA100t;
        assert_eq!(b.fpga_part(), "xc7a100tcsg324");
        assert_eq!(b.openfpgaloader_board(), "qmtech_xc7a100t");
        assert_eq!(b.clock_mhz(), 12);
        assert_eq!(b.chipdb_name(), "xc7a100tcsg324-1");
        assert!(b.default_xdc().contains("qmtech"));
    }

    #[test]
    fn arty_100t_profile() {
        let p = BoardProfile::arty_a7_100t();
        assert_eq!(p.fpga_part, "xc7a100tcsg324");
        assert_eq!(p.clock_hz, 100_000_000);
        assert_eq!(p.led_count, 4);
        assert!(p.has_spi);
        assert!(!p.has_mac_debug);
    }

    #[test]
    fn arty_35t_profile() {
        let p = BoardProfile::arty_a7_35t();
        assert_eq!(p.fpga_part, "xc7a35tcsg324");
        assert_eq!(p.clock_hz, 100_000_000);
        assert_eq!(p.led_count, 4);
        assert!(!p.has_mac_debug);
    }

    #[test]
    fn from_known_roundtrip() {
        assert_eq!(
            BoardProfile::from_known(KnownBoard::QmtechA100t).board,
            KnownBoard::QmtechA100t
        );
        assert_eq!(
            BoardProfile::from_known(KnownBoard::ArtyA7_100t).board,
            KnownBoard::ArtyA7_100t
        );
        assert_eq!(
            BoardProfile::from_known(KnownBoard::ArtyA7_35t).board,
            KnownBoard::ArtyA7_35t
        );
    }

    #[test]
    fn heartbeat_divider_qmtech() {
        let p = BoardProfile::qmtech_a100t();
        assert_eq!(p.heartbeat_divider(), 12_000_000);
    }

    #[test]
    fn heartbeat_divider_arty() {
        let p = BoardProfile::arty_a7_100t();
        assert_eq!(p.heartbeat_divider(), 100_000_000);
    }

    #[test]
    fn heartbeat_counter_width_qmtech() {
        let p = BoardProfile::qmtech_a100t();
        let w = p.heartbeat_counter_width();
        assert!(
            w >= 24,
            "counter width {} should be >= 24 for ~1Hz at 12MHz",
            w
        );
    }

    #[test]
    fn heartbeat_counter_width_arty() {
        let p = BoardProfile::arty_a7_100t();
        let w = p.heartbeat_counter_width();
        assert!(
            w >= 27,
            "counter width {} should be >= 27 for ~1Hz at 100MHz",
            w
        );
    }

    #[test]
    fn board_display() {
        assert_eq!(KnownBoard::QmtechA100t.to_string(), "qmtech-a100t");
        assert_eq!(KnownBoard::ArtyA7_100t.to_string(), "arty-a7-100t");
        assert_eq!(KnownBoard::ArtyA7_35t.to_string(), "arty-a7-35t");
    }

    #[test]
    fn board_from_str_roundtrip() {
        assert_eq!(
            "qmtech-a100t".parse::<KnownBoard>().unwrap(),
            KnownBoard::QmtechA100t
        );
        assert_eq!(
            "arty-a7-100t".parse::<KnownBoard>().unwrap(),
            KnownBoard::ArtyA7_100t
        );
        assert_eq!(
            "arty-a7-35t".parse::<KnownBoard>().unwrap(),
            KnownBoard::ArtyA7_35t
        );
    }

    #[test]
    fn arty_a7_alias() {
        assert_eq!(
            "arty-a7".parse::<KnownBoard>().unwrap(),
            KnownBoard::ArtyA7_35t
        );
    }

    #[test]
    fn board_from_str_unknown_fails() {
        let result = "unknown-board".parse::<KnownBoard>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown board"));
    }

    #[test]
    fn board_from_str_empty_fails() {
        let result = "".parse::<KnownBoard>();
        assert!(result.is_err());
    }

    #[test]
    fn board_serialize_deserialize() {
        let board = KnownBoard::QmtechA100t;
        let json = serde_json::to_string(&board).unwrap();
        let back: KnownBoard = serde_json::from_str(&json).unwrap();
        assert_eq!(back, board);
    }

    #[test]
    fn board_equality() {
        assert_eq!(KnownBoard::QmtechA100t, KnownBoard::QmtechA100t);
        assert_ne!(KnownBoard::QmtechA100t, KnownBoard::ArtyA7_100t);
    }

    #[test]
    fn all_boards_have_consistent_xdc() {
        for board in [
            KnownBoard::QmtechA100t,
            KnownBoard::ArtyA7_100t,
            KnownBoard::ArtyA7_35t,
        ] {
            assert!(!board.default_xdc().is_empty(), "{:?} has no XDC", board);
            assert!(
                board.default_xdc().ends_with(".xdc"),
                "{:?} XDC not .xdc",
                board
            );
        }
    }

    #[test]
    fn all_boards_have_chipdb() {
        for board in [
            KnownBoard::QmtechA100t,
            KnownBoard::ArtyA7_100t,
            KnownBoard::ArtyA7_35t,
        ] {
            let chipdb = board.chipdb_name();
            assert!(
                chipdb.starts_with("xc7a"),
                "chipdb {} should start with xc7a",
                chipdb
            );
        }
    }

    #[test]
    fn profile_uart_baud_standard() {
        for board in [
            KnownBoard::QmtechA100t,
            KnownBoard::ArtyA7_100t,
            KnownBoard::ArtyA7_35t,
        ] {
            let p = BoardProfile::from_known(board);
            assert_eq!(p.uart_baud, 115200, "{:?} baud should be 115200", board);
        }
    }

    #[test]
    fn profile_package_csg324() {
        for board in [
            KnownBoard::QmtechA100t,
            KnownBoard::ArtyA7_100t,
            KnownBoard::ArtyA7_35t,
        ] {
            let p = BoardProfile::from_known(board);
            assert_eq!(p.package, "csg324");
            assert_eq!(p.speedgrade, 1);
        }
    }
}
