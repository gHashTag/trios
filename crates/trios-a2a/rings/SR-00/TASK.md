# TASK — SR-00 (trios-a2a)

## Status: DONE ✅

## Completed

- [x] `AgentId` newtype with Display + Hash
- [x] `AgentCard` builder pattern (`with_capability`, `has_capability`, `is_available`)
- [x] `Capability` enum (Codegen, FileSystem, Git, Shell, LLM, Orchestrator, Custom)
- [x] `AgentStatus` enum with serde snake_case
- [x] Tests: display, builder, serialization, availability

## Open

- [ ] Add `AgentCard::with_description()`
- [ ] Add `AgentCard::metadata: HashMap<String, String>` for extensibility
- [ ] Add `AgentId::validate()` — enforce naming convention (lowercase-alphanumeric)
- [ ] Add `Capability::from_str()` for MCP tool deserialization

## Next ring: SR-01
