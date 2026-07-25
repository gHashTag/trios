# RING — BR-OUTPUT (trios-store)

        ## Identity

        | Field | Value |
        |-------|-------|
        | Metal | 🥉 Bronze |
        | Package | trios-store-br-output |
        | Sealed | No |

        ## Purpose

        Assembles the trios-store rings. Dependency flow: BR-OUTPUT → ST-02 → ST-01 → ST-00. Single entry point `open_and_migrate` used by the rest of the backend.

        ## API Surface (pub)

        | Item | Role |
        |------|------|
        | `open_and_migrate(path)` | open DB via ST-01 and apply ST-02 DDL |
| re-exports | ST-00 rows, ST-01 `Store` |

        ## Laws

        - R1 / R5 / R9: Ring isolation, no sibling imports, parent re-exports only
        - I5: README + TASK + AGENTS present in every ring
