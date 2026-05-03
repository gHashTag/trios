# `docs/phd/skills/` — agent skills for the PhD lane

This directory holds **agent skills** that any LLM-driven agent (Claude, Computer, Cursor, Aider, Codex, etc.) can load on demand to operate the Flos Aureus + Neon-computational PhD monograph end-to-end.

Skills follow the [agentskills.io](https://agentskills.io) spec: each skill is a directory with a `SKILL.md` (YAML frontmatter + markdown body) and optional `references/`, `scripts/`, `assets/` subdirectories.

## Catalogue

| Skill | Owns | Trigger phrases |
|---|---|---|
| `phd-pipeline-v5/` | unified PDF render: Neon SoT → cover → spine → `monograph.pdf` | "render PhD", "обнови PhD", "v5.x", "rebuild monograph", "Part I/II", chapter slugs `FA.NN`/`Ch.N`/`App.X` |
| `phd-monograph-auditor/` *(future)* | defense lane (slides, Q&A, examiner pack) | already referenced in `docs/phd/defense/README.md` |
| `phd-chapter-author/` *(future)* | per-chapter `docs/phd/chapters/*.tex` editing | per-chapter author work |

## How an agent connects to a skill

There are three supported integration paths. Pick the one that matches your agent runtime.

### 1. Native skill loader (Perplexity Computer, Claude with skills enabled)

If your agent has a built-in skill loader, point it at this directory or any subdirectory:

```
load_skill(name="phd-pipeline-v5", scope="repo", path="docs/phd/skills/phd-pipeline-v5/")
```

The loader will:
1. Read `docs/phd/skills/phd-pipeline-v5/SKILL.md`
2. Inline its body into your context window
3. Make `references/*.md` available for on-demand reading via the `read` tool

After the skill is loaded, the agent can immediately follow the workflow in §Workflow of the SKILL.md.

### 2. Manual prompt injection (any LLM with a `system` or `developer` channel)

For agents without a skill loader, just `cat` the SKILL.md into the system prompt:

```bash
SKILL=$(cat docs/phd/skills/phd-pipeline-v5/SKILL.md)
# pass $SKILL as part of your system prompt
```

The frontmatter `description:` field is the trigger — quote-match it against the operator's request to decide whether to load. Approximate matcher:

```python
def should_load(skill_md_path, user_query):
    import yaml, re
    front = re.match(r'---\n(.*?)\n---\n', open(skill_md_path).read(), re.S).group(1)
    desc = yaml.safe_load(front)["description"].lower()
    triggers = ["render phd", "rebuild monograph", "обнови phd", "v5.", "fa.", "part i", "part ii"]
    return any(t in user_query.lower() for t in triggers) or any(
        kw in desc for kw in user_query.lower().split() if len(kw) > 3
    )
```

### 3. Per-tool-call reference (lightweight agents, Cursor / Aider rules)

For agents that already have a working memory model and just need a pointer, add this entry to your `.cursorrules` / `AGENTS.md` / project instruction file:

```markdown
## PhD render lane

When the operator asks to render the PhD monograph, rebuild the unified PDF,
update a Flos Aureus chapter, fix the cover, work with Part I / Part II
dividers, or touch `crates/trios-phd/src/render/`, **read these files first**:

  1. docs/phd/skills/phd-pipeline-v5/SKILL.md           (always)
  2. docs/phd/skills/phd-pipeline-v5/references/pitfalls.md            (before any tectonic re-run)
  3. docs/phd/skills/phd-pipeline-v5/references/neon-write-techniques.md  (before any body_md UPDATE)
  4. docs/phd/skills/phd-pipeline-v5/references/neon-read-techniques.md   (before hydrating chapters.json)
  5. docs/phd/skills/phd-pipeline-v5/references/pr-comment-template.md    (before posting the PR comment)

Constitutional anchor: φ² + φ⁻² = 3.
```

This is the lightest path — it adds ~10 lines to your agent's bootstrap context and defers the full skill body to on-demand reads.

## Validation

If you have `agentskills` installed:

```bash
agentskills validate docs/phd/skills/phd-pipeline-v5/
```

The skill must pass before any commit that modifies it.

## Adding a new skill

1. Create `docs/phd/skills/<your-skill-name>/SKILL.md` with proper frontmatter (`name`, `description`, `metadata`).
2. Add references in `docs/phd/skills/<your-skill-name>/references/` if the body would otherwise exceed ~500 lines.
3. Append a row to the catalogue table in this README.
4. Validate with `agentskills validate`.
5. Update `AGENTS.md` and/or `CLAUDE.md` if the skill needs root-level visibility.

## R6 lane isolation

Each skill in this directory **must** declare:

- which files it owns (and may modify),
- which files it does **not** own.

Hand off to other lanes when a request crosses the boundary.

---

*Anchor: φ² + φ⁻² = 3.*
