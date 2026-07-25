# RING — ST-00 (trios-store)

        ## Identity

        | Field | Value |
        |-------|-------|
        | Metal | 🥉 Bronze |
        | Package | trios-store-st00 |
        | Sealed | No |

        ## Purpose

        Store schema types: pure data + serde rows mirroring the drizzle SQLite schema from `browseros-agent/apps/server/src/lib/db/schema/*` 1:1, so the Rust backend reads/writes the SAME database file during migration. Three tables: `agent_definitions`, `oauth_tokens`, `produced_files`. No I/O, no async, no SQL — bottom of the trios-store ring graph.

        ## API Surface (pub)

        | Item | Role |
        |------|------|
        | `AgentDefinitionRow` | 1:1 row of `agent_definitions` |
| `OAuthTokenRow` | 1:1 row of `oauth_tokens` |
| `ProducedFileRow` | 1:1 row of `produced_files` |

        ## Laws

        - R1 / R5 / R9: Ring isolation, no sibling imports, parent re-exports only
        - I5: README + TASK + AGENTS present in every ring
