# RING — SR-00 (trios-a2a)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-a2a-sr00 |
| Sealed | No |

## Purpose

Agent identity ring. Defines the fundamental types that every A2A participant needs:
who an agent is, what it can do, and whether it is available.

## Why SR-00 is the bottom of the graph

Every other ring (SR-01, SR-02, BR-OUTPUT) depends on `AgentId`.
If SR-00 had any dependencies itself, it would create circular risk.
Keeping it dependency-free guarantees that the entire type graph compiles
in a single pass and can be embedded in any future crate.

## API Surface (pub)

| Type | Role |
|------|------|
| `AgentId(String)` | Unique agent identifier — newtype for safety |
| `AgentCard` | Identity + capabilities + status |
| `Capability` | Enum: Codegen, FileSystem, Git, Shell, LLM, Orchestrator, Custom |
| `AgentStatus` | Enum: Idle, Busy, Offline, Error |

## Dependencies

- `serde` (derive only)

## Laws

- R1: No imports from SR-01, SR-02, BR-OUTPUT
- L6: Pure Rust only
- No I/O, no subprocess, no async
