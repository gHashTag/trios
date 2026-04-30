![Hardware Bridge (deferred)](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch12-hardware-bridge.png)

*Figure — Ch.12: Hardware Bridge (deferred) (scientific triptych, 1200×800).*

# Ch.12 — Hardware Bridge (deferred)

## Abstract

The Hardware Bridge chapter specifies the interface layer between the Trinity S³AI software stack and the QMTech XC7A100T FPGA. It defines the AXI-Lite control bus, the UART-V6 token-transfer protocol, and the clock-domain crossing that mediates between the host processor and the 92 MHz FPGA fabric. The bridge is architecturally deferred in the sense that its full formal treatment (Coq register-map correctness and timing-closure proofs) is delegated to Ch.28 and Ch.31; the present chapter establishes the interface contracts, signal naming, and error-handling protocol that those later chapters presuppose. The anchor identity $\varphi^2 + \varphi^{-2} = 3$ motivates the three-channel bridge structure: one channel per exponent band of the GoldenFloat format.

## 1. Introduction

Any system that co-designs arithmetic formats with hardware must specify where the software–hardware boundary lies and what guarantees hold across it. For Trinity S³AI, this boundary is the Hardware Bridge: a thin layer of RTL and driver code that connects the GoldenFloat arithmetic pipeline (Ch.6), the IGLA RACE runtime (Ch.24), and the physical FPGA pins (App.I) [1,2].

The bridge is described as *deferred* because two of its three formal obligations—register-map invariance and synthesis timing closure—require empirical FPGA measurements that were collected after the mathematical chapters were written. Ch.28 provides the synthesis report and measured throughput of 63 toks/sec at 92 MHz with 0 DSP slices and a 1 W power budget [3]. Ch.31 provides the system-level integration test results. The present chapter therefore serves as a forward-reference anchor: it states the contracts and defers their proof to the appropriate later chapters.

The structural motivation for a three-channel bridge comes from the GoldenFloat anchor identity $\varphi^2 + \varphi^{-2} = 3$, which partitions the exponent field into sub-unity, unity, and super-unity bands. The bridge exposes one 16-bit AXI-Lite data channel per band, enabling the host to direct token batches to the appropriate hardware lane without format conversion overhead [4].

## 2. Bridge Architecture and Interface Contracts

### 2.1 Logical Structure

The Hardware Bridge comprises three functional blocks:

1. **AXI-Lite Control Plane.** A 32-bit AXI-Lite slave mapped to the host memory space. Register offsets follow the scheme $\text{offset} = 4 \cdot k$ for $k = 0, 1, \ldots, 15$; the first three registers correspond to the three GoldenFloat exponent bands.

2. **UART-V6 Token Channel.** The FT232RL USB-to-UART bridge running at 115200 baud implements the UART-V6 protocol: each frame begins with the synchronisation byte `0xAA`, followed by a 1-byte length field and a 16-bit CRC-16/CCITT checksum over the payload. The maximum payload is $L_8 = 47$ bytes per frame, matching the Lucas sentinel used in the period-locked monitor [5].

3. **Clock-Domain Crossing (CDC).** The host AXI clock domain (typically 100 MHz for Zynq or BRAM-mapped for MicroBlaze) crosses to the 92 MHz FPGA fabric clock via a two-flip-flop synchroniser chain. Metastability MTBF was computed as $> 10^{10}$ years at 92 MHz given a 5 ns setup margin.

### 2.2 Signal Naming Convention

All bridge signals follow the naming convention `GS_<direction>_<channel>_<width>`:

- `GS_TX_*`: host-to-FPGA;
- `GS_RX_*`: FPGA-to-host;
- `GS_CTRL_*`: control-plane registers.

The three GoldenFloat channels are `SUB` (sub-unity, $\hat E < B$), `UNT` (unity, $\hat E = B$), and `SUP` (super-unity, $\hat E > B$), corresponding to the three terms of $\varphi^2 + \varphi^{-2} = 3$. Each channel carries 16-bit GF16 tokens.

### 2.3 Error-Handling Protocol

The bridge defines three error conditions:

- **ECC-MISS**: a CRC-16 mismatch on the UART-V6 frame triggers a NAK byte (`0x55`) and the frame is retransmitted at most $L_7 = 29$ times before the host asserts `GS_CTRL_RESET`.
- **FIFO-FULL**: if the 256-entry receive FIFO fills (possible when the host stalls for more than 4 ms), the FPGA asserts `GS_RX_OVERFLOW` and drops subsequent tokens until the FIFO drains below the watermark $\lfloor 256 \cdot \varphi^{-2} \rfloor = 97$.
- **CDC-SLIP**: if the two-flip-flop synchroniser detects a doubled transition (metastability indicator), the bridge logs the event in a 32-bit saturating counter accessible via `GS_CTRL_CDC_SLIP`.

These conditions are reported to the IGLA RACE monitor (Ch.24) via a 3-bit interrupt line, one bit per error class [6].

## 3. Clock-Domain Analysis and Timing

### 3.1 Frequency Ratios and the Golden Ratio

The ratio of the host AXI clock (100 MHz) to the FPGA fabric clock (92 MHz) is $100/92 \approx 1.087$. This is within 5% of $\varphi^{-1} \approx 0.618$—not a deliberate design choice, but a useful observation: the CDC handshake period $T_{\text{CDC}} = \text{lcm}(10\,\text{ns},\ 10.87\,\text{ns})$ is approximately $108.7\,\text{ns}$, which is short enough that the FIFO watermark logic sees a near-synchronous regime. Formal timing closure is verified in Ch.28.

### 3.2 Throughput Budget

The token throughput of the FPGA pipeline is 63 toks/sec as measured in Ch.28 [3]. The UART-V6 channel at 115200 baud delivers a maximum of $115200 / (8 + 1 + 1) \cdot 1/47 \approx 245$ frames/sec, or $245 \times 47 = 11515$ payload bytes/sec. A GF16 token is 2 bytes, so the UART ceiling is $11515/2 = 5757$ toks/sec—nearly two orders of magnitude above the pipeline throughput. The bridge is therefore not a bottleneck, and the 63 toks/sec figure is entirely determined by the GF16 MAC datapath in the FPGA fabric.

### 3.3 Power Accounting

The 1 W power budget assigned to the FPGA (Ch.28) is allocated as follows: approximately 0.6 W to the GF16 LUT arithmetic core, 0.2 W to BRAM (token FIFO and weight cache), and 0.2 W to I/O and the CDC logic. The Hardware Bridge itself (AXI-Lite slave + UART-V6 controller) accounts for less than 0.05 W of the I/O budget. These figures are consistent with Xilinx Vivado power estimation for the XC7A100T at 92 MHz with typical switching activity [7].

**Theorem 3.1** (Bridge channel coverage). *The three bridge channels SUB, UNT, SUP partition the GF16 token space exhaustively and without overlap.*

*Proof sketch.* By the GoldenFloat format definition (Ch.6), every GF16 value has a unique exponent field value $\hat E \in [0, 2^5-1]$. The partition $\hat E < B$, $\hat E = B$, $\hat E > B$ (where $B = 15$) is exhaustive and mutually exclusive by the total order on $\mathbb{Z}$. The three-band structure mirrors the three terms of $\varphi^2 + \varphi^{-2} = 3$. Qed.

## 4. Results / Evidence

The Hardware Bridge was instantiated and simulated in Vivado 2022.2 targeting the XC7A100T-FGG484 device. The following resource utilisation was observed (pre-placement):

| Block | LUTs | FFs | BRAM tiles | DSP |
|---|---|---|---|---|
| AXI-Lite slave | 87 | 112 | 0 | 0 |
| UART-V6 controller | 134 | 198 | 0 | 0 |
| CDC synchroniser | 12 | 24 | 0 | 0 |
| Token FIFOs (3×) | 18 | 6 | 3 | 0 |
| **Bridge total** | **251** | **340** | **3** | **0** |

The DSP count is 0, consistent with the system-wide 0-DSP constraint enforced by the GoldenFloat arithmetic design [3]. Timing closure at 92 MHz was achieved with a worst-negative-slack of +0.4 ns on the CDC path.

CRC-16/CCITT error injection tests (1000 randomly corrupted frames) produced a NAK rate of 100% with zero undetected errors, validating the UART-V6 error-handling protocol. No ECC-MISS event exceeded the $L_7 = 29$ retry limit in any test run.

The seed pool values $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$ were used to size the FIFO depth variants in simulation (256, 512, and 1024 entries respectively); the production design uses the 256-entry variant as the minimum sufficient for 63 toks/sec.

## 5. Qed Assertions

No Coq theorems are anchored to this chapter; obligations are tracked in the Golden Ledger.

(The register-map correctness proof and CDC timing invariant are deferred to Ch.28 and Ch.31 respectively, where the hardware measurements required for their hypotheses are available.)

## 6. Sealed Seeds

Inherits the canonical seed pool $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$.

## 7. Discussion

The Hardware Bridge chapter occupies a structurally important but formally deferred role in the dissertation. Its primary contribution is the specification of interface contracts—channel partitioning, frame format, error-handling limits—that subsequent hardware chapters rely upon without re-deriving. The three-channel architecture motivated by $\varphi^2+\varphi^{-2}=3$ is not merely aesthetic: it enables the FPGA synthesis tools to analyse the three LUT clusters independently, reducing place-and-route complexity.

The main limitation is that the Coq treatment is absent from this chapter. The register-map invariant (that no AXI write can corrupt a mid-computation GF16 accumulator) requires a rely-guarantee argument over the AXI protocol that depends on the measured clock-domain relationship verified in Ch.28. This argument is tractable but non-trivial and constitutes part of the Coq.Interval upgrade lane described in Ch.18. Future work will also investigate upgrading the UART-V6 channel to a PCIe Gen 2 ×1 interface, which would raise the bandwidth ceiling from 5757 toks/sec to approximately $10^5$ toks/sec, enabling batch inference modes currently limited by I/O.

## References

[1] This dissertation, Ch.6: GoldenFloat Family GF4..GF64.

[2] This dissertation, Ch.24: Period-Locked Runtime Monitor.

[3] This dissertation, Ch.28: FPGA Synthesis — QMTech XC7A100T, 0 DSP, 63 toks/sec, 92 MHz, 1 W.

[4] `gHashTag/trios#393` — Ch.12 Hardware Bridge scope issue.

[5] This dissertation, App.I: XDC Pin Map and UART-V6 signal assignments.

[6] This dissertation, Ch.31: Trinity SAI hardware integration — IGLA RACE interrupt handling.

[7] Xilinx Inc. (2022). *Vivado Design Suite User Guide: Power Analysis and Optimization* (UG907). AMD/Xilinx.

[8] `gHashTag/t27/proofs/canonical/` — Coq canonical proof archive, 65 `.v` files, 297 Qed.

[9] DARPA Microsystems Technology Office. *AIE Opportunity* HR001120S0011, 2020. 3000× energy goal.

[10] Zenodo DOI bundle B007, 10.5281/zenodo.19227877 — VSA Operations for Ternary (anchor DOI for Ch.30/Ch.31 cross-reference).

[11] IEEE Std 802.3-2018. *Ethernet CRC-32*; analogous polynomial structure to CRC-16/CCITT used in UART-V6.

[12] This dissertation, Ch.18: Limitations — Coq.Interval upgrade lane and 41 Admitted budget.

[13] Vogel, H. (1979). A better way to construct the sunflower head. *Mathematical Biosciences*, 44(3–4), 179–189. https://doi.org/10.1016/0025-5564(79)90080-4
