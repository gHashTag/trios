# The vault

Tracks gHashTag/trios#1759. Every decision is in gHashTag/t27
`specs/vault/policy.t27`, `audit.t27` and `merge.t27`, vendored under
`specs/vault/` with their wasm cards (`specs/PIN`); the constants are
`t27c gen-ts` output in `apps/server/src/api/services/queen-vault-*-card.gen.ts`.
The TypeScript is glue: the store, HTTP, the `age` process and the CLI.

An agent or a service asks for a scope by name with an Ed25519-signed request.
The vault answers with ciphertext sealed to a one-lease age key that the
requesting process made in memory. `trios-vault run` opens it and puts the
values into the environment of the child it starts, and nowhere else. No
response, log line, event, receipt or report carries a value.

Every row is an age file sealed to the vault's identity and to the owner's
recovery recipient. The `queen_vault_store` table, or an export, opens on any
machine with the owner's recovery key and the stock `age` tool.

## The variables (policy.t27 section 13)

| variable | what | secret? |
|---|---|---|
| `TRIOS_VAULT` | `on` starts the vault; anything else, or unset, is off | no |
| `TRIOS_VAULT_IDENTITY` | the vault's age identity, the `AGE-SECRET-KEY-1...` line | **yes, the only one** |
| `TRIOS_VAULT_RECOVERY` | the owner's recovery recipient, `age1...` | no |
| `TRIOS_VAULT_OWNER_KEYS` | the owner's Ed25519 public keys, hex, comma-separated | no |

**Off means off.** With `TRIOS_VAULT` unset, the server reads none of these
variables and touches no table, and every `/vault/*` request answers 503.

**On with a bad setup.** If a variable is missing or malformed, if the store
was written under another identity or recovery recipient, if there is no
database, or if `age` is not installed, the vault does not start. The log
gets one line, `vault refused to start: <word> (check <VARIABLE>)`, which
never contains a value. The routes stay 503.

The server reads `TRIOS_VAULT_IDENTITY` once at boot and then deletes it from
its own environment, so no process started later inherits it.

## Production runbook (the owner, on the owner's Mac)

Install the tools once. Infisical's formula lives in its own tap:

```sh
brew install age bun infisical/get-cli/infisical
alias trios-vault="bun $HOME/src/trios/trios/agent-server/apps/server/src/vault-cli.ts"
```

1. **Generate the vault's identity locally.**

   ```sh
   age-keygen -o vault-identity.txt
   ```

   This prints the vault's public key. Save the file in your password manager
   and delete the local copy after step 2. Generate your recovery key the same
   way (`age-keygen -o recovery.txt`), keep it only in the password manager,
   and note its `age1...` public key.

2. **Create your owner signing key.**

   ```sh
   trios-vault key new --out ~/.config/trios-vault/owner.key
   ```

   This prints `public key: <64 hex>`. The private half stays in that file, mode 0600.

3. **Set the variables on the Queen's Railway service.**
   - `TRIOS_VAULT_IDENTITY`: paste the `AGE-SECRET-KEY-1...` line in the
     dashboard's variable editor, not on a command line, so it never lands in
     a shell history.
   - `TRIOS_VAULT_RECOVERY`: the recovery `age1...` key.
   - `TRIOS_VAULT_OWNER_KEYS`: the 64-hex public key from step 2.

4. **Set `TRIOS_VAULT=on`** and let the service redeploy. The log should say
   `vault on: 0 secrets, 1 owner keys`. If it says `vault refused to start`
   instead, the line names the variable to fix.

5. **Point the CLI at it.**

   ```sh
   export TRIOS_VAULT_URL=https://<queen host>/vault
   trios-vault names     # prints nothing yet; a refusal here means the key or the URL is wrong
   ```

6. **Import from Infisical.** The values go from Infisical straight into the
   pipe. The CLI seals each one before it leaves your Mac.

   ```sh
   infisical login
   infisical export --env=prod --format=dotenv \
     | trios-vault import --stdin --source infisical:prod --format dotenv
   ```

7. **Import the Railway age file.** Use the key that file was sealed to. This
   is the `### <service>` plus `KEY=VALUE` format.

   ```sh
   age -d -i <the key railway-999-env.age was sealed to> ~/dr/railway-999-env.age \
     | trios-vault import --stdin --source railway:999 --format railway-kv
   ```

   Each Railway service becomes a scope holding its old names. The output
   lists the `RAILWAY_*` platform names it left out.

8. **Deduplicate.** The report names ids, names, sources and services, never
   a value.

   ```sh
   trios-vault dedup --plan-out plan.txt
   # edit plan.txt: uncomment a line, write the canonical id, and for a conflict add keep=<ID>
   trios-vault dedup --apply plan.txt
   ```

   Services keep their old names, which now resolve through aliases to the
   one kept ciphertext.

9. **Classify** what you imported. An imported value is `other`, which only
   the owner's machines may lease, until you name its class:
   `trios-vault class --class provider-api-key <ID> ...`. The classes are
   `provider-api-key`, `db-url`, `bot-token`, `signing-key`, `oauth` and `other`.

10. **Smoke check.** This proves injection without anyone seeing the value.
    The value is random and goes from `/dev/urandom` straight into the pipe.

    ```sh
    head -c 24 /dev/urandom | base64 | tr -d '\n' \
      | trios-vault put SMOKE_SECRET --stdin --scope smoke --class provider-api-key
    trios-vault grant --key <your 64-hex public key> --tier owner --scope smoke
    trios-vault run --scope smoke -- sh -c 'test -n "$SMOKE_SECRET" && echo present'
    ```

    It prints `present`.

11. **Take an export** and keep it next to the recovery key:
    `trios-vault export --out vault-export.age`. It opens only with
    `age -d -i recovery.txt vault-export.age`.

**Rollback:** unset `TRIOS_VAULT`, or set it to anything but `on`, and
redeploy. The vault routes answer 503, and the rest of the server behaves as
before. `queen_vault_store` and the audit stream stay, unread. Dropping the
table is a separate decision and is not part of a rollback.

**Workloads:** give each service its own key (`trios-vault key new`) and a
grant (`trios-vault grant --key <hex> --tier trusted --scope <service>`).
Then start it with
`TRIOS_VAULT_URL=... TRIOS_VAULT_KEY=<its key file> trios-vault run --scope <service> -- <command>`.
`run` renews the lease at half its TTL and stops the child if a renewal is
refused.

## Local mode

Without `TRIOS_VAULT_URL`, the CLI opens a vault in `--dir`, `TRIOS_VAULT_DIR`
or `~/.trios-vault`, in the same process. Its identity is a file there, made by
`trios-vault init --recovery age1...`. Its audit goes to `audit.jsonl` in that
directory, or to the bus when `TRIOS_VAULT_DATABASE_URL` is set.
`trios-vault serve` serves such a vault on 127.0.0.1.

## Limits of slice 1

- One server holds the nonce cache in memory. With two replicas, a captured
  request could be replayed once on the other replica within 30 s. A replayed
  lease is worthless: its answer is sealed to the original requester's
  one-lease key.
- The vault checks that a row opens with its identity and has exactly two
  recipients. It cannot check that the second recipient is your recovery
  key, so that rests on the CLI, which seals to the recovery recipient the
  vault reports. The restore test and the export test show the recovery key
  opens what the CLI wrote.
- A request from a key the vault has not granted is refused without an audit
  row (policy.t27 `deny_recorded`), so junk traffic cannot grow the database;
  every refusal of a known key is recorded.
