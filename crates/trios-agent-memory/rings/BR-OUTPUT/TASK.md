# TASK — BR-OUTPUT AgentMemory

**Issue:** #461  
**Priority:** P2-MEDIUM  
**Status:** in-progress  

## Acceptance criteria

- [ ] `rings/BR-OUTPUT/` with I5 trinity
- [ ] Deps: SR-MEM-00 + SR-MEM-01 + SR-MEM-05
- [ ] Public trait `AgentMemory { recall, remember, reflect, forget }`
- [ ] Default impl `KgAgentMemory` wires SR-MEM-01 + SR-MEM-05
- [ ] Stubs for HyDE expansion / supersede / GDPR: marked TODO with linked issues
- [ ] Integration test: round-trip remember → recall produces SHA-256-equal triples
- [ ] PR closes #461, `Agent: LEAD` trailer

## Notes

Blocked by SR-MEM-05 (#455) for full wiring.
Skeleton + trait definition can proceed immediately.

`phi² + phi⁻² = 3`
