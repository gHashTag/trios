use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::board::{BoardProfile, KnownBoard};
use crate::FPGA_MODULES;

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub repo_root: PathBuf,
    pub board: KnownBoard,
    pub top: String,
    pub output: String,
    pub smoke: bool,
    pub synth_only: bool,
    pub minimal: bool,
    pub docker: Option<bool>,
    pub use_hir: bool,
    pub nextpnr_path: Option<PathBuf>,
    pub chipdb_path: Option<PathBuf>,
    pub xdc_path: Option<PathBuf>,
    pub fasm2frames_path: Option<PathBuf>,
    pub frames2bit_path: Option<PathBuf>,
    pub prjxray_db_path: Option<PathBuf>,
}

impl BuildConfig {
    pub fn new(repo_root: impl Into<PathBuf>, board: KnownBoard) -> Self {
        Self {
            repo_root: repo_root.into(),
            board,
            top: "zerodsp_top".into(),
            output: "build/fpga".into(),
            smoke: false,
            synth_only: false,
            minimal: false,
            docker: None,
            use_hir: false,
            nextpnr_path: None,
            chipdb_path: None,
            xdc_path: None,
            fasm2frames_path: None,
            frames2bit_path: None,
            prjxray_db_path: None,
        }
    }

    pub fn specs_dir(&self) -> PathBuf {
        self.repo_root.join("specs/fpga")
    }

    pub fn build_dir(&self) -> PathBuf {
        self.repo_root.join(&self.output)
    }

    pub fn gen_dir(&self) -> PathBuf {
        self.build_dir().join("generated")
    }

    pub fn synth_dir(&self) -> PathBuf {
        self.build_dir().join("synth")
    }

    pub fn bitstream_path(&self) -> PathBuf {
        self.build_dir().join(format!("{}.bit", self.top))
    }
}

pub struct BuildPipeline {
    config: BuildConfig,
    profile: BoardProfile,
}

impl BuildPipeline {
    pub fn new(config: BuildConfig) -> Self {
        let profile = BoardProfile::from_known(config.board);
        Self { config, profile }
    }

    pub fn profile(&self) -> &BoardProfile {
        &self.profile
    }

    pub fn config(&self) -> &BuildConfig {
        &self.config
    }

    pub fn run(&self, t27c_path: &Path) -> Result<BuildResult> {
        let gen_dir = self.config.gen_dir();
        let synth_dir = self.config.synth_dir();
        fs::create_dir_all(&gen_dir).context("create gen dir")?;
        fs::create_dir_all(&synth_dir).context("create synth dir")?;

        let gen_count = self.generate_verilog(t27c_path, &gen_dir)?;
        self.generate_top_wrapper(&gen_dir)?;

        if self.config.smoke {
            return Ok(BuildResult {
                generated_modules: gen_count,
                bitstream: None,
                smoke: true,
            });
        }

        self.run_synthesis(&gen_dir, &synth_dir)?;

        if self.config.synth_only {
            return Ok(BuildResult {
                generated_modules: gen_count,
                bitstream: None,
                smoke: false,
            });
        }

        let bitstream = self.run_pnr(&synth_dir)?;

        Ok(BuildResult {
            generated_modules: gen_count,
            bitstream: Some(bitstream),
            smoke: false,
        })
    }

    fn generate_verilog(&self, t27c_path: &Path, gen_dir: &Path) -> Result<u32> {
        let specs_dir = self.config.specs_dir();
        let mut count = 0u32;
        let gen_cmd = if self.config.use_hir {
            "gen-verilog-hir"
        } else {
            "gen-verilog"
        };

        println!(
            "=== FPGA Build: Verilog generation{}===",
            if self.config.use_hir {
                " (HIR path) "
            } else {
                " "
            }
        );

        for module in FPGA_MODULES.iter() {
            let spec_file = specs_dir.join(format!("{}.t27", module));
            let out_file = gen_dir.join(format!("{}.v", module));
            if !spec_file.exists() {
                println!("  SKIP {} (spec not found)", module);
                continue;
            }
            let status = std::process::Command::new(t27c_path)
                .arg(gen_cmd)
                .arg(&spec_file)
                .stdout(std::fs::File::create(&out_file)?)
                .stderr(std::process::Stdio::inherit())
                .status()
                .context(format!("t27c {}", gen_cmd))?;
            if !status.success() {
                anyhow::bail!("t27c {} failed for {}", gen_cmd, module);
            }
            println!("  OK {}.v ({})", module, gen_cmd);
            count += 1;
        }
        Ok(count)
    }

    fn generate_top_wrapper(&self, gen_dir: &Path) -> Result<()> {
        let top = &self.config.top;
        let top_wrapper = gen_dir.join(format!("{}.v", top));

        if self.config.minimal {
            let source = format!(
                "`timescale 1ns / 1ps\n\nmodule {top} (\n\
                 input  wire        clk,\n\
                 input  wire        rst_n,\n\
                 input  wire        uart_rx,\n\
                 output wire        uart_tx,\n\
                 output wire [7:0]  led\n\
                 );\n\
                 wire sys_clk   = clk;\n\
                 wire sys_rst_n = rst_n;\n\n\
                 reg [26:0] heartbeat_ctr;\n\
                 always @(posedge sys_clk) begin\n\
                 if (!sys_rst_n)\n\
                 heartbeat_ctr <= 27'd0;\n\
                 else\n\
                 heartbeat_ctr <= heartbeat_ctr + 1'b1;\n\
                 end\n\n\
                 assign led[0] = heartbeat_ctr[24];\n\
                 assign led[1] = 1'b0;\n\
                 assign led[2] = 1'b0;\n\
                 assign led[3] = 1'b0;\n\
                 assign led[4] = 1'b0;\n\
                 assign led[5] = 1'b0;\n\
                 assign led[6] = 1'b0;\n\
                 assign led[7] = 1'b0;\n\
                 assign uart_tx = uart_rx;\n\
                 endmodule\n"
            );
            fs::write(&top_wrapper, &source)?;
            println!("  OK {}.v (minimal top-level)", top);
        } else {
            let source = self.generate_full_top(top);
            fs::write(&top_wrapper, &source)?;
            println!("  OK {}.v (top-level wrapper)", top);
        }
        Ok(())
    }

    fn generate_full_top(&self, top: &str) -> String {
        let led_assigns = if self.profile.has_mac_debug {
            "assign led[0]     = heartbeat_ctr[24];
    assign led[1]     = mac_ready;
    assign led[2]     = uart_ready;
    assign led[3]     = spi_ready;
    assign led[4]     = bridge_ready;
    assign led[5]     = sys_ready;
    assign led[6]     = 1'b0;
    assign led[7]     = 1'b0;"
        } else {
            "assign led[0]     = heartbeat_ctr[24];
    assign led[1]     = 1'b0;
    assign led[2]     = 1'b0;
    assign led[3]     = 1'b0;"
        };

        let mac_section = if self.profile.has_mac_debug {
            format!(
                "
    wire mac_ready;
    ZeroDSP_MAC u_mac (.clk(sys_clk), .rst_n(sys_rst_n), .en(1'b1), .ready(mac_ready));
"
            )
        } else {
            String::new()
        };

        let mac_outputs = if self.profile.has_mac_debug {
            "
    assign mac_done   = mac_ready;
    assign mac_result = {5'd0, heartbeat_ctr[26:0]};"
        } else {
            ""
        };

        let mac_ports = if self.profile.has_mac_debug {
            ",
    output wire        mac_done,
    output wire [31:0] mac_result"
        } else {
            ""
        };

        format!(
            "`timescale 1ns / 1ps

module {top} (
    input  wire        clk,
    input  wire        rst_n,
    input  wire        uart_rx,
    output wire        uart_tx,
    output wire        spi_cs,
    output wire        spi_sck,
    output wire        spi_mosi,
    input  wire        spi_miso,
    output wire [7:0]  led{mac_ports}
);
    wire sys_clk   = clk;
    wire sys_rst_n = rst_n;

    reg [26:0] heartbeat_ctr;
    always @(posedge sys_clk) begin
        if (!sys_rst_n)
            heartbeat_ctr <= 27'd0;
        else
            heartbeat_ctr <= heartbeat_ctr + 1'b1;
    end
{mac_section}
    wire uart_ready;
    ZeroDSP_UART u_uart (.clk(sys_clk), .rst_n(sys_rst_n), .en(1'b1), .ready(uart_ready));

    wire spi_ready;
    SPI_Master u_spi (.clk(sys_clk), .rst_n(sys_rst_n), .en(1'b1), .ready(spi_ready));

    wire bridge_ready;
    FPGA_Bridge u_bridge (.clk(sys_clk), .rst_n(sys_rst_n), .en(1'b1), .ready(bridge_ready));

    wire sys_ready;
    ZeroDSP_TopLevel u_top_level (.clk(sys_clk), .rst_n(sys_rst_n), .en(1'b1), .ready(sys_ready));

    {led_assigns}
    assign uart_tx    = uart_rx;{mac_outputs}
    assign spi_cs     = 1'b1;
    assign spi_sck    = 1'b0;
    assign spi_mosi   = 1'b0;
endmodule
"
        )
    }

    fn run_synthesis(&self, gen_dir: &Path, synth_dir: &Path) -> Result<()> {
        let use_docker = self.config.docker.unwrap_or_else(|| {
            std::process::Command::new("yosys")
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_err()
        });

        let synth_json = synth_dir.join("synth.json");
        let top = &self.config.top;
        let verilog_files = if self.config.minimal {
            format!("{gen}/{top}.v", gen = gen_dir.display(), top = top)
        } else {
            format!(
                "{gen}/mac.v {gen}/uart.v {gen}/spi.v {gen}/bridge.v {gen}/top_level.v {gen}/{top}.v",
                gen = gen_dir.display(),
                top = top
            )
        };

        let synth_script_content = format!(
            "read_verilog {files}\n\
             hierarchy -check -top {top}\n\
             proc; opt; fsm; opt; memory; opt\n\
             synth_xilinx -top {top}\n\
             write_json {json}\n\
             stat\n",
            files = verilog_files,
            top = top,
            json = synth_json.display(),
        );

        let synth_script = self.config.build_dir().join("synth.ys");
        fs::write(&synth_script, &synth_script_content)?;

        if use_docker {
            println!("=== Synthesizing with Yosys (Docker) ===");
            let status = std::process::Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "-v",
                    &format!("{}:/project", self.config.repo_root.display()),
                    "-w",
                    "/project",
                    "hdlc/oss-cad-suite:latest",
                    "yosys",
                    "-s",
                    &format!("{}", synth_script.display()),
                ])
                .status()
                .context("docker run yosys")?;
            if !status.success() {
                anyhow::bail!("Yosys synthesis failed (Docker)");
            }
        } else {
            println!("=== Synthesizing with local Yosys ===");
            let status = std::process::Command::new("yosys")
                .arg("-s")
                .arg(&synth_script)
                .current_dir(synth_dir)
                .status()
                .context("yosys")?;
            if !status.success() {
                anyhow::bail!("Yosys synthesis failed");
            }
        }
        println!("Synthesis complete.");
        Ok(())
    }

    fn run_pnr(&self, synth_dir: &Path) -> Result<PathBuf> {
        let repo_root = &self.config.repo_root;
        let _build_dir = self.config.build_dir();
        let device = self.config.board.chipdb_name();

        let nextpnr_bin = self
            .config
            .nextpnr_path
            .clone()
            .unwrap_or_else(|| repo_root.join("build/nextpnr-xilinx/build/nextpnr-xilinx"));

        let chipdb = self
            .config
            .chipdb_path
            .clone()
            .unwrap_or_else(|| repo_root.join(format!("build/fpga/chipdb/{}.bin", device)));

        anyhow::ensure!(
            nextpnr_bin.exists(),
            "nextpnr-xilinx not found at {}",
            nextpnr_bin.display()
        );
        anyhow::ensure!(chipdb.exists(), "chipdb not found at {}", chipdb.display());

        let synth_json = synth_dir.join("synth.json");
        let xdc = self.prepare_xdc(synth_dir)?;
        let fasm_output = synth_dir.join("design.fasm");
        let frames_output = synth_dir.join("design.frames");
        let bit_output = self.config.bitstream_path();

        println!("=== Place & Route (nextpnr-xilinx) ===");
        let status = std::process::Command::new(&nextpnr_bin)
            .arg("--chipdb")
            .arg(&chipdb)
            .arg("--json")
            .arg(&synth_json)
            .arg("--xdc")
            .arg(&xdc)
            .arg("--fasm")
            .arg(&fasm_output)
            .current_dir(synth_dir)
            .status()
            .context("nextpnr-xilinx")?;
        if !status.success() {
            anyhow::bail!("nextpnr P&R failed");
        }

        self.fasm_to_bitstream(synth_dir, &fasm_output, &frames_output, &bit_output, device)?;

        Ok(bit_output)
    }

    fn prepare_xdc(&self, synth_dir: &Path) -> Result<PathBuf> {
        let xdc_out = synth_dir.join("nextpnr.xdc");

        if self.config.minimal {
            let minimal_xdc = crate::xdc::minimal_qmtech_xdc();
            fs::write(&xdc_out, minimal_xdc)?;
            return Ok(xdc_out);
        }

        let xdc_source = self
            .config
            .xdc_path
            .clone()
            .unwrap_or_else(|| self.config.repo_root.join(self.config.board.default_xdc()));

        anyhow::ensure!(
            xdc_source.exists(),
            "XDC not found at {}",
            xdc_source.display()
        );

        let raw = fs::read_to_string(&xdc_source).context("read XDC")?;
        let processed = crate::xdc::preprocess_for_nextpnr(&raw);
        fs::write(&xdc_out, &processed)?;
        Ok(xdc_out)
    }

    fn fasm_to_bitstream(
        &self,
        synth_dir: &Path,
        fasm: &Path,
        frames: &Path,
        bit: &Path,
        device: &str,
    ) -> Result<()> {
        let repo_root = &self.config.repo_root;

        let fasm2frames = self
            .config
            .fasm2frames_path
            .clone()
            .unwrap_or_else(|| repo_root.join("build/fpga/prjxray/utils/fasm2frames.py"));

        let prjxray_db = self.config.prjxray_db_path.clone().unwrap_or_else(|| {
            repo_root.join("build/nextpnr-xilinx/xilinx/external/prjxray-db/artix7")
        });

        let xc7frames2bit = self
            .config
            .frames2bit_path
            .clone()
            .unwrap_or_else(|| repo_root.join("build/fpga/prjxray/build/tools/xc7frames2bit"));

        anyhow::ensure!(fasm2frames.exists(), "fasm2frames not found");
        anyhow::ensure!(prjxray_db.exists(), "prjxray-db not found");
        anyhow::ensure!(xc7frames2bit.exists(), "xc7frames2bit not found");

        self.ensure_prjxray_mapping(&prjxray_db, device)?;

        println!("=== FASM -> Frames ===");
        let status = std::process::Command::new("python3")
            .arg(&fasm2frames)
            .arg("--db-root")
            .arg(&prjxray_db)
            .arg("--part")
            .arg(device)
            .arg(fasm)
            .arg(frames)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .current_dir(synth_dir)
            .status()
            .context("fasm2frames")?;
        if !status.success() {
            anyhow::bail!("fasm2frames failed");
        }

        self.generate_part_yaml(synth_dir, &prjxray_db, device)?;

        let part_yaml = synth_dir.join("part.yaml");
        println!("=== Frames -> Bitstream ===");
        let status = std::process::Command::new(&xc7frames2bit)
            .arg(format!("--part_file={}", part_yaml.display()))
            .arg(format!("--part_name={}", device))
            .arg(format!("--frm_file={}", frames.display()))
            .arg(format!("--output_file={}", bit.display()))
            .status()
            .context("xc7frames2bit")?;
        if !status.success() {
            anyhow::bail!("xc7frames2bit failed");
        }

        let size = fs::metadata(bit)?.len();
        println!("Bitstream: {} ({} bytes)", bit.display(), size);
        println!("=== FPGA E2E build finished ===");
        Ok(())
    }

    fn ensure_prjxray_mapping(&self, prjxray_db: &Path, device: &str) -> Result<()> {
        let mapping_dir = prjxray_db.join("mapping");
        if !mapping_dir.exists() {
            fs::create_dir_all(&mapping_dir)?;
        }
        let parts_yaml = mapping_dir.join("parts.yaml");
        if !parts_yaml.exists() {
            fs::write(
                &parts_yaml,
                format!(
                    "\"{device}\":\n  device: \"xc7a100t\"\n  package: \"csg324\"\n  speedgrade: \"1\"\n"
                ),
            )?;
        }
        let devices_yaml = mapping_dir.join("devices.yaml");
        if !devices_yaml.exists() {
            fs::write(&devices_yaml, "\"xc7a100t\":\n  fabric: \"xc7a100t\"\n")?;
        }
        Ok(())
    }

    fn generate_part_yaml(&self, synth_dir: &Path, prjxray_db: &Path, device: &str) -> Result<()> {
        let part_json_path = prjxray_db.join(device).join("part.json");
        let part_json = fs::read_to_string(&part_json_path).context("read part.json")?;
        let pj: serde_json::Value = serde_json::from_str(&part_json)?;
        let idcode = pj["idcode"].as_u64().unwrap_or(0x3631093);
        let mut yaml = format!(
            "!<xilinx/xc7series/part>\nidcode: 0x{:08x}\nconfiguration_ranges:\n",
            idcode
        );

        if let Some(gcr) = pj["global_clock_regions"].as_object() {
            for (region_name, region) in gcr {
                if let Some(rows) = region["rows"].as_object() {
                    for (row_id, row_data) in rows {
                        if let Some(buses) = row_data["configuration_buses"].as_object() {
                            for (bus_name, bus_data) in buses {
                                if let Some(cols) = bus_data["configuration_columns"].as_object() {
                                    for (col_id, col_data) in cols {
                                        let fc =
                                            col_data["frame_count"].as_u64().unwrap_or(0) as u32;
                                        yaml.push_str(&format!(
                                            "  - !<xilinx/xc7series/configuration_frame_range>\n\
                                             begin: !<xilinx/xc7series/configuration_frame_address>\n\
                                             block_type: {}\n  row_half: {}\n  row: {}\n  column: {}\n  minor: 0\n\
                                             end: !<xilinx/xc7series/configuration_frame_address>\n\
                                             block_type: {}\n  row_half: {}\n  row: {}\n  column: {}\n  minor: {}\n",
                                            bus_name, region_name, row_id, col_id,
                                            bus_name, region_name, row_id, col_id, fc
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        fs::write(synth_dir.join("part.yaml"), &yaml)?;
        Ok(())
    }
}

pub struct BuildResult {
    pub generated_modules: u32,
    pub bitstream: Option<PathBuf>,
    pub smoke: bool,
}
