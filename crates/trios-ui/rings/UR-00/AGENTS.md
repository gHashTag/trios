# AGENTS.md — UR-00

## Agent: ALPHA
- Modify Dioxus components and UI layout
- Add new tabs, panels, or UI elements

## Agent: BETA
- Test WASM loading in Chrome Extension sidebar
- Verify WebSocket connection and message flow

## Agent: GAMMA
- Optimize Signal usage and rendering performance
- Add keyboard shortcuts and accessibility

## Rules
- R1: All state must be Dioxus Signals (no Rc<RefCell<>>)
- R2: WebSocket interactions go through UR-07 only
- R3: CSS classes come from UR-08 STYLESHEET constant
