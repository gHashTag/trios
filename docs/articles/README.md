# docs/articles — canonical article sources

Each subdirectory here is one article rendered by the `tri article`
service. `tri article list` enumerates these by their `[article].slug`
in `<slug>/article.toml`.

| Slug | Description |
|---|---|
| `pellis-trinity-full` | Pellis-Trinity PhD-Style Atlas, v21 style-safe edition |

## Canonical commands

```bash
tri article list
tri article presets
tri article build <slug> --pdf
tri article build <slug> --html
tri article qa    <slug>
```

If `tri` is not installed:

```bash
cargo run -p tri-cli -- article list
cargo run -p tri-cli -- article presets
cargo run -p tri-cli -- article build <slug> --pdf
cargo run -p tri-cli -- article build <slug> --html
cargo run -p tri-cli -- article qa    <slug>
```

## Build blocker

At the time of this commit the `article` subcommand on `tri` / `tri-cli`
is not yet implemented in this repository — see
[`BLOCKER.md`](./BLOCKER.md). The article sources are nevertheless laid
out in the canonical structure so that the renderer, when wired up, has
a stable upstream source of truth.
