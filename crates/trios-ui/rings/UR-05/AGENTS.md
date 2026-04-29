# AGENTS.md — UR-05

## Agent: ALPHA
- Add agent detail panel (expand card on click)
- Add agent filtering by status
- Add agent search

## Agent: BETA
- Test AgentList with empty/ populated agent list
- Test AgentCard status badge colors
- Test agent selection updates ChatAtom

## Rules
- R1: AgentCard uses Badge from UR-02
- R2: No direct API calls — reads from UR-00 AgentsAtom
- R3: Click handler delegates to UR-00 atom mutation
