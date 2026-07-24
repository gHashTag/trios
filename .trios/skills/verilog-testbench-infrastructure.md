# Skill: Verilog Testbench Infrastructure for RTL Benchmarking

## When to Use

When you need to create real Verilog testbenches to validate generated RTL and bridge toward empirical Pass@K benchmarking. This supports the synthesis verification pipeline.

## Steps

1. **Create testbench file** in `data/testbenches/` with naming `tb_<module>.v`
2. **Follow template:**
   ```verilog
   `timescale 1ns/1ps
   module tb_<name>;
       reg clk, rst;
       // ... inputs
       <device_under_test> dut (.clk(clk), ...);
       
       // Clock generation
       initial begin clk = 0; forever #5 clk = ~clk; end
       
       // Test vectors
       initial begin
           #10;
           // ... stimulus
           $display("PASS");
           $finish;
       end
   endmodule
   ```
3. **Add synthesis-bridge functions** in `specs/igla/coder/eval.t27`:
   - `parse_yosys_json(json: string) -> YosysReport`
   - `run_icarus_sim(rtl: string, testbench: string) -> bool`
   - `run_yosys_synth_real(rtl: string) -> YosysReport`
4. **Add tests** for bridge functions (mock where needed).
5. **Run suite** and regenerate seals if needed.

## Limitations

- `run_icarus_sim` and `run_yosys_synth_real` are currently conceptual stubs.
- Real integration requires extern primitives for subprocess execution or C FFI.
- For now, testbenches serve as golden references for manual verification.

## Files

- `data/testbenches/tb_adder.v`
- `data/testbenches/tb_counter.v`
- `data/testbenches/tb_fsm.v`
- `data/testbenches/tb_uart_rx.v`
- `data/testbenches/tb_alu_slice.v`
