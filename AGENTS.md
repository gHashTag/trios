# AGENTS.md — trios workspace root

## Agent Protocol

All agents working in this workspace follow the AIP (Agent Interaction Protocol).

## Agent Codenames

| Codename | Domain |
|----------|--------|
| ALPHA | trios-ipc |
| BETA | trios-server |
| GAMMA | trios-ui |
| DELTA | trios-doctor |
| EPSILON | trios-ext |
| ZETA | trios-a2a |
| LEAD | orchestration |

## Invariant I-SCOPE: Agent Scope Isolation

**Агент работает ТОЛЬКО внутри своего крейта.**

- ❌ НИКОГДА не трогать файлы за пределами своего `crates/<X>/`
- ❌ НИКОГДА не удалять, переименовывать или перемещать кольца других крейтов
- ❌ НИКОГДА не менять корневой `Cargo.toml` без явного задания на это
- ✅ Если нужно добавить зависимость от другого крейта — только `path =` в своём `Cargo.toml`

**Исключение:** корневой `Cargo.toml` может быть изменён только по явному заданию (например, "добавить кольца в workspace members").

## Commit Convention

Every commit MUST include an `Agent: <CODENAME>` trailer.

```
feat(trios-doctor): implement DR-01..03

Agent: DELTA
```
