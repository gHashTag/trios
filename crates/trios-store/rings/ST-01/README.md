# RING — ST-01 (trios-store)

        ## Identity

        | Field | Value |
        |-------|-------|
        | Metal | 🥉 Bronze |
        | Package | trios-store-st01 |
        | Sealed | No |

        ## Purpose

        SQLite repository (sqlx) over ST-00 rows. Opens the SAME `.db` file the TS backend uses (WAL, foreign_keys=ON) so Rust and TS can coexist during migration. All queries are typed against the ST-00 row structs.

        ## API Surface (pub)

        | Item | Role |
        |------|------|
        | `Store` | sqlx SqlitePool wrapper |
| `Store::open / open_memory` | open file / in-memory DB |
| `upsert_agent / get_agent / list_agents / delete_agent` | agent_definitions CRUD |
| `upsert_token / get_token` | oauth_tokens CRUD |
| `upsert_produced_file / files_for_turn` | produced_files CRUD |

        ## Laws

        - R1 / R5 / R9: Ring isolation, no sibling imports, parent re-exports only
        - I5: README + TASK + AGENTS present in every ring
