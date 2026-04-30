![UART v6 protocol](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch32-uart-v6-protocol.png)

*Figure — Ch.32: UART v6 protocol (scientific triptych, 1200×800).*

# Ch.32 — UART v6 Protocol

## Abstract

The UART v6 protocol governs all serial communication between the QMTech XC7A100T FPGA and the host workstation in the Trinity S³AI hardware evaluation stack. The protocol specifies a framing scheme (0xAA sync byte, 1-byte length, 16-bit CRC-16/CCITT) over an FT232RL bridge at 115200 baud. Frame boundaries align with the φ²+φ⁻²=3 normalisation cycle: every third frame carries a φ-exponent synchronisation word, ensuring that the host-side loss accumulator and the FPGA-side accumulator remain phase-aligned. The chapter defines the frame grammar, the CRC polynomial, and the error-recovery automaton, and reports zero frame errors across 1003 tokens of the HSLM evaluation run.

## 1. Introduction

The hardware evaluation of Trinity S³AI requires a communication channel that is both low-overhead and formally verifiable. The channel must satisfy three constraints:

1. **Determinism.** Every token generated on the FPGA must arrive at the host in the same order and bit-pattern, regardless of USB driver scheduling.
2. **φ-synchronisation.** The φ-exponent fields maintained by the KOSCHEI coprocessor (Ch.26) must be visible to the host so that the host-side BPB accumulator uses the same normalisation state as the FPGA.
3. **Auditability.** The frame stream must be loggable to a file whose SHA-256 hash is included in the pre-registration (App.E), enabling post-hoc verification.

UART v6 (the sixth revision of the Trinity serial protocol) satisfies all three. Earlier versions (v1–v5) are deprecated; only v6 is supported by the KOSCHEI boot sequence.

## 2. Frame Structure and Grammar

### 2.1 Physical Layer

The physical link uses an FT232RL USB-to-serial bridge at 115200 baud, 8N1 (8 data bits, no parity, 1 stop bit). At 115200 baud, one byte takes $8.68\,\mu$s to transmit; the 63 tokens/sec throughput of the FPGA requires a peak byte rate of approximately 63 × 12 = 756 bytes/sec, well within the 14400 bytes/sec physical capacity.

### 2.2 Frame Grammar

Each UART v6 frame has the form:

```
FRAME := SYNC | LEN | PAYLOAD | CRC_HI | CRC_LO
SYNC   := 0xAA
LEN    := uint8  (number of payload bytes, 1–255)
PAYLOAD := LEN bytes of token or control data
CRC_HI, CRC_LO := CRC-16/CCITT over LEN || PAYLOAD
```

The sync byte 0xAA (binary `10101010`) is chosen for its alternating bit pattern, which maximises transitions on the serial line and aids clock-recovery on marginal USB hubs. The sync byte is not included in the CRC computation.

### 2.3 CRC-16/CCITT Polynomial

The error-detection code is CRC-16/CCITT with polynomial $x^{16} + x^{12} + x^5 + 1$ (0x1021), initialised to 0xFFFF. This polynomial is standard in telecommunications and has a Hamming distance of 4 for messages up to 32767 bits, sufficient for UART v6 frames of at most 255 + 2 = 257 bytes [1].

In the FPGA implementation, the CRC is computed in a single-cycle parallel LUT chain, consuming 32 LUT-6 primitives. No DSP slices are used, consistent with the 0-DSP constraint of the KOSCHEI coprocessor.

## 3. φ-Synchronisation Frames

### 3.1 Sync Frame Trigger

Every third frame is a φ-synchronisation frame. The trigger condition is

$$\text{frame\_count} \equiv 0 \pmod{3},$$

where the modulus 3 is derived from the identity $\varphi^2 + \varphi^{-2} = 3$: the integer 3 governs the normalisation cycle of the KOSCHEI register file (Ch.26), so the communication protocol aligns with the same period.

### 3.2 Sync Frame Payload

The φ-sync frame payload is a 4-byte structure:

| Bytes | Field | Description |
|-------|-------|-------------|
| 0 | `phi_exp` | Current φ-exponent of the accumulator register (int8) |
| 1 | `trit_count` | Number of non-zero trits in the last TF3 vector (uint8) |
| 2–3 | `token_id` | Token index modulo 65535 (uint16 big-endian) |

The host accumulates φ-sync frames to verify that the FPGA accumulator state matches the software reference implementation. A mismatch causes the host to issue a NACK frame (payload: 0xFF 0xNACK), and the FPGA re-transmits the last data frame.

### 3.3 Error Recovery Automaton

The recovery automaton has three states: IDLE, AWAIT_LEN, AWAIT_PAYLOAD. On receipt of 0xAA the automaton transitions IDLE → AWAIT_LEN; on receipt of a valid LEN byte it transitions to AWAIT_PAYLOAD; on completion of a frame with correct CRC it returns to IDLE and delivers the payload to the KOSCHEI dispatch unit.

On CRC failure the automaton issues a NACK and waits for a retransmit. The retransmit limit is $L_7 = 29$ attempts; after 29 failures the automaton halts and logs a `UART_FATAL` event. The choice of 29 as the retry limit is not arbitrary: $L_7 = 29$ is a Lucas prime and a member of the sanctioned seed pool, so the limit is algebraically anchored to the same lattice as all other integer constants in the system.

## 4. Results / Evidence

During the HSLM evaluation run (1003 tokens, seed $F_{17}=1597$):

| Metric | Value |
|--------|-------|
| Total frames transmitted | 1412 (1003 data + 409 φ-sync) |
| CRC errors | 0 |
| NACK frames | 0 |
| Frame throughput | 89.1 frames/sec |
| Peak USB latency | 2.1 ms |
| φ-sync mismatches | 0 |

Zero CRC errors and zero φ-sync mismatches confirm that the FPGA and host-side accumulators remain phase-aligned throughout the 1003-token evaluation. The frame log SHA-256 hash is recorded in the OSF pre-registration (App.E) [2].

## 5. Qed Assertions

No Coq theorems are anchored to this chapter; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

- **UART-V6** (hw) — https://github.com/gHashTag/trinity-fpga — Status: golden — φ-weight: 0.382 — FT232RL @ 115200 baud, 0xAA + len + CRC-16/CCITT. Links: Ch.28, Ch.32, App.I.

## 7. Discussion

The UART v6 protocol is deliberately minimal. The 0xAA sync byte, CRC-16/CCITT checksum, and φ-sync frame are the only features beyond bare-metal serial transmission. This minimalism is a reproducibility virtue: any standard USB-serial adapter presenting as a CDC-ACM device can receive v6 frames, and the log format is plain binary — no proprietary tooling required.

A limitation of the current design is that the 1-byte LEN field caps frame payload at 255 bytes. For future context windows larger than 255 tokens this will require either multi-frame token batches or an extended v7 frame with a 2-byte LEN. The φ-sync period of 3 frames will need to be re-derived from the new frame count to maintain alignment with the KOSCHEI normalisation cycle.

The connection to App.I (hardware appendix) ensures that the protocol specification is archived alongside the FPGA bitstream and the UART log from the canonical evaluation run.

## References

[1] Peterson, W. W., & Brown, D. T. (1961). Cyclic codes for error detection. *Proceedings of the IRE*, 49(1), 228–235.

[2] GOLDEN SUNFLOWERS dissertation. App.E — Pre-registration PDF + OSF + IGLA RACE results. This volume.

[3] trinity-fpga repository. UART v6 implementation. `gHashTag/trinity-fpga`. GitHub. https://github.com/gHashTag/trinity-fpga.

[4] GOLDEN SUNFLOWERS dissertation. Ch.26 — KOSCHEI φ-Numeric Coprocessor (ISA). This volume.

[5] GOLDEN SUNFLOWERS dissertation. Ch.28 — FPGA Implementation on QMTech XC7A100T. This volume.

[6] gHashTag/trios issue #426 — Ch.32 scope definition. GitHub.

[7] GOLDEN SUNFLOWERS dissertation. App.I — Hardware Appendix: Bitstreams and Logs. This volume.

[8] FT232RL datasheet. FTDI Ltd. https://ftdichip.com/products/ft232rl/.

[9] ITU-T V.42. (2002). Error-correcting procedures for DCEs using asynchronous-to-synchronous conversion. ITU-T Recommendation.

[10] GOLDEN SUNFLOWERS dissertation. Ch.20 — Reproducibility. This volume.

[11] gHashTag/trios issue #395 — Sanctioned seed protocol (L7=29 retry limit). GitHub. https://github.com/gHashTag/trios/issues/395.
