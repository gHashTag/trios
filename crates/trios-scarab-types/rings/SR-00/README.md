# SR-00 — scarab-types

The 16-element `Term` alphabet that every higher-tier scarab in
SR-01..04 composes from. Ring-pattern foundation for parallel agent
execution via `trinity-bootstrap --codename` (PR #469).

## Term taxonomy

| Variant | Slug |
|---|---|
| `Term::OType`     | `O-Type` (origin / monad) |
| `Term::OneType`   | `One-Type` |
| `Term::TwoType`   | `Two-Type` |
| `Term::ThreeType` | `Three-Type` |
| `Term::FourType`  | `Four-Type` |
| `Term::FiveType`  | `Five-Type` |
| `Term::SixType`   | `Six-Type` |
| `Term::SevenType` | `Seven-Type` |
| `Term::EightType` | `Eight-Type` |
| `Term::NineType`  | `Nine-Type` |
| `Term::TenType`   | `Ten-Type` |
| `Term::ElevenType`  | `Eleven-Type` |
| `Term::TwelveType`  | `Twelve-Type` |
| `Term::TrinityType` | `Trinity-Type` |
| `Term::PhiType`     | `Phi-Type` |
| `Term::LucasType`   | `Lucas-Type` |

Each variant implements `Display` and `as_markdown` and round-trips
through serde.

## Constitutional compliance

- R-RING-DEP-002: `serde` only.
- I5: README.md, AGENTS.md, RING.md, Cargo.toml, src/lib.rs.
- L13: single-ring scope.
