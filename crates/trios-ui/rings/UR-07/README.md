# UR-07 — WebSocket API Client

## Purpose
WebSocket client that connects to trios-server on `ws://localhost:9005/ws`.
Provides `ApiClient` with methods for chat, agent listing, and tool listing.

## Dependencies
- None (standalone ring)

## Ring Rules
- R1: Only ring that touches raw WebSocket API
- R2: All server communication goes through this ring
- R3: Callbacks are `FnMut` to allow Dioxus Signal mutation
