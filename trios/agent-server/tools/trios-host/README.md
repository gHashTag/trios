# trios-host

`trios-host` lends a computer to the Queen. It runs shards of the t27 corpus
(`t27c test-report` on one spec at a pinned commit, the work the t27b and t27c
labs do) and earns TRI credit, off-chain, for every result another host agrees
with. Self-hosting, slice 1: gHashTag/trios#1756.

Every decision it makes, and that the Queen makes about it, is a card built
from gHashTag/t27 `specs/hosting/*.t27` (`host`, `placement`, `proof`,
`credit`); the toolchain pins are data in `toolchain.t27`.

## One command

There is no installer script. On a Mac with Apple silicon:

```
curl -fLo trios-host <release URL of trios-host-darwin-arm64> && chmod +x trios-host
./trios-host join --queen <queen URL> --t27c <path to a t27c binary>
```

On Linux x86-64, use the `trios-host-linux-x64` asset. `join` makes
`~/.trios-host/` (mode 0700) and the host key `~/.trios-host/key` (mode 0600;
only the public key and signatures ever leave the machine). It downloads zig
0.16.0 from ziglang.org and keeps it only when its SHA-256 and size are the
pinned ones. Then it registers, beats every 5 s, and runs one shard at a time.

`--t27c` is required today: no t27c release carries a binary yet, so
`toolchain.t27` pins none, and an unpinned tool is never downloaded. The
digest of the t27c you name goes into every receipt as `model_hash`.

## Commands

| command | what it does |
|---|---|
| `trios-host key` | make or read the key; print the host id (16 hex) and the public key |
| `trios-host join --queen <url> [--tier public\|trusted\|owner] [--slots <n>] [--t27c <path>] [--zig <path>] [--t27-repo <dir>] [--once]` | run as a host. `--tier` is a claim: the Queen grants at most what the owner's allowlist names for this key, and public otherwise. `--t27-repo` reads a shard's files from a local t27 clone instead of raw.githubusercontent.com. `--once` runs one shard and exits |
| `trios-host job --queen <url> --t27-repo <dir> --commit <sha> --spec <path> [--secret] [--personal]` | owner key only: ask the Queen to run one spec at one commit; every file of its `use` closure is pinned by SHA-256 |
| `trios-host status --queen <url>` | this host's row of the public ledger |

`TRIOS_HOST_HOME` moves `~/.trios-host`.

## What a shard can see

- **Files:** only the files the job pins. Each is checked against its
  SHA-256 before it is written to disk.
- **Environment:** built from nothing: `PATH` to zig and the system tools, a
  scratch `HOME` and `TMPDIR`, zig's caches, and `T27C_TEST_REPORT_EXIT_ZERO`.
  No variable of your shell reaches it.

It is not an OS sandbox. t27c and zig run as your user, and a spec's tests run
as code on your machine. A shard is only ever a spec from the t27 repository at
the job's commit.

## What a host earns

Credit is 1 mTRI per job that two hosts agree on, recorded in the Queen's
public ledger (`GET /hosting/ledger`). It is off-chain:
- it is not a token;
- it is not transferable;
- it claims no value;
- settlement is switched off in `credit.t27`, and any value flow needs a
  device-bound identity that no host has.

A host whose receipt is tampered, or whose result dissents from an agreed
one, gets a strike. Three strikes, and it is placed nothing more.

## For the owner

- **Server:** the Queen serves `/hosting/*` only with `TRIOS_HOSTING=on` (503
  otherwise).
- **Allowlist:** `TRIOS_HOSTING_ALLOWLIST=<host id>=owner,<host id>=trusted`
  names your own machines and the people you trust. Every other key is public,
  and a public host only ever receives shards that declare no secret and no
  personal data.
- **Build:**

  ```
  bun build --compile --target=bun-darwin-arm64 tools/trios-host/trios-host.ts --outfile trios-host-darwin-arm64
  bun build --compile --target=bun-linux-x64    tools/trios-host/trios-host.ts --outfile trios-host-linux-x64
  ```

  `bun build --compile` output is not byte-reproducible: two builds give two
  digests. A release therefore pins the digest of the file it uploads.
- **Demo:** the demo runs as tests, all local:
  `cd apps/server && bun test tests/api/hosting.test.ts`.
