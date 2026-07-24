# Wave Research Loop — Security Hardening Skill

## Description

Autonomous security-audit loop for the trios-mb Rust monorepo. Each wave scans for vulnerabilities, reviews scientific literature, decomposes findings into an implementation plan, remediates CRITICAL/HIGH issues, documents MEDIUM/LOW deferred items, and preserves discovered patterns as reusable skills.

## How to Use

1. Trigger with `/loop 15m` (or any interval).
2. The agent will:
   - Research weaknesses in the current codebase.
   - Review academic / industry papers matching the findings.
   - Create a decomposed plan (`WAVE_NNN_PLAN.md`).
   - Implement fixes (CRITICAL first).
   - Write `WAVE_NNN_REPORT.md` with verification, literature review, and three cooperation variants for the next wave.
   - Append new patterns to this skill file.
3. At the end of each wave, commit the report and skill updates.

---

## Pattern Library

### money-type
Replace `f64` monetary values with an integer-based `Money(i64)` in minor units to prevent silent financial drift.

**Reference:** `rings/GOLD-RING-TY00/src/money.rs`

### fsm-state-loss
Never use `unwrap_or_default()` on dialogue-state fields (`images`, `action`, etc.). Use explicit `match` that returns the user to the menu with a warning if state is missing.

**Reference:** `rings/SILVER-RING-SN00/src/morphing.rs`, `sticker.rs`

### log-injection-guard
Before logging attacker-controlled strings (URLs, payloads, filenames), truncate them:

```rust
const MAX_LOG_FIELD_LEN: usize = 128;
fn truncate_for_log(s: &str, max: usize) -> &str { &s[..s.len().min(max)] }
```

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`

### prompt-injection-scoring
Count matched prompt-injection indicators; reject if score ≥ threshold (default 2). Never rely on a single boolean match.

**Reference:** `rings/SILVER-RING-AI00/src/providers/mod.rs`

### circuit-breaker-jitter
Add `max_jitter: Duration` to circuit-breaker `reset_timeout`. Use `effective_reset_timeout()` when evaluating Open→HalfOpen transitions to prevent thundering-herd on recovery.

**Reference:** `rings/SILVER-RING-AI00/src/circuit_breaker.rs`

### bulkhead-semaphore
Place a `tokio::sync::Semaphore` (default 10 permits) per provider inside `AiOrchestrator.dispatch()`. Acquire a permit before calling `provider.generate()`; skip provider if bulkhead is saturated.

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### retry-budget-sliding-window
Track a `Vec<Instant>` of failure timestamps per provider. If `retries_in_window(window_secs) > max_retries`, suppress further retries until the window slides.

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### poison-pill-backoff
In worker loops, count `consecutive_failures`. Increase poll interval to 15s after 3 failures, 30s after 5 failures. Prevents a poison-pill job from starving the pool.

**Reference:** `rings/SILVER-RING-JB00/src/worker.rs`

### atomic-webhook-gate
For webhook idempotency, treat the DB INSERT as source of truth. Redis is only a fast-path cache; never gate idempotency solely on Redis.

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### ttl-in-memory-maps
Every unbounded `HashMap` / `DashMap` in long-lived server state (`ServerState`, `WorkflowState`, `AppState`) must have TTL eviction or a background cleanup task.

**Reference:** `rings/BRONZE-RING-SRV/src/lib.rs` (`BatchStore`), `rings/BRONZE-RING-SRV/src/workflow.rs` (`cleanup_old_runs`)

### atomic-idempotency-insert
For any "check-then-create" flow vulnerable to TOCTOU races (e.g., referral commissions, duplicate payments), collapse the guard into the DB layer:
```sql
INSERT INTO ... ON CONFLICT (idempotency_key) DO NOTHING
```
Return `true` if `rows_affected() > 0`, `false` if skipped. Never rely on application-level pre-checks for concurrency safety.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`create_transaction_and_credit_balance_idempotent`)

### streaming-byte-cap
Never trust `Content-Length` or provider-reported metadata for external streams. Wrap download sinks in an `AsyncWrite` adapter that aborts mid-stream when a hard byte cap is exceeded.

**Reference:** `rings/SILVER-RING-TG00/src/file_downloader.rs` (`LimitedWriter`)

### hot-cache-flush-interval
Any in-memory cache that accumulates events awaiting DB persistence must have a background `tokio::time::interval` task that drains the cache at a fixed cadence (e.g., every 60s). Tie the task to the process `TaskTracker` for graceful shutdown.

**Reference:** `rings/BRONZE-RING-SRV/src/analytics_collector.rs` (`drain`), `rings/BRONZE-RING-APP/src/main.rs`

### explicit-saturation-log
When a bounded ring buffer overflows and drops the oldest entry, emit a `tracing::warn!` with the event/entity identifier. Silent eviction is an operational blind spot.

**Reference:** `rings/BRONZE-RING-SRV/src/events.rs` (`post_event`, `post_job_event`)

### fire-and-forget-webhook-notify
Never `await` downstream API calls (Telegram Bot API, DB logging) inside webhook HTTP handlers. Wrap them in `tokio::spawn` so the handler returns `200 OK` / `202 Accepted` immediately while notifications proceed asynchronously.

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### webhook-metrics-per-provider
Add Prometheus histogram `webhook_processing_duration_seconds` and counter `webhook_processing_total` with a `provider` label. Measure only the success path to avoid skewing latency with signature-validation failures.

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### moderation-fail-open
External content-moderation APIs (OpenAI Moderations, etc.) must be wrapped with a short timeout and **fail open**: API errors, timeouts, and parse failures should all result in "allow" to prevent moderation downtime from blocking legitimate traffic.

**Reference:** `rings/SILVER-RING-AI00/src/moderation.rs`

### single-background-cleanup
Replace per-request spawned cleanup tasks with a **single** background `tokio::time::interval` loop tied to the process `TaskTracker`. Prevents unbounded task growth during traffic spikes.

**Reference:** `rings/BRONZE-RING-SRV/src/temp_storage.rs` (`spawn_periodic_cleanup`)

### stripe-failure-logging
Record webhook events for **all** branches of a webhook handler, not just the success path. Early returns (invalid signature, unsupported content type, invalid JSON) must also persist a `WebhookEvent` row for observability.

**Reference:** `rings/BRONZE-RING-SRV/src/billing.rs`

### webhook-retry-exponential-backoff-jitter
Webhook delivery workers must use a bounded exponential backoff with **full jitter**:
```rust
fn next_retry(retry_count: u32) -> Duration {
    let base = BASE_DELAY_SECS * 2u64.pow(retry_count.min(10));
    let capped = base.min(MAX_DELAY_SECS);
    Duration::from_secs(capped) + Duration::from_millis(rand::random::<u64>() % 1000)
}
```
Poll every 10s with a batch size ≤ 50. Always set an `Idempotency-Key` header equal to the webhook UUID.

**Reference:** `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`

### payment-verification-integer-cents
Never parse payment amounts from external gateways as `f64`. Convert immediately to integer minor units (cents / kopecks / nano) at the gateway boundary. The `PaymentVerification` struct must expose `amount_cents: i64`, not `amount: f64`.

**Reference:** `rings/SILVER-RING-PY00/src/robokassa.rs` (`parse_rub_to_cents`), `rings/GOLD-RING-TR00/src/payment_gateway.rs`

### atomic-temp-file-rename
Sensitive file writes (credentials, config, backups) must use a temp-file + atomic rename pattern. Set restrictive permissions (`0o600`) on the temp file **before** renaming.

```rust
let temp = path.with_extension("tmp");
tokio::fs::write(&temp, content).await?;
tokio::fs::set_permissions(&temp, Permissions::from_mode(0o600)).await?;
tokio::fs::rename(&temp, &path).await?;
```

**Reference:** `rings/BRONZE-RING-SRV/src/backup.rs` (`write_pgpass_file`)

### ip-rate-limiter-bounded-cap
Every `DashMap` / `HashMap` used for IP-level rate limiting must have a hard `MAX_ENTRIES` ceiling. When exceeded, evict the oldest 10% entries by timestamp. Emit a gauge metric (`ip_rate_limiter_entries`) for observability.

**Reference:** `rings/BRONZE-RING-SRV/src/middleware.rs` (`IpRateLimiter`)

### compliance-flags-generation
Add boolean compliance columns (`c2pa_signed`, `watermark_applied`) to the `generations` table and `GenerationResult` type early, even before the actual pipeline integration. This prevents schema-migration churn later and gives the frontend a stable API contract.

**Reference:** `rings/GOLD-RING-TY00/src/generation.rs`, `rings/SILVER-RING-DB00/src/migration_generations_compliance.rs`

---

## Literature Index

| Topic | Reference |
|-------|-----------|
| Circuit Breaker, Bulkhead, Timeout | Nygard, *Release It!* 2nd Ed. |
| Retry Budgets | Beyer et al., *Site Reliability Engineering* (Google) |
| Circuit-Breaker Jitter | Fowler, martinfowler.com/bliki/CircuitBreaker.html |
| Input Validation / Prompt Injection | OWASP Input Validation Cheat Sheet |
| Unbounded Resource Consumption | CWE-20, CWE-770, CWE-400 |
| Exponential Backoff & Jitter | AWS Builders' Library |
| SSRF Prevention | OWASP SSRF Cheat Sheet |
| TOCTOU File Race Conditions | Microsoft Security Blog, CWE-362 |
| Financial Precision (Minor Units) | Stripe API Docs — Amount in smallest currency unit |

---

### cancellation-token-background-loops
Every background `tokio::spawn` or `task_tracker.spawn` that contains an infinite `loop` must accept a `CancellationToken` and use `tokio::select!` with `token.cancelled()` as a shutdown branch. This prevents hanging during graceful shutdown and rolling deploys.

```rust
loop {
    tokio::select! {
        _ = interval.tick() => {}
        _ = token.cancelled() => {
            tracing::info!("Task shutting down gracefully");
            break;
        }
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/lib.rs`, `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`, `rings/BRONZE-RING-SRV/src/temp_storage.rs`

### never-silently-drop-db-errors
Never use `let _ = db.mark_...().await`. If the DB is temporarily unavailable the webhook / job / event will not be persisted and will be re-processed infinitely. Always log at `error!` level when persistence fails.

```rust
if let Err(e) = db.mark_outgoing_webhook_failed(id, msg, None).await {
    tracing::error!(error = %sanitize_db_error(&e), webhook_id = %id, "Failed to mark webhook failed");
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`

### redis-pubsub-reconnect-backoff
Redis pub/sub `subscribe` must be wrapped in an outer reconnect loop with exponential backoff (1s → capped at 60s). Each stage (open client, get pubsub, subscribe) is retried independently. Respect `CancellationToken` between retries.

**Reference:** `rings/BRONZE-RING-SRV/src/redis_cache.rs`

### module-level-allow-dead-code-is-debt
A module-wide `#![allow(dead_code)]` suppresses compiler signals about real architectural debt. Remove it. Instead attach `#[allow(dead_code)]` (with a comment) to each legitimately unused item so the debt is visible and countable.

**Reference:** `rings/GOLD-RING-PR00/src/replicate.rs`

### n-plus-one-batch-load
When a loop issues a separate query per iteration (e.g., `for row in rows { get_team(row.id) }`), collapse it into a fixed number of batch queries (e.g., `WHERE id IN (...)`), then assemble the result in memory. Limit batch sizes to prevent overly-large `IN` clauses.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`list_teams_for_user`)

### sse-cancel-token-graceful-shutdown
SSE streams in Axum block hyper graceful shutdown by default. Add a `CancellationToken` branch inside every `async_stream::stream!` loop that uses `tokio::select!`:

```rust
loop {
    tokio::select! {
        result = broadcast.next() => { /* ... */ }
        _ = heartbeat.tick() => { yield ... }
        _ = token.cancelled() => {
            tracing::info!("SSE stream shutting down gracefully");
            break;
        }
    }
}
```

The connection slot (via `Drop` guard) will be released automatically on break.

**Reference:** `rings/BRONZE-RING-SRV/src/events.rs`

### redis-publish-never-silent
Never `let _ = cache.publish(...).await`. Redis publish failures (partition, maxmemory) silently drop events. Always log at `error!` and emit a counter metric:

```rust
if let Err(e) = cache.publish("events:generation", &event).await {
    tracing::error!(error = %e, "Redis publish failed");
    metrics::counter!("sse.redis_publish_failures_total", "channel" => "events:generation").increment(1);
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/events.rs`

### unimplemented-endpoint-no-db-mutation
If an endpoint is not yet implemented, return `501 Not Implemented` (or `503`) **before** any DB writes, rate-limit consumption, or state mutation. Do not burn user quota on a no-op.

**Reference:** `rings/BRONZE-RING-SRV/src/social_scraping.rs`

### balance-deduction-before-generation-creation
Every API endpoint that creates a billable generation must deduct user balance **before** calling `create_generation`. Refund on failure so the user is not charged for broken generations. The `create_generation` repository method itself should **not** deduct balance (it is a pure insertion primitive).

**Reference:** `rings/BRONZE-RING-SRV/src/api_v1.rs` (`run_workflow`, `create_batch`)

### redirect-policy-ssrf-defense
`reqwest::Client` instances used for outgoing webhook delivery must disable redirects:
```rust
reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
```
Otherwise SSRF guards (IP/DNS checks) can be bypassed by a `302` to an internal/metadata endpoint.

**Reference:** `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`

### batch-toctou-atomic-per-item
Avoid batch-level pre-checks (e.g., quota) when each item is already validated inside an atomic transaction. The batch-level check creates a TOCTOU race: concurrent batches can both pass before either commits.

**Reference:** `rings/BRONZE-RING-SRV/src/api_v1.rs` (`create_batch`)

### env-var-master-key-hardening
Environment-variable fallback API keys must have:
1. Optional expiration (`API_KEY_EXPIRES_AT` RFC3339).
2. Per-IP rate limiting.
3. Structured audit logging on every successful use.

**Reference:** `rings/BRONZE-RING-SRV/src/api_v1.rs` (`api_key_middleware`)

### spawn-webhook-task-monitor
Never `let _ = tokio::spawn(async move { ... })` for background tasks. Use a named helper that monitors the JoinHandle and logs panics:
```rust
fn spawn_webhook_task<F>(desc: &'static str, fut: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let handle = tokio::spawn(fut);
    let _monitor = tokio::spawn(async move {
        if let Err(e) = handle.await {
            tracing::error!(task = %desc, error = %e, "Background task failed");
        }
    });
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### sanitize-error-deduplicate
Truncate-and-sanitize error strings should live in **one** crate (`trios_mb_types`) and be re-exported everywhere:
```rust
pub use trios_mb_types::sanitize_error;
```
This prevents divergence (e.g., one ring truncating at 200 chars while others use 250).

**Reference:** `rings/GOLD-RING-TY00/src/lib.rs` + 8 consumer crates.

### lazy-lock-env-constants
Hardcoded circuit-breaker, rate-limit, and TTL constants should be `std::sync::LazyLock` values reading from environment variables:
```rust
static FAILURE_THRESHOLD: LazyLock<u32> = LazyLock::new(|| {
    std::env::var("ORCHESTRATOR_FAILURE_THRESHOLD")
        .ok().and_then(|s| s.parse().ok()).unwrap_or(3)
});
```

### f64-is-finite-financial-guard
Every monetary `f64` value from untrusted input (user text, webhook callbacks, query params) must pass `is_finite()` **and** `> 0.0` (or `>= 0.0` for balances) before reaching business logic. `NaN` and `Inf` pass naive `> 0.0` checks silently.

```rust
if !amount.is_finite() || amount <= 0.0 {
    return Err(AppError::Validation(format!("amount must be finite and > 0: {}", amount)));
}
```

**Reference:** `rings/SILVER-RING-PY00/src/services/payment_processor.rs`, `rings/SILVER-RING-PY00/src/ton.rs`, `rings/SILVER-RING-SN00/src/payment.rs`

### centralized-validation-chokepoint
Place validation guards at the earliest **unified** entry point (e.g., `PaymentProcessor::create_payment`) rather than relying on each downstream gateway to defend itself. This prevents bypass chains when a new gateway is added without its own guard.

**Reference:** `rings/SILVER-RING-PY00/src/services/payment_processor.rs`

### signature-verification-is-not-semantics-verification
After HMAC signature verification succeeds, independently validate numeric and enum fields for sanity. A valid signature proves origin and integrity, not that the payload values are semantically correct.

**Reference:** `rings/SILVER-RING-PY00/src/robokassa.rs` (`verify_callback`), `rings/SILVER-RING-PY00/src/telegram_stars.rs`

### restart-limit-supervised-loops
Exponential backoff alone is insufficient for loops that restart on "normal" exit. Add a `restart_count` ceiling (`MAX_RESTARTS`) so persistent non-panic failures (network partitions, upstream outages) do not spin forever.

```rust
loop {
    // ... work ...
    restart_count += 1;
    if restart_count >= MAX_RESTARTS {
        tracing::error!("Exceeded max restarts; giving up");
        break;
    }
    tokio::time::sleep(backoff).await;
}
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### response-builder-fallback-helper
Eliminate `.unwrap()` on `Response::builder()` in request-handling code. Provide a `build_sanitized_response` helper that uses `.unwrap_or_else` to construct a manual fallback, removing the panic path entirely:

```rust
fn build_sanitized_response(code: StatusCode) -> Response {
    Response::builder()
        .status(code)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Body::from("Bad Request"))
        .unwrap_or_else(|_| {
            let mut resp = Response::new(Body::from("Bad Request"));
            *resp.status_mut() = code;
            resp
        })
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`
Production tuning no longer requires recompilation.

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### unbounded-hashmap-capacity
In-memory caches (`HashMap`, `DashMap`) need both **TTL eviction** and a **max capacity**. Under burst load, TTL-only caches can grow unbounded within the TTL window.

```rust
while map.len() > MAX_ENTRIES {
    let oldest = map.iter().min_by_key(|(_, v)| v.created_at).map(|(k, _)| *k);
    if let Some(k) = oldest { map.remove(&k); }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/workflow.rs`, `rings/BRONZE-RING-SRV/src/lib.rs` (`BatchStore`)

### for-update-row-lock
Use `SELECT ... FOR UPDATE` inside an explicit transaction before any balance mutation. This eliminates TOCTOU races where concurrent requests read the same pre-mutation snapshot.

```rust
let mut tx = pool.begin().await?;
let _row: (i64,) = sqlx::query_as("SELECT balance FROM users WHERE telegram_id = $1 FOR UPDATE")
    .bind(telegram_id)
    .fetch_one(&mut *tx)
    .await?;
// UPDATE now safe from concurrent races
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`deduct_balance`, `add_balance`)

### safe-limit-universal
Every list query — whether SeaORM `.limit()`, raw SQL `LIMIT $1`, or `IN` subquery — must pass its limit through a `safe_limit(limit)` clamp (bounds to `0..=10_000`). Audit with a grep for `fn list_` and verify each body calls `safe_limit`.

```rust
fn safe_limit(limit: i64) -> u64 {
    if limit <= 0 { 0 } else if limit > 10_000 { 10_000 } else { limit as u64 }
}
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`safe_limit`, `list_dead_letter_jobs`, `list_teams_for_user`)

### argon2-spawn-blocking
Memory-hard key derivation (Argon2id, bcrypt, scrypt) must run on a blocking thread pool:
```rust
tokio::task::spawn_blocking(move || derive_key(password, salt)).await?
```
Running inside an async task blocks the Tokio runtime and stalls all other futures on the same thread.

**Reference:** `rings/BRONZE-RING-SRV/src/backup.rs`

### redis-lua-atomic-pipeline
Combine INCR + EXPIRE (and similar multi-command sequences) into a single Lua script to eliminate race conditions and save a round-trip:
```lua
local val = redis.call('INCR', KEYS[1])
if val == 1 then
    redis.call('EXPIRE', KEYS[1], ARGV[1])
end
return val
```

**Reference:** `rings/BRONZE-RING-SRV/src/redis_cache.rs`

### sql-group-by-over-rust-aggregation
Push `COUNT(*)` / `SUM` / `AVG` aggregation to PostgreSQL rather than fetching thousands of rows into Rust:
```rust
"SELECT event_type, COUNT(*) as cnt FROM analytics_events GROUP BY event_type"
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### single-pass-string-escape
Replace chained `.replace()` calls (which create N intermediate strings) with a single pre-allocated pass:
```rust
let mut out = String::with_capacity(text.len() * 2);
for c in text.chars() {
    if NEEDS_ESCAPE.contains(c) { out.push('\\'); }
    out.push(c);
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs` (`escape_markdown_v2`)

### empty-slice-sql-guard
When building `IN (...)` placeholders from a user-provided slice, return early if the slice is empty to avoid producing invalid SQL (`IN ()`):
```rust
if job_types.is_empty() { return Ok(None); }
```

**Reference:** `rings/SILVER-RING-JB00/src/queue.rs`

### bola-post-filter-over-unscoped-query
When a DB method accepts only a secondary identifier (e.g., `bot_id`) and lacks a `telegram_id` parameter, add an ownership post-filter in the handler rather than waiting for a schema migration:

```rust
let items: Vec<Dto> = docs
    .into_iter()
    .filter(|d| d.telegram_id == caller_tid)
    .map(Dto::from)
    .collect();
```

This closes the IDOR window immediately. A follow-up migration can add the scoped DB method for efficiency.

**Reference:** `rings/BRONZE-RING-SRV/src/documents.rs`, `rings/BRONZE-RING-SRV/src/model_hub.rs`

### batch-uniform-ownership-guard
Before returning a batch response, verify that **every** item in the batch belongs to the same owner. If a cross-user batch is detected, return `403 Batch ownership conflict`:

```rust
let tid = first_gen.telegram_id;
if gens.iter().any(|g| g.telegram_id != tid) {
    return (StatusCode::FORBIDDEN, Json(json!({"error": "Batch ownership conflict"}))).into_response();
}
```

### sentinel-value-rejection
When an env-var loader returns a sentinel (e.g., `0`) to avoid panics, the sentinel must be explicitly rejected in every security predicate. Never assume the sentinel will never match a real identity.

```rust
const SUPER_ADMIN_SENTINEL: i64 = 0;

pub fn is_super_admin(user_id: i64) -> bool {
    if user_id == SUPER_ADMIN_SENTINEL {
        return false;
    }
    user_id == *SUPER_ADMIN_ID
}
```

**Reference:** `rings/SILVER-RING-TG00/src/access.rs`

### de-hardcode-security-identifiers
Deployment-specific identifiers (bot usernames, webhook endpoints, tenant names) must be loaded from environment variables with a safe fallback. Literal strings in `match` arms are CWE-547 violations.

```rust
pub static HAIM_GROUP_BOT_NAME: LazyLock<String> =
    LazyLock::new(|| load_bot_name("HAIM_GROUP_BOT_NAME", "HaimGroupMedia_bot"));

match bot_name {
    n if n == HAIM_GROUP_BOT_NAME.as_str() => HAIM_GROUP_STAFF_IDS.contains(&user_id),
    _ => false,
}
```

**Reference:** `rings/SILVER-RING-TG00/src/access.rs`

### never-unwrap-or-default-on-serialization
`serde_json::to_string(...).unwrap_or_default()` silently produces empty output in release builds. Always use `match` with a `tracing::warn!` so serialization failures are observable. `debug_assert!` is insufficient because it is stripped in release builds.

```rust
match serde_json::to_string(&value) {
    Ok(s) => s,
    Err(e) => {
        tracing::warn!(error = %e, "Serialization failed");
        String::new()
    }
}
```

**Reference:** `rings/GOLD-RING-PR00/src/telegram.rs`

### provider-response-format-drift-logging
When parsing provider API responses with `and_then` + `unwrap_or_default`, add an explicit `else` branch that logs the unexpected format. This makes schema changes visible within minutes rather than days.

```rust
match value.as_ref() {
    Some(o) if o.is_array() => { /* ... */ }
    Some(o) if o.is_string() => { /* ... */ }
    Some(o) => {
        tracing::warn!(preview = %truncate_for_log(&o.to_string(), 256), "Unexpected provider response format");
        Vec::new()
    }
    None => Vec::new(),
}
```

**Reference:** `rings/GOLD-RING-PR00/src/replicate.rs`

### missing-user-sentinel-observability
A DB lookup that returns a sentinel (e.g., `0.0` balance for a missing user) must log a warning. Otherwise enrollment bugs remain invisible.

```rust
match user {
    Some(u) => Ok(u.balance),
    None => {
        tracing::warn!(telegram_id, "get_balance called for non-existent user; returning 0.0 sentinel");
        Ok(0.0)
    }
}
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### instrument-access-control-methods
Policy-enforcement methods (`check_access`, `is_transition_allowed`, `get_allowed_transitions`) should carry `#[tracing::instrument]` so unauthorized access attempts are visible in distributed traces.

```rust
#[tracing::instrument(skip(self), fields(scene_id = %id.scene_name(), is_subscriber, is_admin))]
pub fn check_access(&self, id: &SceneId, is_subscriber: bool, is_admin: bool) -> bool { ... }
```

**Reference:** `rings/SILVER-RING-TG00/src/registry.rs`

**Reference:** `rings/BRONZE-RING-SRV/src/api_v1.rs` (`get_batch_status`)

### deprecated-hash-audit-trail
When a legacy cryptographic fallback must remain for backward compatibility, do not remove it silently. Instead, track usage with a metric and log a deprecation warning so operators know when to rotate keys:

```rust
if used_legacy {
    tracing::warn!("Deprecated SHA-256 API key used — migrate to HMAC-SHA256");
    metrics::counter!("auth_legacy_sha256_total").increment(1);
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/api_v1.rs` (`api_key_auth_middleware`)

### negative-identifier-rejection
Every handler that accepts `telegram_id` (or any surrogate primary key) should reject values `<= 0` at the earliest boundary — ideally inside the centralized `check_ownership` function:

```rust
if resource_telegram_id <= 0 {
    tracing::warn!("Ownership check blocked: invalid telegram_id <= 0");
    return Some((StatusCode::BAD_REQUEST, ...).into_response());
}
```

This prevents downstream logic from operating on sentinel or corrupted values.

**Reference:** `rings/BRONZE-RING-SRV/src/api_v1.rs` (`check_ownership`)

### transaction-rollback-log
Never `let _ = txn.rollback().await;`. If rollback fails (network error, connection drop), row locks may be held indefinitely and the transaction state becomes unknown. Always log rollback failures at `error!` level before returning the application-level error.

```rust
if let Err(e) = txn.rollback().await {
    tracing::error!(error = %e, "Transaction rollback failed");
}
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### dispatcher-restart-loop
Any long-lived external listener (Telegram polling, Redis pub/sub, SSE stream) must be wrapped in an outer retry loop with exponential backoff and a `CancellationToken` branch. A single unrecoverable error must not permanently stop the listener until the process restarts.

```rust
loop {
    tokio::select! {
        _ = dispatch_with_listener(...) => { /* log and retry */ }
        _ = cancel.cancelled() => { break; }
    }
    tokio::time::sleep(backoff).await;
    backoff = std::cmp::min(backoff * 2, max_backoff);
}
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### magic-byte-upload-gate
After accepting a file upload based on `Content-Type`, validate the first N bytes against known magic signatures before writing to permanent storage. This prevents executable or HTML payloads disguised as benign types.

**Reference:** `rings/BRONZE-RING-SRV/src/temp_storage.rs`, `rings/SILVER-RING-TG00/src/file_downloader.rs`

### compile-time-auth-bypass
Test-only auth bypass flags should be `const bool`, not runtime-mutable `AtomicBool`. A compile-time constant cannot be accidentally toggled in production by a misconfigured feature flag or environment variable.

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### group-by-over-per-row-count
Replace per-row `COUNT(*)` loops with a single `GROUP BY` query. For bot health dashboards:

```sql
SELECT bot_id,
       COUNT(*)::bigint AS total,
       COUNT(*) FILTER (WHERE status = 'failed')::bigint AS failed
FROM generations
WHERE bot_id IS NOT NULL
GROUP BY bot_id
LIMIT 1000
```

This collapses up to 2,000 queries into one round-trip.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`get_bot_health`)

### pg-class-approximate-counts
For admin dashboards that do not need exact totals, query `pg_class.reltuples` instead of running `COUNT(*)` on large tables:

```sql
SELECT relname, reltuples::bigint AS approx
FROM pg_class
WHERE relname IN ('users', 'generations', ...)
  AND relkind = 'r'
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`get_system_stats`)

### async-mutex-over-sync-in-async-fn
Even if an `async fn` does not currently contain an `.await` inside the critical section, prefer `tokio::sync::Mutex` over `std::sync::Mutex`. Future edits may add an await, and the sync mutex will silently block the runtime worker thread.

**Reference:** `rings/BRONZE-RING-SRV/src/marketplace.rs` (`MarketplaceState`)

### startup-sync-io-deferral
Never perform synchronous filesystem IO inside constructors that are called during async runtime initialization. Defer `create_dir_all` / `canonicalize` to the first async method call (e.g., `run_backup`) and use `tokio::fs` there.

**Reference:** `rings/BRONZE-RING-SRV/src/backup.rs` (`BackupService::new`)

### dynamic-feature-flag-over-env-var
Replace `std::env::var("STRIPE_API_KEY").is_ok()` gating with `db.is_feature_enabled("stripe_checkout", None).await`. This allows dynamic toggling without restart and supports scoped rollouts (per-bot, per-team).

**Reference:** `rings/BRONZE-RING-SRV/src/billing.rs` (`create_checkout`)

### provider-config-health-endpoint
Add a lightweight `/health/providers` route that checks env-var presence for every external AI provider. Do **not** make live calls — this avoids rate-limiting provider APIs while still surfacing configuration drift:

```rust
let providers = vec!["FAL_KEY", "OPENAI_API_KEY", "REPLICATE_API_TOKEN", ...];
```

**Reference:** `rings/BRONZE-RING-SRV/src/health.rs` (`health_providers`)

### scene-state-loss-guard
After dialogue TTL expiry, all fields become `None` / default. Any handler that uses `unwrap_or_default()` on a state field and proceeds silently risks charging the user for work that is never performed. Add an explicit guard that refunds any already-deducted balance and returns the user to the menu with a clear message.

```rust
let audio_url = match state.audio_url.clone() {
    Some(url) if !url.is_empty() => url,
    _ => {
        if let Err(e) = db.add_balance_with_transaction(tid, COST).await { ... }
        bot.send_message(chat_id, "Session timed out. Please start again.").await?;
        return return_to_menu(&bot, &dialogue, chat_id, lang).await;
    }
};
```

**Reference:** `rings/SILVER-RING-SN00/src/ai_cover.rs`

### periodic-db-pruning
Any table that accumulates terminal rows (`completed`, `failed`, `delivered`) needs a background cleanup loop. Implement `purge_old_{table}(older_than_days)` in the repository, expose it on the trait, and wire it into an existing maintenance interval (or a new one) tied to the process `CancellationToken`.

**Reference:** `rings/SILVER-RING-JB00/src/worker.rs`, `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`

### redis-default-ttl-gate
Redis `set`/`set_bytes` methods without TTL are a silent operational time-bomb. Replace raw `SET` with `SET EX` using a conservative default TTL (e.g., 24 hours). Log a warning every time the default is applied so future callers are alerted to use `set_with_ttl` with an explicit domain-appropriate expiration.

**Reference:** `rings/BRONZE-RING-SRV/src/redis_cache.rs`

### manual-debug-for-non-debug-fields
When a struct contains types that do not implement `Debug` (e.g., `tokio::sync::broadcast::Sender`), implement `Debug` manually and use `finish_non_exhaustive()`:

```rust
impl std::fmt::Debug for EventState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventState")
            .field("event_buffer", &self.event_buffer)
            .finish_non_exhaustive()
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/events.rs` (`EventState`)

### expect-to-match-in-spawned-task
Replace `.expect()` inside `tokio::spawn` or `task_tracker.spawn` with `match` that logs and returns early. A panic in a spawned task aborts only that task, but it still drops in-flight work and may leave shared state inconsistent:

```rust
let client = match reqwest::Client::builder()...build() {
    Ok(c) => c,
    Err(e) => {
        tracing::error!(error = %e, "Failed to build reqwest client");
        return;
    }
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`

### tracing-instrument-handlers
Add `#[tracing::instrument(skip(...))]` to every `pub async fn` HTTP handler so spans propagate across await points. This enables distributed trace correlation and makes latency bottlenecks visible in Jaeger / Grafana Tempo.

**Reference:** `rings/BRONZE-RING-SRV/src/broadcasts.rs`, `rings/BRONZE-RING-SRV/src/referrals.rs`, `rings/BRONZE-RING-SRV/src/marketplace.rs`, `rings/BRONZE-RING-SRV/src/invoices.rs`, `rings/BRONZE-RING-SRV/src/scheduled_jobs.rs`, `rings/BRONZE-RING-SRV/src/user_notes.rs`, `rings/BRONZE-RING-SRV/src/documents.rs`, `rings/BRONZE-RING-SRV/src/model_hub.rs`, `rings/BRONZE-RING-SRV/src/teams.rs`

### env-var-dynamic-pricing
Hardcoded plan amounts and feature-flag thresholds should read from environment variables at startup, with safe integer fallbacks. This lets ops adjust pricing or limits without recompiling:

```rust
"starter" => std::env::var("PLAN_AMOUNT_STARTER")
    .ok().and_then(|s| s.parse().ok()).or(Some(499)),
```

**Reference:** `rings/BRONZE-RING-SRV/src/billing.rs` (`plan_amount`)

### redis-cached-provider-health
For health endpoints that must reflect actual subsystem liveness without hammering external APIs, cache a lightweight "last success" timestamp in Redis after every successful provider call:

```rust
let live_ok = if present {
    match cache.get(&format!("provider_health:{}", name)).await {
        Some(ts_str) => ts_str.parse::<i64>().ok().map(|ts| {
            let now = chrono::Utc::now().timestamp();
            now - ts < 3600
        }).unwrap_or(false),
        None => false,
    }
} else { false };
```

**Reference:** `rings/BRONZE-RING-SRV/src/health.rs` (`health_providers`)

### concurrent-webhook-spawn
When delivering webhooks in batches, spawn each delivery as an independent `tokio::task` instead of awaiting sequentially. Collect `JoinHandle`s and await them together. This prevents a single slow DNS or HTTP target from blocking the entire batch:

```rust
let mut handles = Vec::with_capacity(batch.len());
for wh in batch {
    let db = db.clone();
    let client = client.clone();
    handles.push(tokio::spawn(async move {
        deliver_webhook(db, client, wh).await;
    }));
}
for h in handles {
    let _ = h.await;
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`

### marketplace-lru-ttl-eviction
In-memory caches that map surrogate IDs to entities (e.g., `MarketplaceState`) need **both** TTL pruning and LRU cap eviction to prevent unbounded growth under burst load. Update `last_accessed` on every read and write. On insert, prune entries older than `MARKETPLACE_TTL_SECS`, then evict oldest by timestamp if `len() > MAX_MARKETPLACE_ENTRIES`.

**Reference:** `rings/BRONZE-RING-SRV/src/marketplace.rs` (`evict_if_needed`)

### f64-to-money-migration
When a monetary field is currently `f64` and used in arithmetic, migrate it to an integer-based `Money(i64)` type representing minor units (cents). Convert to `f64` **only** at the DB boundary. Use `saturating_add` for safe accumulation to prevent silent wrap-around:

```rust
let mut failed_cost = Money::ZERO;
let mut total_cost = Money::ZERO;
total_cost = total_cost.saturating_add(cost);
```

**Reference:** `rings/BRONZE-RING-SRV/src/api_v1.rs` (`create_batch`, `run_workflow`), `rings/GOLD-RING-TY00/src/money.rs`

### check-referrer-exists-before-create
Before creating a referral record, validate that the referrer user actually exists via `get_user_by_telegram_id`. Return a clear `400 Bad Request` if the referrer is missing, rather than letting the DB foreign-key constraint fail with a generic error:

```rust
match state.db.get_user_by_telegram_id(req.referrer_id).await {
    Ok(Some(_)) => {}
    Ok(None) => {
        return (StatusCode::BAD_REQUEST,
                Json(json!({"error": "referrer not found"}))).into_response();
    }
    Err(e) => { /* ... */ }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/referrals.rs` (`claim_referral`)

### dead-code-sql-cleanup
When a SQL query-building function has a shadowed `values` or `conditions` vector (e.g., from an earlier refactor), remove the dead block immediately. It creates confusion and risks future edits being applied to the wrong branch, causing runtime parameter mismatches.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`get_analytics_aggregate`)

### bulk-update-over-n-plus-one
Replace per-row `SELECT` + `UPDATE` loops in admin batch operations with a single bulk `UPDATE ... WHERE id = ANY($1)`. This collapses 2N queries into one round-trip and avoids holding row locks for extended periods:

```rust
let sql = r#"
    UPDATE users
    SET is_active = false,
        updated_at = NOW()
    WHERE telegram_id = ANY($1)
      AND is_active = true
"#;
```

Always guard with `if ids.is_empty() { return Ok(0); }` to avoid invalid `ANY()`.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`bulk_ban_users`, `batch_update_users`)

### webhook-delivery-semaphore
Cap concurrent outgoing webhook deliveries with a `tokio::sync::Semaphore` to prevent HTTP connection-pool exhaustion and file-descriptor starvation. Acquire an owned permit per delivery and hold it until the HTTP response is processed:

```rust
let semaphore = Arc::new(tokio::sync::Semaphore::new(20));
let permit = semaphore.clone().acquire_owned().await?;
tokio::spawn(async move {
    let _permit = permit;
    deliver_webhook(db, client, wh).await;
});
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`

### stripe-plan-cache-over-amount-mapping
Do not map Stripe checkout sessions to subscription plans by exact cent amount. Tax, coupons, and env-var overrides break the mapping. Instead, cache `plan_id` in Redis at checkout creation (`stripe_session_plan:{session_id}`) and read it back in the webhook handler:

```rust
let plan = match server_state.cache.get(&format!("stripe_session_plan:{}", session.id)).await {
    Some(plan_id) if !plan_id.is_empty() => plan_id,
    _ => amount_to_plan(amount_cents).to_string(),
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/billing.rs` (`create_checkout`, Stripe webhook handler)

### health-check-db-connectivity
A `/health` endpoint that returns static JSON without verifying the primary data store is an anti-pattern. Add at least a lightweight DB `ping()` and return `503` if disconnected, so load balancers stop routing traffic to degraded instances:

```rust
let db_ok = matches!(state.db.health_check().await, Ok(true));
if db_ok { Json(json!({"status": "ok"})) } else {
    (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"db": "disconnected"}))).into_response()
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/health.rs` (`health_check`)

### job-queue-limited-retry-stuck
Bound `UPDATE` statements that retry stuck jobs with a CTE + `LIMIT` to prevent table-wide locks during large outages:

```sql
WITH stuck AS (
    SELECT id FROM job_queue
    WHERE status = 'running' AND started_at < NOW() - INTERVAL '1 second' * $1
    LIMIT 1000
)
UPDATE job_queue SET status = 'queued', ... FROM stuck WHERE job_queue.id = stuck.id
```

**Reference:** `rings/SILVER-RING-JB00/src/queue.rs` (`retry_stuck`)

### robokassa-redis-setnx-gate
Close the TOCTOU window on payment-webhook retries by setting a short-lived Redis `SET NX` gate **before** calling the DB completion method. If the gate already exists, return `200 OK` immediately (idempotent). If the DB transaction fails, the TTL auto-expires so the next retry can proceed:

```rust
let gate_acquired = cache.set_nx_ttl(&format!("robokassa_gate:{}", tx_id), "1", 60).await;
if !gate_acquired {
    return (StatusCode::OK, Json(json!({"status": "ok"}))).into_response();
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/payment_webhooks.rs` (`robokassa_callback`)

### orchestrator-global-timeout
Wrap the entire AI orchestrator dispatch retry loop in `tokio::time::timeout` to prevent unbounded latency accumulation when multiple providers are slow:

```rust
match tokio::time::timeout(Duration::from_secs(120), self.dispatch_inner(request)).await {
    Ok(result) => result,
    Err(_) => Err(AppError::Ai(AiError::AllProvidersFailed { ... })),
}
```

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### latency-tracker-vecdeque
Replace `Vec<Duration>` with `VecDeque<Duration>` for fixed-size sliding windows. `pop_front()` and `push_back()` are O(1), eliminating O(n) shifts from `Vec::remove(0)` under high load:

```rust
struct LatencyTracker {
    latencies: VecDeque<Duration>,
}
fn record(&mut self, latency: Duration) {
    if self.latencies.len() >= WINDOW_SIZE { self.latencies.pop_front(); }
    self.latencies.push_back(latency);
}
```

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### cdn-origin-allowlist
Validate source URLs against a comma-separated `CDN_ALLOWED_ORIGINS` env-var before rewriting them through a CDN. This prevents CDN abuse (cloaking, bandwidth theft) and reduces unexpected egress costs:

```rust
let allowed: Vec<String> = std::env::var("CDN_ALLOWED_ORIGINS")
    .map(|s| s.split(',').map(|o| o.trim().to_lowercase()).collect())
    .unwrap_or_default();
```

**Reference:** `rings/BRONZE-RING-SRV/src/cdn.rs`

### x-forwarded-for-configurable-direction
Make `X-Forwarded-For` extraction direction configurable via env var. AWS ALB / Cloudflare append proxies at the end, so the **first** IP is the client. Other stacks append at the front, so the **last** IP is the client:

```rust
let take_first = std::env::var("X_FORWARDED_FOR_FIRST")
    .map(|v| v.eq_ignore_ascii_case("true"))
    .unwrap_or(false);
let ip = if take_first { parts.find(...) } else { parts.rfind(...) };
```

**Reference:** `rings/BRONZE-RING-SRV/src/middleware.rs`

### quota-middleware-fail-closed
Quota and rate-limit enforcement must fail **closed** (deny access) when the underlying database or cache is unreachable. Returning `None` (allow) on DB errors creates a silent bypass during outages:

```rust
Err(e) => {
    tracing::warn!(..., "Failed to get daily generation count; failing quota closed");
    return Some((StatusCode::TOO_MANY_REQUESTS, Json(...)).into_response());
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/middleware.rs` (`check_daily_quota`)

### retry-budget-pre-dispatch-check
A retry budget must be checked **before** attempting a provider, not just recorded on failure. Otherwise the budget is a passive metric rather than an active throttle:

```rust
if let Some(budget) = budgets.get_mut(name) {
    if budget.remaining() == 0 {
        tracing::warn!(provider = name, "Retry budget exhausted; skipping provider");
        continue;
    }
}
```

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### ssrf-port-restriction
Validating URL scheme and IP ranges is insufficient for SSRF prevention. Restrict to standard HTTPS ports (`443`, `8443`) to prevent attacks against internal SSH/SMTP/metadata services on arbitrary ports:

```rust
if let Some(port) = url.port() {
    if port != 443 && port != 8443 {
        return Err("non-standard ports are not allowed");
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/social_scraping.rs`

### conflicting-parameter-rejection
When a handler accepts the same identifier from multiple sources (e.g., JSON body and HTTP header), reject the request if the values differ. Silent precedence rules create data-scoping confusion and potential BOLA:

```rust
let bot_id = match (req.bot_id, header_bot_id) {
    (Some(body), Some(header)) if body != header => {
        return (StatusCode::BAD_REQUEST, Json(...)).into_response();
    }
    (Some(body), _) => Some(body),
    (None, header) => header,
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/documents.rs`

### sql-aggregate-over-app-accumulation
Push `SUM`, `COUNT`, `AVG` aggregation to PostgreSQL rather than fetching rows into Rust and summing `f64` values. DB engines use higher internal precision and avoid order-dependent floating-point errors:

```rust
let total_cost: f64 = query
    .select_only()
    .column_as(g::Column::Cost.sum(), "total_cost")
    .into_tuple::<f64>()
    .one(pool)
    .await?
    .unwrap_or(0.0);
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`get_total_generation_cost`)

### composite-index-for-range-queries
Add composite indexes on columns frequently queried together with range filters. For transaction history endpoints querying by `telegram_id` with `ORDER BY created_at DESC`:

```sql
CREATE INDEX idx_payments_v2_telegram_id_created_at ON payments_v2 (telegram_id, created_at);
```

**Reference:** `rings/SILVER-RING-DB00/src/migration_baseline.rs`

### job-queue-pruning
Any queue table that accumulates terminal jobs (`completed`, `failed`, `cancelled`) needs a `purge_old_jobs(older_than_days)` method. Without pruning, `SKIP LOCKED` dequeue performance degrades linearly with table size:

```sql
DELETE FROM job_queue
WHERE status IN ('completed', 'failed', 'cancelled')
  AND updated_at < NOW() - INTERVAL '1 day' * $1
```

**Reference:** `rings/SILVER-RING-JB00/src/queue.rs`

### provider-cancel-stub-honesty
If a provider does not support cancellation, return a clear error instead of `Ok(())`. Silent no-ops mislead callers into believing resources were freed while upstream providers continue charging:

```rust
async fn cancel(&self, _generation_id: &str) -> Result<(), AppError> {
    Err(AppError::Ai(AiError::Provider {
        provider: "openai".to_string(),
        message: "Cancellation not supported by provider".to_string(),
    }))
}
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`

### f64-serialization-precision-control
Round `f64` values to a fixed number of decimal places before JSON serialization to prevent IEEE-754 imprecision leaking into API responses:

```rust
fn serialize_rating_one_decimal<S>(rating: &f64, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer,
{
    let rounded = (*rating * 10.0).round() / 10.0;
    serializer.serialize_f64(rounded)
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/marketplace.rs`

### content-type-upload-allowlist
Validate the `content-type` of file uploads against an explicit allowlist before writing bytes to disk. This prevents executable or dangerous file uploads disguised as benign types:

```rust
const ALLOWED_CONTENT_TYPES: [&str; 10] = [
    "image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml",
    "video/mp4", "video/webm", "audio/mpeg", "audio/wav", "audio/ogg",
];
```

**Reference:** `rings/BRONZE-RING-SRV/src/temp_storage.rs`

### moderation-before-optimisation
Any endpoint that sends user-controlled text to an external AI optimisation/completion API must run a content-moderation check **first**. If the text is flagged, return a localised rejection message **without** calling the external API. This prevents policy violations from reaching the provider account and protects platform reputation.

**Reference:** `rings/SILVER-RING-SN00/src/improve_prompt.rs`

### percent-encoded-path-traversal
Path-traversal defences must cover percent-encoded variants of dangerous sequences. Instead of pulling in a full URL-decoding crate, perform inline lower-case string checks for the encoded patterns:

```rust
let lower = path.to_lowercase();
if path.contains("..")
    || lower.contains("%2e%2e")
    || lower.contains("%2e.")
    || lower.contains(".%2e")
{ /* block */ }
```

**Reference:** `rings/BRONZE-RING-SRV/src/cdn.rs`

### request-body-limit-layer-order
Global `RequestBodyLimitLayer` values applied after per-route limits will **shadow** the more permissive route-specific values. Move the default limit into the individual sub-router builders (`build_admin_router`, `build_public_router`) rather than the top-level `assemble_router`, or apply limits explicitly per-route.

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### secretstring-event-signing
HMAC signing keys used for webhook/event integrity must be stored as `secrecy::SecretString` (not `String`) so the underlying buffer is zeroised on drop. Expose the raw bytes only at the signing boundary via `ExposeSecret`:

```rust
fn compute_signature(key: &SecretString, payload: &str) -> String {
    let bytes = key.expose_secret().as_bytes();
    // ... HMAC computation ...
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/events.rs`

### min-query-length-guard
Full-text or `ILIKE` search endpoints should reject queries below a minimum length (e.g., 3 characters) before hitting the database. Short wildcard queries (`%a%`) are expensive and often indicate enumeration attacks:

```rust
if query.trim().len() < 3 {
    return Ok(vec![]);
}
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### compile-time-auth-bypass-stable
When replacing `AtomicBool` with `const bool` for test-only auth bypass, remove **all** runtime `.store()` calls in tests. The feature flag alone controls the bypass. Ensure the test crate enables the feature in `Cargo.toml`:

```toml
[dependencies]
trios-mb-server = { workspace = true, features = ["test-auth-bypass"] }
```

**Reference:** `tests/e2e-tests/tests/e2e_health.rs`, `rings/BRONZE-RING-SRV/src/router.rs`

### secretstring-trait-migration
When migrating a trait method from `String` to `secrecy::SecretString`, change the trait definition first, then let the compiler guide you to every implementation and call site. Use `.expose_secret()` only at the cryptographic boundary (HMAC, HTTP header, encryption). Do **not** convert back to `String` for intermediate storage.

**Reference:** `rings/GOLD-RING-TR00/src/secret_store.rs`, `rings/BRONZE-RING-SRV/src/billing.rs`

### trait-timeout-parameter
Add a `timeout: std::time::Duration` parameter to every async trait method that calls an external network service. This prevents unbounded hangs from stalling the caller's Tokio worker thread. Default to 30s for idempotent verification calls.

**Reference:** `rings/GOLD-RING-TR00/src/payment_gateway.rs`

### cancellation-token-trait-parameter
Add `cancel: &tokio_util::sync::CancellationToken` to async trait methods that may invoke long-running external work (AI generation, video rendering, etc.). Check `cancel.is_cancelled()` at the start of the method and before each expensive external call. Return `AppError::Internal("cancelled")` if already cancelled.

**Reference:** `rings/GOLD-RING-TR00/src/ai_provider.rs`, `rings/SILVER-RING-AI00/src/orchestrator.rs`

### log-truncate-attacker-controlled
Before logging any attacker-controlled string (IDs, URLs, filenames, event IDs), truncate it:

```rust
const MAX_LOG_FIELD_LEN: usize = 128;
fn truncate_for_log(s: &str, max: usize) -> &str { &s[..s.len().min(max)] }
```

This prevents log-bloat DoS and audit-trail corruption.

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`, `rings/BRONZE-RING-SRV/src/user_webhooks.rs`, `rings/BRONZE-RING-SRV/src/webhooks.rs`

### input-length-guard-inline-const
Declare a small `const` cap at module level and validate attacker-controlled strings before they reach logging, filesystem, or DB code:

```rust
const MAX_MARKETPLACE_ID_LEN: usize = 128;
if id.len() > MAX_MARKETPLACE_ID_LEN {
    return (StatusCode::BAD_REQUEST, Json(json!({"error": "item id too long"}))).into_response();
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/marketplace.rs`, `rings/BRONZE-RING-SRV/src/temp_storage.rs`

### hmac-signed-url-idor-prevention
When serving user-uploaded files via public URL, append an HMAC-SHA256 signature as a query parameter (`?sig=...`) and verify it in the handler before returning bytes. This closes the IDOR window without requiring DB schema changes or auth middleware that might break existing download flows.

```rust
fn sign_file_name(secret: &SecretString, file_name: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.expose_secret().as_bytes()).unwrap();
    mac.update(file_name.as_bytes());
    hex::encode(&mac.finalize().into_bytes()[..16])
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/temp_storage.rs`

### default-rate-limit-null-columns
When a rate-limit configuration is stored in nullable DB columns, a `NULL` value must map to a **safe default** rather than "unlimited". Define constants at module scope and use `unwrap_or(DEFAULT)`:

```rust
const DEFAULT_API_KEY_RATE_LIMIT: i32 = 100;
const DEFAULT_API_KEY_RATE_WINDOW: i32 = 60;
let limit = key.rate_limit.unwrap_or(DEFAULT_API_KEY_RATE_LIMIT);
let window = key.rate_limit_window.unwrap_or(DEFAULT_API_KEY_RATE_WINDOW);
```

**Reference:** `rings/BRONZE-RING-SRV/src/api_v1.rs`

### csp-report-body-cap
CSP violation-report endpoints are a natural DoS vector because browsers POST JSON automatically. Apply a `RequestBodyLimitLayer` (e.g., 64 KB) and truncate the emitted description before persisting it to the security-event log:

```rust
const MAX_CSP_DESCRIPTION_LEN: usize = 2048;
let mut description = serde_json::to_string(&report)?;
if description.len() > MAX_CSP_DESCRIPTION_LEN {
    description.truncate(MAX_CSP_DESCRIPTION_LEN);
    description.push_str("...[truncated]");
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`, `rings/BRONZE-RING-SRV/src/csp_report.rs`

### read-limited-body-cancellation
HTTP body-read futures can stall a Tokio worker even after the caller has cancelled. Pass `&CancellationToken` into `read_limited_body` and wrap `resp.bytes().await` in `tokio::select!`:

```rust
pub async fn read_limited_body(
    resp: reqwest::Response,
    max_size: usize,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, AppError> {
    // ... content-length check ...
    let bytes = tokio::select! {
        result = resp.bytes() => result.map_err(|e| ...)?,
        _ = cancel.cancelled() => {
            return Err(AppError::Internal("generation cancelled during body read".into()));
        }
    };
    // ... size check ...
}
```

**Reference:** `rings/SILVER-RING-AI00/src/lib.rs`

### send-cancelable-helper
Never let a bare `.send().await` block a Tokio worker when the caller has already cancelled. Wrap reqwest (and any other long-running external) futures in `tokio::select!` with a `CancellationToken`:

```rust
pub async fn send_cancelable(
    req: reqwest::RequestBuilder,
    cancel: &CancellationToken,
) -> Result<Response, AppError> {
    tokio::select! {
        resp = req.send() => resp.map_err(|e| ...),
        _ = cancel.cancelled() => Err(AppError::Internal("cancelled".into())),
    }
}
```

**Reference:** `rings/SILVER-RING-AI00/src/lib.rs`

### connect-timeout-security-control
Treat `.connect_timeout()` as a security control, not just reliability. Without it, malicious endpoints can hang TCP handshakes indefinitely, exhausting dispatcher worker threads. Pair with `.redirect(Policy::none())` and a custom DNS resolver for defence-in-depth.

**Reference:** `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`

### spawn-traced-panic-monitoring
Replace bare `tokio::spawn` in production code with a helper that awaits the `JoinHandle` and logs panics at `error!` level. Silent panics in background tasks drop in-flight work and may leave shared state inconsistent.

```rust
fn spawn_traced<F>(desc: &'static str, fut: F)
where F: Future<Output = ()> + Send + 'static,
{
    let handle = tokio::spawn(fut);
    tokio::spawn(async move {
        if let Err(e) = handle.await {
            tracing::error!(task = %desc, error = %e, "Background task panicked");
        }
    });
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/billing.rs`

### panic-hook-secret-safe
Install a custom `std::panic::set_hook` that logs through `tracing::error!` instead of dumping a full backtrace to stderr. Backtraces can leak secrets, tokens, and PII from stack frames. Emit a `panic_total` counter metric for alerting:

```rust
std::panic::set_hook(Box::new(|info| {
    let msg = info.payload().downcast_ref::<&str>().unwrap_or(&"unknown panic");
    let location = info.location().map(|l| format!("{}:{}", l.file(), l.line()))
        .unwrap_or_else(|| "unknown location".to_string());
    tracing::error!(target: "panic", %msg, %location, "PANIC occurred");
    metrics::counter!("panic_total").increment(1);
}));
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### security-event-metrics
Every `emit_security_event` call must produce a structured Prometheus counter (`security_events_total`) with `kind` and `severity` labels. Without metrics, security events are invisible to operators and alerting pipelines:

```rust
metrics::counter!(
    "security_events_total",
    "kind" => event.kind.metric_label(),
    "severity" => event.kind.severity(),
).increment(1);
```

**Reference:** `rings/GOLD-RING-TY00/src/security_event.rs`

### public-media-signed-proxy
Never return raw external CDN URLs in public API responses. Instead, generate time-limited HMAC-signed URLs to a proxy endpoint that verifies the signature, checks the resource is public, and then redirects to the real URL:

```rust
fn build_media_url(id: Uuid, secret: &SecretString) -> String {
    let expiry = Utc::now().timestamp() + 3600;
    let sig = hmac_sign(secret, &format!("{}:{}", id, expiry));
    format!("/api/v1/public-generations/{}/media?sig={}&exp={}", id, sig, expiry)
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/public_generations.rs`

### sql-format-param-index-guard
When `format!` is used to inject SQL parameter placeholders (e.g., `${}`), wrap the index computation in a validation helper with a hard ceiling. This prevents accidental injection if a future refactor makes the index user-controlled:

```rust
fn validate_param_idx(idx: usize) -> Result<usize, AppError> {
    const MAX_SQL_PARAMS: usize = 100;
    if idx > MAX_SQL_PARAMS {
        return Err(AppError::Db(DbError::Query("too many SQL parameters".into())));
    }
    Ok(idx)
}
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### admin-endpoint-pii-minimization
Admin endpoints should return only the fields strictly necessary for the task. Remove financial PII (`balance`, `daily_generation_limit`, etc.) from impersonation and audit endpoints unless the caller explicitly requests a financial report:

```rust
let dto = serde_json::json!({
    "id": user.id,
    "telegram_id": user.telegram_id,
    // "balance": user.balance, // intentionally omitted
});
```

**Reference:** `rings/BRONZE-RING-SRV/src/impersonate.rs`

### test-auth-bypass-production-guard
Compile-time feature flags that disable authentication must be guarded with a startup panic so they can never be accidentally enabled in production builds:

```rust
#[cfg(feature = "test-auth-bypass")]
{
    panic!("FATAL: test-auth-bypass feature is enabled. Never use in production.");
}
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### url-validation-chain
Validate every URL before redirecting, storing in the DB, or passing to an external client. Checks: scheme whitelist (`http`/`https`), no embedded credentials, no loopback / private / ULA IPs, no `localhost` hostname, and a max length cap (4096). Apply the same validator at every ingestion point (admin endpoint, webhook handler, public proxy):

```rust
pub fn validate_result_url(url: &str) -> Result<(), String> {
    const MAX_URL_LEN: usize = 4096;
    if url.len() > MAX_URL_LEN { return Err("url too long".into()); }
    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("invalid scheme".into());
    }
    if parsed.password().is_some() { return Err("embedded credentials".into()); }
    if let Some(host) = parsed.host() {
        if host.to_string().eq_ignore_ascii_case("localhost") { return Err("localhost".into()); }
        if let std::net::IpAddr::V4(ip) = host {
            if ip.is_loopback() || ip.is_private() { return Err("private ip".into()); }
        }
        if let std::net::IpAddr::V6(ip) = host {
            if ip.is_loopback() || (u128::from_be_bytes(ip.octets()) & 0xfe00_0000_0000_0000_0000_0000_0000_0000) == 0xfc00_0000_0000_0000_0000_0000_0000_0000 {
                return Err("ipv6 ula".into());
            }
        }
    }
    Ok(())
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/utils.rs`, `rings/BRONZE-RING-SRV/src/public_generations.rs`, `rings/BRONZE-RING-SRV/src/admin_update_generation_status.rs`, `rings/BRONZE-RING-SRV/src/webhooks.rs`

### db-purge-terminal-rows
Every table that accumulates terminal-status rows (`completed`, `failed`, `cancelled`, `delivered`) needs a `purge_old_{table}(older_than_days)` method in the repository with a `LIMIT` cap (e.g., `10000`) to keep transactions short:

```rust
async fn purge_old_generations(&self, days: i32) -> Result<u64, DbError> {
    let sql = r#"
        DELETE FROM generations
        WHERE status IN ('completed', 'failed', 'cancelled')
          AND updated_at < NOW() - INTERVAL '1 day' * $1
        LIMIT 10000
    "#;
    sqlx::query(sql).bind(days).execute(&self.pool).await.map(|r| r.rows_affected())
        .map_err(|e| DbError::Query(e.to_string()))
}
```

Wire the purge into a background maintenance interval tied to the process `CancellationToken`.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`, `rings/GOLD-RING-TR00/src/database.rs`, `rings/BRONZE-RING-APP/src/main.rs`

### composite-index-migration
Add composite indexes for filter-and-sort queries that currently scan large tables. Use `IF NOT EXISTS` for idempotency:

```sql
CREATE INDEX IF NOT EXISTS idx_documents_telegram_id_created_at
    ON documents (telegram_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_generations_is_public_created_at
    ON generations (is_public, created_at DESC);
```

Register the migration in `lib.rs` and `migration.rs`.

**Reference:** `rings/SILVER-RING-DB00/src/migration_add_composite_indexes.rs`

### cors-no-localhost-fallback
Never fall back to `http://localhost:3000` or `http://127.0.0.1:3000` when `FRONTEND_URL` is unset. Removing the fallback forces operators to explicitly configure origins and prevents CSRF from locally running malicious sites:

```rust
let origins: Vec<HeaderValue> = std::env::var("FRONTEND_URL")
    .map(|s| s.split(',').map(|o| o.trim().parse::<HeaderValue>().unwrap()).collect())
    .unwrap_or_default();
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### bot-scoped-pii-redaction
In bot-scoped endpoints, omit sensitive identifiers (`telegram_id`, `balance`, etc.) from the response when the caller is a bot-scoped key, not the master key. This prevents a compromised bot key from enumerating all users who interacted with that bot:

```rust
let mut dto = serde_json::json!({ ... });
if !is_master {
    dto["telegram_id"] = json!(null);
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/api_v1.rs`

### ai-service-url-validation
Every AI service that writes a provider-supplied `result_url` to the database must validate it with `validate_result_url` before persisting. On failure, log a security event and set `result_url = None` rather than storing the bad URL:

```rust
let result_url = match result.result_url.as_deref() {
    Some(url) => match trios_mb_types::validate_result_url(url) {
        Ok(()) => Some(url),
        Err(e) => {
            tracing::warn!(url = %url, error = %e, "Provider returned invalid result_url");
            None
        }
    },
    None => None,
};
self.db.update_generation_status(gen.id, GenerationStatus::Completed, result_url, None).await?;
```

**Reference:** `rings/SILVER-RING-AI00/src/services/*.rs`

### backup-temp-file-cleanup
Ensure temporary credential files are removed even if the subprocess spawn fails. Place `remove_file` in an unconditional block before the success/error check:

```rust
let output_result = Command::new("pg_dump").env("PGPASSFILE", &pgpass_file).args([...]).output().await;
if let Err(e) = tokio::fs::remove_file(&pgpass_file).await {
    tracing::error!(...);
}
let output = match output_result {
    Ok(o) => o,
    Err(e) => { /* return error */ }
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/backup.rs`

### webhook-response-body-cap
Cap response body reads in webhook delivery workers to prevent OOM from malicious endpoints. Check `content_length` against a hard limit; if absent, read with `r.bytes()` and truncate post-read:

```rust
const MAX_WEBHOOK_RESPONSE_BYTES: usize = 1_048_576;
let body = match r.content_length() {
    Some(cl) if cl > MAX_WEBHOOK_RESPONSE_BYTES as u64 => {
        format!("[response body truncated: Content-Length {} bytes]", cl)
    }
    _ => match r.bytes().await {
        Ok(b) if b.len() <= MAX_WEBHOOK_RESPONSE_BYTES => String::from_utf8_lossy(&b).into_owned(),
        Ok(b) => { /* truncate */ }
        Err(_) => String::new(),
    },
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhook_delivery_worker.rs`

### nan-float-guard
Any `f64` parameter that undergoes range validation must also be checked with `is_finite()`. `NaN` bypasses all `<` and `>` comparisons:

```rust
if !req.limit.is_finite() || req.limit < 0.0 || req.limit > 1_000_000.0 {
    return (StatusCode::BAD_REQUEST, Json(...)).into_response();
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/team_spending.rs`

### recursive-json-sanitizer
Before echoing untrusted JSON to a client, recursively sanitize it: cap object key length, limit array size, cap string length, validate URL-like strings, and reject unknown structures:

```rust
fn sanitize_scrape_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => { /* limit key len, recurse */ }
        serde_json::Value::Array(arr) => { /* limit len, recurse */ }
        serde_json::Value::String(s) => { /* cap len, validate URLs */ }
        other => other,
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/social_scraping.rs`

### money-type-trait-migration
When migrating financial fields from `f64` to an integer-based `Money` type, change the trait boundary first, add repository shims that convert `Money` ↔ `f64` for the DB, then update call sites:

```rust
// Trait boundary (clean)
async fn deduct_balance(&self, telegram_id: i64, amount: Money) -> Result<bool, AppError>;

// Repository shim (transitional)
let amount_f64 = amount.minor_units() as f64 / 100.0;
// ... DB query still uses f64 until schema migration ...
```

**Reference:** `rings/GOLD-RING-TR00/src/database.rs`, `rings/SILVER-RING-DB00/src/repository.rs`

### money-overflow-checked-arithmetic
Monetary constructors that multiply by a scaling factor (e.g., major units → minor units) must use `checked_mul`, not bare `*`:

```rust
pub const fn from_major(major: i64) -> Self {
    match major.checked_mul(100) {
        Some(v) => Self(v),
        None => panic!("Money::from_major overflow"),
    }
}
pub const fn try_from_major(major: i64) -> Option<Self> {
    match major.checked_mul(100) { Some(v) => Some(Self(v)), None => None }
}
```

**Reference:** `rings/GOLD-RING-TY00/src/money.rs`

### toctou-remove-pre-check
Never use application-level "check then insert" for uniqueness-critical flows. Remove the pre-check and let the DB unique constraint act as the single source of truth. Map the DB duplicate/constraint error to the appropriate HTTP status:

```rust
match state.db.create_referral(req.referrer_id, req.referred_id).await {
    Ok(()) => (StatusCode::OK, ...).into_response(),
    Err(AppError::Db(DbError::Query(ref msg))) if msg.contains("duplicate") => {
        (StatusCode::CONFLICT, ...).into_response()
    }
    Err(e) => { /* 500 */ }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/referrals.rs`

### secretstring-env-read
Read sensitive environment variables into `secrecy::SecretString` immediately, exposing the raw value only at the cryptographic boundary:

```rust
let secret = match std::env::var("ADMIN_API_SECRET") {
    Ok(s) if !s.is_empty() => secrecy::SecretString::new(s.into_boxed_str()),
    _ => return None,
};
let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())?;
```

**Reference:** `rings/BRONZE-RING-SRV/src/admin.rs`, `rings/BRONZE-RING-SRV/src/api_v1.rs`, `rings/BRONZE-RING-SRV/src/referrals.rs`

### health-provider-enumeration-leak
Health endpoints that return per-subsystem booleans leak deployment topology. Remove `configured` / `enabled` fields from public JSON; only return live-health status. Use `var_os` instead of `var` to avoid materializing secret values:

```rust
let present = std::env::var_os(env_key).map(|v| !v.is_empty()).unwrap_or(false);
let entry = json!({ "live_healthy": live_ok }); // no "configured"
```

**Reference:** `rings/BRONZE-RING-SRV/src/health.rs`

### secretstring-provider-api-key
All external API provider structs must store credentials as `secrecy::SecretString`, never `String`. Expose the raw value only when building the HTTP Authorization header:

```rust
pub struct OpenAiProvider {
    api_key: secrecy::SecretString,
    // ...
}

// Usage at HTTP boundary only:
.header("Authorization", format!("Bearer {}", self.api_key.expose_secret()))
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`, `replicate.rs`, `fal.rs`, `elevenlabs.rs`, `heygen.rs`, `hedra.rs`, `kie.rs`

### cors-explicit-allowlist
Never use `CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)` in production. Read allowed origins from `FRONTEND_URL` and restrict methods/headers explicitly:

```rust
let allowed_origins: Vec<String> = std::env::var("FRONTEND_URL")
    .map(|s| s.split(',').map(|o| o.trim().to_string()).collect())
    .unwrap_or_default();
let origins: Vec<HeaderValue> = allowed_origins
    .into_iter()
    .map(|o| HeaderValue::from_str(&o).unwrap_or(HeaderValue::from_static("*")))
    .collect();
CorsLayer::new()
    .allow_origin(origins)
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([CONTENT_TYPE, AUTHORIZATION])
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### webhook-uuid-validation
Never use `unwrap_or_default()` when parsing user-supplied UUIDs in webhook handlers. Return `400 Bad Request` on malformed input instead of silently creating nil UUIDs:

```rust
fn parse_uuid(s: &str) -> Result<Uuid, (StatusCode, String)> {
    Uuid::parse_str(s)
        .map_err(|e| {
            tracing::warn!(input = %s, error = %e, "Invalid UUID in webhook");
            (StatusCode::BAD_REQUEST, format!("Invalid UUID: {}", e))
        })
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### webhook-db-error-logging
Replace `let _ = db.update_...().await` in webhook handlers with explicit error logging. Silent DB failures mean webhook events are not persisted and may be re-processed infinitely:

```rust
if let Err(e) = state.db.update_generation_status(id, status, url, None).await {
    tracing::error!(error = %e, generation_id = %id, "Failed to update generation status on webhook");
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`, `payment_webhooks.rs`

### job-deserialization-fail-closed
Background job workers must reject malformed payloads rather than silently defaulting. A deserialization failure should return an error immediately, not proceed with a zeroed/default request:

```rust
let request = serde_json::from_value::<GenerationRequest>(payload)
    .map_err(|e| {
        tracing::error!(job_id = %job.id, error = %e, "Failed to deserialize generation request");
        AppError::Validation(format!("Invalid job payload: {}", e))
    })?;
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### circuit-breaker-seqcst
Circuit breaker atomic operations must use `Ordering::SeqCst` (not `Relaxed`) for the failure count and open-state flag. `Relaxed` allows torn reads/writes when multiple Tokio workers dispatch concurrently:

```rust
pub fn allow_request(&self) -> bool {
    if !self.is_open.load(Ordering::SeqCst) { return true; }
    // ...
}
```

**Reference:** `rings/SILVER-RING-AI00/src/circuit_breaker.rs`

### reqwest-connect-timeout
Every `reqwest::Client` must set both `.timeout()` (total request) and `.connect_timeout()` (TCP handshake). Without the latter, malicious endpoints can hang worker threads indefinitely during connection establishment:

```rust
reqwest::Client::builder()
    .timeout(Duration::from_secs(60))
    .connect_timeout(Duration::from_secs(10))
    .build()
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`, `replicate.rs`

### deduct-balance-result-handling
Balance deduction returns `bool`, not `Result`. Never silently ignore a `false` return; always notify the user and abort the operation:

```rust
if !deduct_balance(&db, tid, cost).await {
    tracing::error!(telegram_id = tid, "Failed to deduct balance");
    bot.send_message(chat_id, "❌ Failed to deduct balance. Please try again later.").await?;
    return Ok(());
}
```

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`

### provider-error-body-logging
On non-2xx responses, use `resp.text().await.unwrap_or_else(|e| format!("[body unreadable: {}]", e))` instead of `unwrap_or_default()`. Silent empty bodies hide the actual server error:

```rust
if !status.is_success() {
    let text = resp.text().await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Failed to read provider error response body");
        format!("[body unreadable: {}]", e)
    });
    return Err(AiError::Provider { ... });
}
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`, `fal.rs`, `hedra.rs`, `kie.rs`, `replicate.rs`, `elevenlabs.rs`, `heygen.rs`

### circuit-breaker-mutex-poison-recovery
Never `.unwrap()` a mutex lock in production. Match `Ok(guard)` / `Err(poisoned)` and recover with `into_inner()`:

```rust
match self.last_failure.lock() {
    Ok(guard) => guard,
    Err(poisoned) => {
        tracing::error!("CircuitBreaker mutex poisoned; resetting state");
        let guard = poisoned.into_inner();
        self.is_open.store(false, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        return true;
    }
}
```

**Reference:** `rings/SILVER-RING-AI00/src/circuit_breaker.rs`

### orchestrator-global-timeout
Wrap provider dispatch retry loops in `tokio::time::timeout` to prevent unbounded latency accumulation when multiple providers are slow:

```rust
async fn dispatch(&self, request: &GenerationRequest) -> Result<GenerationResult, AppError> {
    match tokio::time::timeout(Duration::from_secs(120), self.dispatch_inner(request)).await {
        Ok(result) => result,
        Err(_) => Err(AppError::Ai(AiError::AllProvidersFailed { ... })),
    }
}
```

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### critical-refund-logging
When refunding balance after a failure, log at `error!` level if the refund itself fails. This is a double-fault condition:

```rust
if let Err(refund_err) = db.add_balance(tid, cost).await {
    tracing::error!(telegram_id = tid, error = %refund_err, "CRITICAL: Failed to refund balance");
}
```

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`

### telegram-id-sentinel-guard
Every `msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0)` extraction must be followed by an `if tid == 0 { return Ok(()); }` guard before any DB or chargeable operation:

```rust
let tid = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
if tid == 0 {
    tracing::warn!("Missing telegram_id; aborting handler");
    return Ok(());
}
```

**Reference:** `rings/SILVER-RING-SN00/src/` (29 handlers)

### cors-origin-filter-not-fallback
Invalid CORS origins must be filtered and logged, not silently replaced with a wildcard `*`:

```rust
for o in allowed_origins {
    match http::HeaderValue::from_str(&o) {
        Ok(hv) => origins.push(hv),
        Err(e) => tracing::warn!(origin = %o, error = %e, "Invalid CORS origin; skipping"),
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### orchestrator-status-timeout
Provider `check_status` and `get_result` calls must be wrapped in `tokio::time::timeout` to prevent unbounded hangs on slow providers:

```rust
match tokio::time::timeout(Duration::from_secs(30), provider.check_status(generation_id)).await {
    Ok(result) => result,
    Err(_) => {
        tracing::warn!(provider = provider_name, generation_id, "check_status timed out");
        Err(AppError::Ai(AiError::Provider { ... }))
    }
}
```

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### serde-fail-closed
Never use `unwrap_or_default()` on `serde_json::to_value`. Return an explicit error on serialization failure:

```rust
let payload = serde_json::to_value(&request).map_err(|e| {
    tracing::error!(error = %e, "Failed to serialize generation request");
    AppError::Validation(format!("Failed to serialize request: {}", e))
})?;
```

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`

### msg-text-match-guard
Never use `msg.text().unwrap()` in Telegram bot handlers. If the user sends a photo, sticker, or voice message while in a text-expecting state, the bot will panic. Always match on `msg.text()` and return a localized error if `None`:

```rust
let text = match msg.text() {
    Some(t) => t.to_string(),
    None => {
        let err = if lang.is_russian() { "❌ Отправьте текстовое сообщение" } else { "❌ Send text message" };
        bot.send_message(msg.chat.id, err).await?;
        return Ok(());
    }
};
```

**Reference:** `rings/SILVER-RING-SN00/src/instagram_scraping.rs`, `instagram_parser.rs`, `neuro_coder.rs`, `tech_support.rs`

### file-id-sentinel-guard
Never extract `file_id` from Telegram photos with `photos.last().map(|p| p.file.id.clone()).unwrap_or_default()`. An empty string gets stored in state and passed to downstream providers, causing cryptic errors. Use an explicit `match` that returns a localized error if no photo is found:

```rust
let file_id = match photos.last() {
    Some(p) => p.file.id.clone(),
    None => {
        let err = if lang.is_russian() { "❌ Не удалось получить изображение." } else { "❌ Could not retrieve image." };
        bot.send_message(msg.chat.id, err).await?;
        return Ok(());
    }
};
```

**Reference:** `rings/SILVER-RING-SN00/src/image_to_video.rs`, `flux_kontext.rs`, `remove_bg.rs`, `face_swap.rs`, `avatar_transform.rs`, `image_to_prompt.rs`, `image_upscaler.rs`, `hedra_render.rs`, `neuro_photo.rs`, `train_flux_model.rs`, `ai_photoshop.rs`, `digital_avatar_body.rs`, `morphing.rs`

### state-images-guard-before-dispatch
Before dispatching a generation job that depends on previously uploaded images, verify `state.images` is `Some` and contains the minimum required count. If not, refund any already-deducted balance and return the user to the menu:

```rust
let images = match state.images.as_ref() {
    Some(imgs) if imgs.len() >= 2 => imgs,
    _ => {
        let err = if lang.is_russian() { "❌ Нужно минимум 2 изображения." } else { "❌ Need at least 2 images." };
        bot.send_message(chat_id, err).await?;
        return return_to_menu(&bot, &dialogue, chat_id, lang).await;
    }
};
```

**Reference:** `rings/SILVER-RING-SN00/src/morphing.rs`, `train_flux_model.rs`

### webhook-task-id-explicit
Never use `unwrap_or_default()` on webhook payload fields that act as primary identifiers. Return a clear `400 Bad Request` before reaching UUID parsing or DB lookup:

```rust
let task_id = match payload.task_id.as_deref() {
    Some(t) => t,
    None => {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "missing task_id"})),
        );
    }
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### active-model-field-before-conversion
When updating a SeaORM ActiveModel field that may be `NULL`, extract the concrete value from the `Model` **before** calling `.into()` on it. The `into()` consumes the model, so any later access is a compile-time borrow-checker error:

```rust
let current_balance = user.balance;
let mut active: users::ActiveModel = user.into();
active.balance = Set(current_balance - amount);
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### secret-store-fail-fast
Never use `unwrap_or_default()` on `secret_store.get()` results for mandatory secrets. An unreachable secret store or missing key silently produces an empty string, leading to cryptic downstream auth failures. Fail fast with `?` and an explicit empty-string guard:

```rust
let token = secret_store.get("BOT_TOKEN_1").await?;
if token.is_empty() {
    anyhow::bail!("BOT_TOKEN_1 is required but empty");
}
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### tcp-bind-match-not-unwrap
Never `unwrap()` on `TcpListener::bind` or `axum::serve` in production entrypoints. If the port is in use, the entire process crashes. Use structured `match` that logs a fatal error and exits cleanly:

```rust
let listener = match tokio::net::TcpListener::bind(addr).await {
    Ok(l) => l,
    Err(e) => {
        tracing::error!(addr = %addr, error = %e, "Failed to bind HTTP listener");
        return Err(anyhow::anyhow!("Failed to bind: {}", e));
    }
};
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### callback-chat-id-match
Never `unwrap()` on `q.chat_id()` in Telegram callback handlers. Callback queries from channels or older clients may lack a chat context. Return early with `Ok(())` instead of panicking the dispatcher task:

```rust
let chat_id = match q.chat_id() {
    Some(id) => id,
    None => return Ok(()),
};
```

**Reference:** `rings/SILVER-RING-SN00/src/` (34 callback handlers)

### input-length-guard-uniform
Every free-text message handler should enforce a maximum input length before forwarding to downstream AI providers or the database. A multi-megabyte paste bomb can exhaust provider quota or overflow DB columns:

```rust
if text.len() > 4000 {
    let err = if lang.is_russian() { "❌ Текст слишком длинный." } else { "❌ Text too long." };
    bot.send_message(msg.chat.id, err).await?;
    return Ok(());
}
```

**Reference:** `rings/SILVER-RING-SN00/src/text_to_image.rs`, `text_to_video.rs`, `music_generation.rs`, `chat_with_avatar.rs`, `improve_prompt.rs`, `neuro_coder.rs`, `tech_support.rs`, `instagram_scraping.rs`, `instagram_parser.rs`

### provider-response-field-ok-or
Required fields from external provider JSON responses must use `ok_or_else(...)?` instead of `unwrap_or_default()`. Malformed or rate-limited responses should surface as explicit errors rather than silently producing empty defaults:

```rust
let status_str = data.status.ok_or_else(|| AiError::InvalidResponse {
    provider: "heygen".into(),
    message: "missing status field".into(),
})?;
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/heygen.rs`, `fal.rs`

```rust
let task_id = match payload.task_id.as_deref() {
    Some(t) => t,
    None => {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "missing task_id"})),
        );
    }
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### reqwest-builder-fail-fast
Replace `reqwest::Client::builder()...build().unwrap_or_default()` with `build().expect("Failed to build {provider} reqwest client")`. Builder failures silently fall back to a default `Client` with different timeout/connect settings, masking configuration errors. Fail fast at startup.

```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(60))
    .connect_timeout(Duration::from_secs(10))
    .build()
    .expect("Failed to build OpenAI reqwest client");
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`, `replicate.rs`, `elevenlabs.rs`, `fal.rs`, `heygen.rs`, `hedra.rs`, `kie.rs`, `midjourney.rs`

### spawn-traced-supervision-loop
Wrap long-lived background tasks in a panic-aware supervision loop. If the inner task panics, log the error, wait briefly, and restart. Tie to a `CancellationToken` for graceful shutdown.

```rust
fn spawn_traced<F, Fut>(
    desc: &'static str,
    cancel: CancellationToken,
    factory: F,
) -> JoinHandle<()>
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    tracing::info!(task = %desc, "Supervised task shutting down gracefully");
                    break;
                }
                result = std::panic::AssertUnwindSafe(factory()).catch_unwind() => {
                    if result.is_ok() {
                        tracing::info!(task = %desc, "Supervised task completed normally");
                        break;
                    }
                    tracing::error!(task = %desc, "Supervised task panicked; restarting in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    })
}
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### payment-param-validation
Payment callback handlers must reject missing parameters with explicit validation errors instead of defaulting to empty strings or zero values. Malformed callbacks should fail fast at the edge.

```rust
let transaction_id = params["transaction_hash"].as_str()
    .ok_or_else(|| AppError::Validation("Missing transaction_hash".into()))?;
let amount = params["amount"].as_f64()
    .ok_or_else(|| AppError::Validation("Missing amount".into()))?;
```

**Reference:** `rings/SILVER-RING-PY00/src/robokassa.rs`, `x402.rs`, `ton.rs`, `telegram_stars.rs`

### hmac-map-err-not-unwrap
HMAC key initialization must use `map_err()` instead of `unwrap()`. Even when the key is internally generated, a panic path in payment-critical code is unacceptable.

```rust
let mut mac = HmacSha256::new_from_slice(data.as_bytes())
    .map_err(|e| AppError::Internal(format!("HMAC key error: {}", e)))?;
```

**Reference:** `rings/SILVER-RING-PY00/src/robokassa.rs`

### sql-injection-parameterized-queries
Never build SQL strings with `format!` or string concatenation when user input is involved. Always use `$N` placeholders with bound values. For dynamic `IN` clauses, generate the placeholder list (`$1, $2, ...`) dynamically and append corresponding values to the bind vector.

```rust
let placeholders: Vec<String> = (1..=job_types.len())
    .map(|i| format!("${}", i))
    .collect();
let sql = format!("... IN ({}) ...", placeholders.join(", "));
let values: Vec<Value> = job_types.iter().map(|t| Value::String(Some(Box::new(t.to_string())))).collect();
```

**Reference:** `rings/SILVER-RING-JB00/src/queue.rs`

### payment-signature-verification
Every payment callback handler must verify cryptographic signatures before mutating any state. Reconstruct the expected signature using the provider's secret/key and compare in constant time. Reject mismatches before parsing amounts or IDs.

```rust
fn verify_callback_signature(&self, amount: &str, inv_id: &str, signature_value: &str) -> Result<(), AppError> {
    let expected = compute_hmac(amount, inv_id, &self.password2);
    if expected.len() != signature_value.len() {
        return Err(AppError::Validation("signature mismatch".into()));
    }
    let mut diff = 0u8;
    for (a, b) in expected.bytes().zip(signature_value.bytes()) {
        diff |= a ^ b;
    }
    if diff != 0 {
        return Err(AppError::Validation("signature mismatch".into()));
    }
    Ok(())
}
```

**Reference:** `rings/SILVER-RING-PY00/src/robokassa.rs`

### payment-idempotency-guard
Before completing a payment, load the existing transaction. If status is already `Completed`, return success immediately without modifying balance. Log as a security event.

```rust
match db.get_transaction(tx_id).await? {
    Some(tx) if tx.status == PaymentStatus::Completed => {
        tracing::info!(tx_id = %tx_id, "Payment already completed; skipping");
        return Ok(());
    }
    _ => {}
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/payment_webhooks.rs`, `rings/SILVER-RING-PY00/src/services/payment_processor.rs`

### atomic-balance-update
Replace SELECT-then-UPDATE balance checks with a single atomic `UPDATE` that includes the guard condition in the `WHERE` clause. Check `rows_affected()` to determine success.

```rust
let result = sqlx::query(
    "UPDATE users SET balance = balance - $1, updated_at = NOW() WHERE telegram_id = $2 AND balance >= $1"
)
.bind(amount)
.bind(telegram_id)
.execute(pool)
.await?;
Ok(result.rows_affected() > 0)
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### atomic-deduct-balance-result-pattern
Balance deduction must return a `Result` that communicates the remaining balance on success or a user-facing error message on failure. This eliminates the TOCTOU race between `check_balance` and `deduct_balance`.

```rust
pub async fn deduct_balance(
    db: &Arc<dyn Database>,
    telegram_id: i64,
    cost: f64,
    lang: Language,
) -> Result<f64, String> {
    match db.deduct_balance(telegram_id, cost).await {
        Ok(true) => Ok(0.0),
        Ok(false) => {
            let balance = db.get_balance(telegram_id).await.unwrap_or(0.0);
            let msg = if lang.is_russian() {
                format!("❌ Недостаточно средств.\n\nТребуется: {:.0} ⭐\nВаш баланс: {:.1} ⭐", cost, balance)
            } else {
                format!("❌ Insufficient funds.\n\nRequired: {:.0} ⭐\nYour balance: {:.1} ⭐", cost, balance)
            };
            Err(msg)
        }
        Err(e) => {
            tracing::error!(error = %e, "DB error during balance deduction");
            let msg = if lang.is_russian() {
                "❌ Ошибка списания средств. Попробуйте позже.".to_string()
            } else {
                "❌ Failed to deduct balance. Please try again later.".to_string()
            };
            Err(msg)
        }
    }
}
```

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`

### webhook-constant-time-secret
Webhook handlers must verify a shared secret header using constant-time comparison. Length mismatch is rejected before the constant-time loop.

```rust
fn verify_webhook_secret(headers: &HeaderMap, env_var: &str) -> Result<(), (StatusCode, String)> {
    let expected = std::env::var(env_var)?;
    let provided = headers.get("X-Webhook-Secret").and_then(|h| h.to_str().ok())?;
    if expected.len() != provided.len() {
        return Err((StatusCode::UNAUTHORIZED, "Invalid webhook secret".into()));
    }
    let mut diff = 0u8;
    for (a, b) in expected.bytes().zip(provided.bytes()) { diff |= a ^ b; }
    if diff != 0 { return Err((StatusCode::UNAUTHORIZED, "Invalid webhook secret".into())); }
    Ok(())
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### worker-queue-io-timeout
All queue I/O in background workers must have a hard timeout. On timeout, log and `continue` the loop so the worker survives transient stalls.

```rust
const QUEUE_IO_TIMEOUT: Duration = Duration::from_secs(30);
match tokio::time::timeout(QUEUE_IO_TIMEOUT, queue.dequeue()).await {
    Ok(Ok(Some(job))) => job,
    Ok(Ok(None)) => { tokio::time::sleep(poll_interval).await; continue; }
    Ok(Err(e)) => { tracing::error!(error = %e, "Queue dequeue error"); continue; }
    Err(_) => { tracing::error!("Queue dequeue timed out"); continue; }
}
```

**Reference:** `rings/SILVER-RING-JB00/src/worker.rs`

### tower-governor-rate-limit
Add per-IP rate limiting via `tower_governor` on public routers. Apply the layer after CORS but before body limit.

```rust
fn rate_limit_layer(per_second: u64, burst_size: u32) -> GovernorLayer<Arc<Governor>, ConnectInfo<SocketAddr>> {
    let config = GovernorConfigBuilder::default()
        .per_second(per_second)
        .burst_size(burst_size)
        .finish()
        .expect("rate limit config is valid");
    GovernorLayer { config: Arc::new(config) }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### webhook-result-url-validation
Webhook handlers that store provider-supplied URLs must validate them before DB persistence. Reject non-HTTP schemes, private IPs, `localhost`, and link-local addresses.

```rust
fn validate_result_url(url: &str) -> Result<(), String> {
    if url.is_empty() { return Err("empty".into()); }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("invalid scheme".into());
    }
    let lower = url.to_lowercase();
    if lower.contains("127.") || lower.contains("10.") || lower.contains("192.168.")
        || lower.contains("localhost") || lower.contains("file://") {
        return Err("private or unsupported address".into());
    }
    Ok(())
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### sql-interval-param-fix
PostgreSQL does **not** substitute `$N` placeholders inside string literals. `INTERVAL '$1 seconds'` is invalid — use expression-based intervals instead.

```rust
// WRONG:  AND started_at < NOW() - INTERVAL '$1 seconds'
// RIGHT:  AND started_at < NOW() - INTERVAL '1 second' * $1
```

**Reference:** `rings/SILVER-RING-JB00/src/queue.rs`

### enum-mapping-fail-closed
Functions that map DB string columns to Rust enums must return `Result` / `Err` for unrecognized values. Never default to a "safe" variant — it hides data corruption and injection.

```rust
fn str_to_generation_status(s: &str) -> Result<GenerationStatus, AppError> {
    match s {
        "queued" => Ok(GenerationStatus::Queued),
        "completed" => Ok(GenerationStatus::Completed),
        _ => Err(AppError::Db(DbError::Query(format!("Unknown status: {}", s)))),
    }
}
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### generation-id-in-job-payload
When a generation is created before enqueueing a job, include the generation UUID in the job payload so the worker can update the correct row. Never use `job.id` (the queue UUID) as a surrogate for the generation UUID.

```rust
let gen = db.create_generation(&request).await?;
request.params = serde_json::json!({
    "cost": params.cost,
    "generation_id": gen.id.to_string(),
});
```

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`, `rings/BRONZE-RING-APP/src/main.rs`

### payment-webhook-external-id-lookup
External payment gateways use their own identifiers (`InvId`, `session_id`, etc.). Look up transactions by the stored `external_id` rather than parsing the gateway ID as an internal UUID.

```rust
let tx = match db.get_transaction_by_external_id(external_id).await? {
    Some(tx) => tx,
    None => return "ERROR: transaction not found".to_string(),
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/payment_webhooks.rs`, `rings/SILVER-RING-DB00/src/repository.rs`

### appconfig-debug-redaction
Implement `Debug` manually for structs that contain secrets. Deriving `Debug` leaks credentials in logs, core dumps, and error traces.

```rust
impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("database_url", &"[REDACTED]")
            .field("infisical_client_secret", &"[REDACTED]")
            // ... other fields ...
            .finish()
    }
}
```

**Reference:** `rings/GOLD-RING-TY00/src/config.rs`

### cors-deny-all-fallback
When `FRONTEND_URL` is unset or all configured origins fail parsing, deny cross-origin requests instead of falling back to `allow_origin(Any)`.

```rust
let cors = if origins.is_empty() {
    tracing::warn!("No valid CORS origins; denying cross-origin requests");
    CorsLayer::new().allow_methods([Method::GET, Method::POST])
} else {
    CorsLayer::new().allow_origin(origins)...
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### worker-panic-supervision
Wrap each worker poll cycle in a nested `tokio::spawn` and await the `JoinHandle`. On `Err(e)` (panic), log and restart after a brief delay.

```rust
let handle = tokio::spawn(async move {
    poll_and_execute(&q, &types, &h, timeout, &name).await
});
match handle.await {
    Ok(Err(e)) => { tracing::error!(error = %e, "Worker error"); }
    Err(e) => { tracing::error!(error = %e, "Worker panicked; restarting in 5s"); }
    Ok(Ok(())) => {}
}
```

**Reference:** `rings/SILVER-RING-JB00/src/worker.rs`

### reqwest-redirect-none
Every `reqwest::Client` used for external provider calls must disable automatic redirects to prevent SSRF bypass via HTTP 302.

```rust
reqwest::Client::builder()
    .timeout(Duration::from_secs(60))
    .connect_timeout(Duration::from_secs(10))
    .redirect(reqwest::redirect::Policy::none())
    .build()
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`, `replicate.rs`, `fal.rs`, `elevenlabs.rs`, `heygen.rs`, `hedra.rs`, `kie.rs`, `midjourney.rs`

### serde-to-string-propagate
Never `unwrap_or_default()` on `serde_json::to_string` for DB column values. Propagate the error to prevent silent empty-string corruption.

```rust
active.status = Set(serde_json::to_string(&status).map_err(|e| {
    AppError::Internal(format!("Serialize payment status: {}", e))
})?);
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

---

## Cooperation Variants Template

At the end of every wave, propose three variants for the next loop:

1. **Deep Defence** — close deferred MEDIUM/LOW findings, add property-based tests.
2. **Observability First** — add Prometheus metrics for every new defence mechanism; build dashboards.
3. **Penetration Simulation** — write a load-test / chaos binary to validate defences under realistic failure modes.

The user may reply with the preferred variant (A, B, or C) or a custom direction.

### deduct-balance-negative-guard
Financial deduction primitives must reject non-positive amounts **before** any DB interaction. A negative amount transforms `balance - (-100)` into `balance + 100` (inflation). Apply the same guard symmetrically to `add_balance`.

```rust
if amount <= 0.0 {
    return Err(AppError::Validation(format!("Deduction amount must be positive: {}", amount)));
}
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`deduct_balance`)

### terminal-state-webhook-guard
Webhook handlers that update entity status must load the current DB row before writing. If the status is already terminal (`Completed`, `Failed`, `Cancelled`), return `200 OK` immediately and log the skip. This prevents replay/delayed webhooks from overwriting terminal states.

```rust
fn is_terminal_status(s: GenerationStatus) -> bool {
    matches!(s, GenerationStatus::Completed | GenerationStatus::Failed | GenerationStatus::Cancelled)
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs` (`replicate_webhook`, `kie_ai_webhook`)

### serde-json-deser-fail-closed
Never use `serde_json::from_str(...).unwrap_or(DefaultValue)` for enum fields in DB models. Corrupt JSON silently normalizes to the default. Return `AppError::Internal` with the corruption context so operators are alerted.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`get_transaction`, `get_transaction_by_external_id`)

### prompt-length-cap
Any user-controlled text stored in the DB (`prompts`, `logs`, `notes`) must have a hard length cap validated at ingestion. Set the limit at a crate-global constant and enforce it in the repository method, not the handler, to prevent leakage through internal call sites.

```rust
const MAX_PROMPT_LEN: usize = 2000;
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`save_prompt`)

### language-code-fail-closed
Enum mapping from DB string columns must return `Result`, not silently fall back to a default variant. Unknown language codes indicate data corruption or injection.

```rust
Language::from_code(&m.language).ok_or_else(|| {
    AppError::Db(format!("Unknown language code '{}' for user {}", m.language, m.telegram_id))
})?
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs` (`get_user_by_telegram_id`)

### result-url-credential-guard
URL validation for webhook result URLs must reject `@` (credential embedding), `%` (encoding bypasses), and impose a `MAX_URL_LEN` (e.g., 4096 bytes). Combine with existing private-IP and scheme checks for layered defense.

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs` (`validate_result_url`)

### config-debug-full-redaction
The manual `Debug` impl for configuration structs must redact **all** credential-adjacent fields: secrets, client IDs, API keys, database URLs, and any list of sensitive identifiers (admin/staff IDs).

**Reference:** `rings/GOLD-RING-TY00/src/config.rs`

### ok-none-vs-err-distinction
Repository methods returning `Option<T>` must be handled with separate branches for `Ok(None)` (not found) and `Err` (transient failure). Blindly falling through to `create_user` after any error risks unique-key violations or invisible outages.

```rust
match db.get_user_by_telegram_id(tid).await {
    Ok(Some(user)) => { /* serve existing user */ }
    Ok(None) => { /* create user */ }
    Err(e) => {
        tracing::error!(error = %e, "DB lookup failed");
        return bot.send_message(msg.chat.id, localize!("db_error")).await;
    }
}
```

**Reference:** `rings/SILVER-RING-SN00/src/start.rs`


### atomic-cte-payment-credit
Eliminate double-spend races in payment callbacks by collapsing the status update + balance credit into a single PostgreSQL CTE. The inner UPDATE ... WHERE status <> "completed" RETURNING id acts as a lightweight compare-and-swap; the outer UPDATE users runs only if the CTE returned a row.

```sql
WITH updated_tx AS (
    UPDATE payments_v2
    SET status = $4, updated_at = NOW()
    WHERE id = $1 AND status <> $4
    RETURNING id
)
UPDATE users
SET balance = balance + $2, updated_at = NOW()
WHERE telegram_id = $3
  AND EXISTS (SELECT 1 FROM updated_tx)
```

Returns rows_affected() > 0 to distinguish first credit (true) from idempotent replay (false).

**Reference:** rings/SILVER-RING-DB00/src/repository.rs (complete_robokassa_payment)

### webhook-db-timeout
Every DB call inside an HTTP webhook handler must have a deadline shorter than the provider retry timeout. Wrap each call in tokio::time::timeout(Duration::from_secs(10), ...). On Elapsed, return 503 Service Unavailable so the provider backs off rather than retrying immediately.

**Reference:** rings/BRONZE-RING-SRV/src/webhooks.rs, rings/BRONZE-RING-SRV/src/payment_webhooks.rs

### rate-limit-route-isolation
Never share a single rate-limit bucket across traffic classes. Create separate GovernorLayer instances for health, webhooks, and payments, and apply them via .route_layer() on sub-routers before merging.

**Reference:** rings/BRONZE-RING-SRV/src/router.rs

### callback-form-debug-redaction
Payment-gateway callback structs containing signatures or secrets must implement manual std::fmt::Debug that masks sensitive fields. #[derive(Debug)] on a webhook form leaks credentials into any span or log line that captures the struct.

**Reference:** rings/BRONZE-RING-SRV/src/payment_webhooks.rs (RobokassaCallbackForm)

### owned-repository-methods
Close IDOR windows by adding *_owned variants of retrieval/mutation methods that accept the callers telegram_id and add AND telegram_id = $N to the SQL WHERE clause. The guard belongs at the repository boundary, not the handler, to prevent bypass through internal call sites.

**Reference:** rings/GOLD-RING-TR00/src/database.rs, rings/SILVER-RING-DB00/src/repository.rs (get_generation_owned, update_generation_status_owned)

### worker-tracing-instrument
Background worker functions (poll_and_execute, run_retry_maintenance, drain_cache) must carry #[tracing::instrument] with explicit skip lists for dyn Trait parameters that do not implement Debug. Fields should include identifiers (worker name, interval) needed for production troubleshooting.

**Reference:** rings/SILVER-RING-JB00/src/worker.rs

### spawn-traced-exponential-backoff
A panic-aware `spawn_traced` supervisor must use capped exponential backoff (5s → 60s) and an absolute `MAX_CONSECUTIVE_FAILURES` (e.g., 10) before giving up. Count resets on normal completion. Never restart panicked tasks with a fixed interval — it spams logs and CPU.

```rust
fn spawn_traced<F, Fut>(desc: &str, cancel: CancellationToken, factory: F) -> JoinHandle<()>
where F: Fn() -> Fut + Send + 'static, Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        let mut backoff = 5u64;
        let mut failures = 0u32;
        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => { break; }
                _ = std::panic::AssertUnwindSafe(factory()).catch_unwind() => {
                    failures += 1;
                    if failures >= 10 { tracing::error!("Giving up"); break; }
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    backoff = std::cmp::min(backoff * 2, 60);
                }
            }
        }
    })
}
```

**Reference:** rings/BRONZE-RING-APP/src/main.rs

### bot-dispatcher-clone
Long-lived listeners that need to restart after errors require the dispatcher builder (or the dispatcher itself) to be `Clone`. Derive `Clone` on the builder struct so the `spawn_traced` factory can reconstruct the listener after each panic or error.

```rust
#[derive(Clone)]
pub struct BotDispatcher { /* Arc<...> fields */ }
```

**Reference:** rings/SILVER-RING-TG00/src/dispatcher.rs

### webhook-idempotency-raw-sql
When adding an idempotency table for webhooks, use a composite key `(provider, event_id)` and `INSERT ... ON CONFLICT DO NOTHING` in raw SQL. Return `true` if `rows_affected() > 0`, `false` on duplicate. This avoids the `last_insert_id` issues of composite-key ORM inserts.

```rust
let result = sqlx::query(
    "INSERT INTO webhook_events (provider, event_id, processed_at)
     VALUES ($1, $2, NOW())
     ON CONFLICT (provider, event_id) DO NOTHING"
)
.bind(provider).bind(event_id).execute(&self.pool).await?;
Ok(result.rows_affected() > 0)
```

**Reference:** rings/SILVER-RING-DB00/src/repository.rs

### f64-is-finite-balance-guard
Balance primitives (`deduct_balance`, `add_balance`, `complete_payment`) must guard against `NaN` and `Infinity` with `is_finite()` in addition to range checks. `NaN` bypasses all `<`/`>` comparisons and corrupts the DB permanently.

```rust
if !amount.is_finite() || amount <= 0.0 {
    return Err(AppError::Validation(format!("Invalid amount: {}", amount)));
}
```

**Reference:** rings/SILVER-RING-DB00/src/repository.rs

### truncate-for-log-utf8-safe
Log truncation must be Unicode-safe. Count Unicode scalar values (chars), not bytes, using `char_indices().nth(max)`. Never slice string indices directly — it can produce invalid UTF-8.

```rust
pub fn truncate_for_log(s: &str, max: usize) -> &str {
    if s.chars().count() <= max { s }
    else if let Some((idx, _)) = s.char_indices().nth(max) { &s[..idx] }
    else { "" }
}
```

**Reference:** rings/GOLD-RING-TY00/src/utils.rs

### ai-response-content-length-cap
Before calling `resp.bytes()` on external AI provider responses, check `resp.content_length()`. If it exceeds a hard cap (e.g., 50 MiB), return `InvalidResponse` before allocating memory. This prevents OOM from malicious or misbehaving providers.

```rust
const MAX_RESPONSE_BYTES: u64 = 50 * 1024 * 1024;
if let Some(cl) = resp.content_length() {
    if cl > MAX_RESPONSE_BYTES {
        return Err(AiError::InvalidResponse {
            provider: "openai".into(),
            message: format!("Response too large: {} bytes", cl),
        });
    }
}
```

**Reference:** rings/SILVER-RING-AI00/src/providers/openai.rs, rings/SILVER-RING-AI00/src/providers/elevenlabs.rs

### job-worker-owned-repository-calls
Workers that update generation status on behalf of a user must call `update_generation_status_owned(id, telegram_id, ...)` instead of the unguarded `update_generation_status`. Wire the user's `telegram_id` from the job payload so the repository enforces `AND telegram_id = $N`.

**Reference:** rings/BRONZE-RING-APP/src/main.rs (`handle_generation_job`)

### reqwest-build-result-not-expect
AI provider constructors that build a `reqwest::Client` must return `Result<Self, AppError>` instead of panicking with `.expect()`. A TLS misconfiguration or invalid proxy should degrade gracefully (omit the provider from the pool) rather than crashing the process.

```rust
pub fn new(api_key: &str) -> Result<Self, AppError> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build OpenAI reqwest client: {}", e)))?;
    Ok(Self { /* ... */ })
}
```

**Reference:** rings/SILVER-RING-AI00/src/providers/openai.rs, replicate.rs, fal.rs, kie.rs, elevenlabs.rs, heygen.rs, hedra.rs, midjourney.rs

### url-parse-ssrf-validation
Never validate URLs with string heuristics (`starts_with`, `contains`). Use `url::Url::parse` and inspect the parsed host. Reject loopback, private IPv4, link-local, and IPv6 ULA addresses via `std::net::IpAddr` checks. Also reject embedded credentials (`username()`, `password()`).

```rust
let parsed = url::Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;
match parsed.scheme() { "http" | "https" => {} _ => return Err("bad scheme".into()) }
if !parsed.username().is_empty() || parsed.password().is_some() {
    return Err("embedded credentials".into());
}
if let Some(host) = parsed.host_str() {
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        if ip.is_loopback() { return Err("loopback".into()); }
        // match V4/V6 for is_private / is_link_local / ULA ...
    }
}
```

**Reference:** rings/BRONZE-RING-SRV/src/webhooks.rs (`validate_result_url`)

### tracing-instrument-async-handlers
Every `pub async fn` that serves user requests or processes background jobs must carry `#[tracing::instrument]` so spans propagate across await points. Use `skip_all` when parameters include types without `Debug` (dyn traits, `teloxide::Bot`, broadcast senders).

```rust
#[tracing::instrument(skip_all, fields(job_id = %job.id))]
async fn handle_generation_job(...) -> Result<(), AppError> { ... }
```

**Reference:** rings/BRONZE-RING-APP/src/main.rs, rings/SILVER-RING-SN00/src/*.rs

### input-length-cap-dialogue-state
Any user-controlled text stored in dialogue state or forwarded to downstream providers must have a hard length cap validated at ingestion. Set the limit at a crate-global constant and enforce it in the handler before assignment.

```rust
const MAX_DIALOGUE_TEXT_LEN: usize = 2000;
if text.len() > MAX_DIALOGUE_TEXT_LEN {
    bot.send_message(msg.chat.id, localize!("text_too_long")).await?;
    return Ok(());
}
state.prompt = Some(text.to_string());
```

**Reference:** rings/SILVER-RING-SN00/src/avatar_transform.rs, ai_photoshop.rs, ai_reels.rs, fal_render.rs, hedra_render.rs, improve_prompt.rs, image_to_video.rs, neuro_photo.rs, text_to_speech.rs, heygen_render.rs, avatar_brain.rs

### rate-limit-config-option
`GovernorConfigBuilder::finish()` returns `Option`; never `.expect()` it. Return `Option<GovernorLayer>` and conditionally apply the layer in the router builder. On `None`, log an error and continue without rate limiting.

```rust
fn rate_limit_layer(...) -> Option<GovernorLayer<...>> {
    let config = GovernorConfigBuilder::default()...finish()?;
    Some(GovernorLayer { config: Arc::new(config) })
}
```

**Reference:** rings/BRONZE-RING-SRV/src/router.rs

### db-boundary-string-length-caps
Every DB method that accepts a user-provided string must validate `len() <= MAX_*` before constructing the `ActiveModel`. Do not rely on upstream callers.

```rust
const MAX_USERNAME_LEN: usize = 32;
if let Some(u) = username {
    if u.len() > MAX_USERNAME_LEN {
        return Err(AppError::Validation(format!("username exceeds max length of {}", MAX_USERNAME_LEN)));
    }
}
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### handler-input-sanitization
Normalize user input (trim whitespace, reject control chars, cap length) in the handler before passing to DB.

```rust
let username = username.and_then(|s| {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_USERNAME_LEN { return None; }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) { return None; }
    Some(trimmed.to_string())
});
```

**Reference:** `rings/SILVER-RING-SN00/src/start.rs`

### refund-amount-re-validation
Before executing a refund, re-validate `tx.amount.is_finite() && tx.amount > 0.0` even if the amount was validated on creation.

```rust
if !tx.amount.is_finite() || tx.amount <= 0.0 {
    return Err(AppError::Validation(format!("refund amount must be finite and > 0: {}", tx.amount)));
}
```

**Reference:** `rings/SILVER-RING-PY00/src/services/payment_processor.rs`

### environment-config-fail-fast
Parse `PORT`, comma-separated ID lists, and other environment variables with explicit error paths. Return `AppError::Config` with the invalid token, rather than silently defaulting.

```rust
http_port: match std::env::var("PORT") {
    Ok(v) => v.parse::<u16>()
        .map_err(|_| AppError::Config(format!("PORT '{}' is not a valid port number", v)))?,
    Err(_) => 3000,
},
```

**Reference:** `rings/GOLD-RING-TY00/src/config.rs`

### module-level-constants-for-defense-in-depth
Define length-limit constants at the module level (outside `impl` blocks) so they are accessible by bare name in both trait and inherent impls.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### money-integer-type-checked-arithmetic
Replace `f64` monetary fields with an `i64` minor-unit newtype (`Money`) that provides `checked_add`, `checked_sub`, `checked_mul`, `checked_div`. `from_f64` rejects `NaN`, `Inf`, negatives, and overflow. `Debug` redacts the raw value. Serialize as `"123.45"` string in JSON to avoid float drift.

```rust
pub struct Money(i64);
impl Money {
    pub fn from_f64(v: f64) -> Option<Self> {
        if !v.is_finite() || v < 0.0 { return None; }
        let scaled = (v * 100.0).round();
        if scaled > i64::MAX as f64 { return None; }
        Some(Self(scaled as i64))
    }
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }
}
```

**Reference:** `rings/GOLD-RING-TY00/src/money.rs`

### money-internal-chokepoint-integration
Rather than migrating every trait signature at once, convert `f64` to `Money` at the highest-risk internal chokepoint (`PaymentProcessor`) first. Gateways and DB traits continue accepting `f64` at boundaries until a dedicated migration wave.

**Reference:** `rings/SILVER-RING-PY00/src/services/payment_processor.rs`

### serde-deny-unknown-fields-inbound
Add `#[serde(deny_unknown_fields)]` to all inbound deserialization structs (callbacks, webhooks, API payloads) unless they use `flatten`. This prevents silent field truncation and variant mis-match attacks on untagged enums.

**Reference:** `rings/GOLD-RING-PR00/src/telegram.rs` (`CallbackData`)
**Reference:** `rings/GOLD-RING-PR00/src/replicate.rs` (`WebhookPayload`)
**Reference:** `rings/GOLD-RING-PR00/src/kie.rs` (`WebhookPayload`, `WebhookResponse`)

### tracing-instrument-financial-methods
All public `PaymentProcessor` async methods must carry `#[tracing::instrument]` with sanitized fields (`transaction_id`, `telegram_id`, `method`), ensuring payment flows are observable in production traces without leaking secrets.

**Reference:** `rings/SILVER-RING-PY00/src/services/payment_processor.rs`

### gateway-float-to-integer-overflow-guard
When scaling a monetary `f64` to minor units (e.g. `amount * 1_000_000.0`), clamp the `amount` to a safe ceiling **before** multiplication. `f64` overflow to `Inf` followed by `as u64` silently produces `u64::MAX` (or 0), corrupting the value sent to the provider.

```rust
const MAX_X402_AMOUNT: f64 = 1_000_000_000.0;
if amount > MAX_X402_AMOUNT {
    return Err(AppError::Validation(...));
}
let scaled = (amount * 1_000_000.0) as u64;
```

**Reference:** `rings/SILVER-RING-PY00/src/x402.rs`

### deterministic-hmac-float-formatting
Never use default `Display` (`{}`) for `f64` inside HMAC payloads or signed URLs. Format with a fixed precision (`{:.2}`) so that `1.5` always produces `"1.50"`, preventing signature mismatches caused by inconsistent trailing zeros or scientific notation.

```rust
let amount_fmt = format!("{:.2}", amount);
let data = format!("{}:{}:{}:{}", login, amount_fmt, inv_id, secret);
```

**Reference:** `rings/SILVER-RING-PY00/src/robokassa.rs`

### supervisor-restart-rate-warning
Track the number of restarts within a sliding window (e.g. 60 s). If a supervised background task restarts ≥3 times within the window, emit a `tracing::warn!` so operators know the root cause (DB outage, bad config) is still unaddressed.

```rust
if now.duration_since(last).as_secs() < 60 {
    restarts_in_window += 1;
    if restarts_in_window >= 3 {
        tracing::warn!(task = %desc, "Restarting rapidly; root cause not resolved");
    }
}
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### infisical-store-reqwest-hardening
Secret-store constructors that build an internal `reqwest::Client` must set explicit timeouts and disable redirects. `Client::new()` hangs indefinitely on slow upstreams and follows redirects by default, creating SSRF risk if the upstream domain is compromised.

```rust
let http = Client::builder()
    .timeout(Duration::from_secs(30))
    .connect_timeout(Duration::from_secs(10))
    .redirect(reqwest::redirect::Policy::none())
    .build()
    .map_err(|e| AppError::Internal(format!("Failed to build Infisical reqwest client: {}", e)))?;
```

**Reference:** `rings/SILVER-RING-SC00/src/store.rs`

### update-limit-subquery
Maintenance UPDATE queries that select rows by status or age must use a sub-select with `LIMIT` to prevent table-wide locks during large outages.

```sql
UPDATE job_queue
SET status = 'queued', attempts = 0, started_at = NULL, updated_at = NOW()
WHERE id IN (
    SELECT id FROM job_queue
    WHERE status = 'running'
      AND started_at < NOW() - INTERVAL '1 second' * $1
      AND attempts < max_attempts
    LIMIT $2
)
```

**Reference:** `rings/SILVER-RING-JB00/src/queue.rs`

### unwrap-or-db-error-elimination
Never use `db.get_balance(tid).await.unwrap_or(0.0)`. A DB error (connection failure, timeout) silently appears as "balance: 0" to the user and the operator sees nothing. Use explicit `match`, log the error, and return a localized "service temporarily unavailable" message.

```rust
match db.get_balance(tid).await {
    Ok(balance) => balance,
    Err(e) => {
        tracing::error!(telegram_id = tid, "balance read failed: {}", e);
        return Err(AppError::Internal("Service temporarily unavailable".into()));
    }
}
```

**Reference:** `rings/SILVER-RING-SN00/src/balance.rs`, `rings/SILVER-RING-SN00/src/handlers.rs`, `rings/SILVER-RING-SN00/src/generation_utils.rs`

### infisical-response-deny-unknown-fields
Add `#[serde(deny_unknown_fields)]` to structs that deserialize external API responses (e.g., Infisical secret list). If the upstream adds unexpected fields, deserialization fails loudly instead of silently ignoring them, preventing logic errors from missing newly required fields.

**Reference:** `rings/GOLD-RING-PR00/src/infisical.rs`

### gateway-overflow-clamp-before-scale
When scaling a monetary `f64` to integer nano-units, clamp `amount` to a hard ceiling **before** multiplication. Apply the same ceiling pattern consistently across every gateway (x402, TON, Robokassa, TelegramStars) to prevent `Inf as u64` corruption.

```rust
const MAX_TON_AMOUNT: f64 = 1_000_000_000.0;
if amount > MAX_TON_AMOUNT {
    return Err(AppError::Validation(format!(
        "TON amount exceeds maximum of {}: {}", MAX_TON_AMOUNT, amount
    )));
}
let nanoton = (amount * 1_000_000_000.0) as u64;
```

**Reference:** `rings/SILVER-RING-PY00/src/ton.rs`

### retry-stuck-preserve-attempts
Maintenance UPDATE queries that retry stuck jobs must **not** reset `attempts = 0`. Preserve the count so `max_attempts` is eventually reached and the job can transition to `failed`.

**How to apply:**
```rust
// WRONG: SET status = 'queued', attempts = 0, ...
// RIGHT: SET status = 'queued', started_at = NULL, ... (keep attempts)
```

Add a second query for exhausted jobs:
```sql
UPDATE job_queue
SET status = 'failed', error = 'Job stuck and max attempts exhausted', ...
WHERE id IN (
    SELECT id FROM job_queue
    WHERE status = 'running'
      AND started_at < NOW() - INTERVAL '1 second' * $1
      AND attempts >= max_attempts
    LIMIT $2
)
```

**Reference:** `rings/SILVER-RING-JB00/src/queue.rs`

### supervisor-catch-unwind-factory
`tokio::spawn` catches panics **inside** the spawned future, not in the closure that builds the future. A panic in `factory()` before `tokio::spawn` aborts the supervisor thread.

**How to apply:**
```rust
let fut = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| factory())) {
    Ok(f) => f,
    Err(_) => {
        tracing::error!("Factory panicked; restarting with backoff");
        tokio::time::sleep(backoff).await;
        continue;
    }
};
let mut task = tokio::spawn(fut);
```

**Reference:** `rings/SILVER-RING-JB00/src/worker.rs`

### outbound-url-validation-symmetry
URL validation applied to webhook ingress must also be applied to the outbound provider-success path. A compromised provider can return internal URLs that get persisted and served to users.

**How to apply:**
```rust
let validated_url = result_url.and_then(|url| {
    match validate_result_url(url) {
        Ok(()) => Some(url),
        Err(reason) => {
            tracing::warn!(url = %truncate_for_log(url, 128), reason, "Invalid provider result_url; rejecting");
            None
        }
    }
});
db.update_generation_status(id, status, validated_url, None).await?;
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### log-payload-redaction
Never log full `serde_json::Value` job payloads or user messages into structured logs. Log only metadata (type, length, identifiers).

**How to apply:**
```rust
// WRONG
tracing::info!(payload = %job.payload, "Job executed");

// RIGHT
let payload_len = job.payload.to_string().len();
tracing::info!(job_type, payload_len, "Job executed");
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### url-parse-not-contains
Never validate URLs with `contains("domain.com")`. Use `url::Url::parse`, enforce exact host whitelist, and reject private IPs / embedded credentials.

**How to apply:**
```rust
let parsed = url::Url::parse(url_str).map_err(|_| "Invalid URL")?;
match parsed.scheme() { "http" | "https" => {} _ => return Err("bad scheme") }
let host_ok = parsed.host_str().map_or(false, |h| {
    let lower = h.to_lowercase();
    lower == "example.com" || lower == "www.example.com"
});
if !host_ok { return Err("bad host") }
if !parsed.username().is_empty() || parsed.password().is_some() {
    return Err("embedded credentials");
}
```

**Reference:** `rings/SILVER-RING-SN00/src/instagram_scraping.rs`

### telegram-api-timeout-wrapper
Every `bot.send_message`, `bot.answer_callback_query`, and `dialogue.update` call that blocks a dispatcher worker must have a hard timeout (default 30s). If the Telegram API stalls, the handler should return a `TimedOut` error and log at `error!` level rather than hanging the worker forever.

**How to apply:**
```rust
pub async fn send_message_timeout(
    bot: &Bot,
    chat_id: ChatId,
    text: impl Into<String>,
) -> Result<Message, std::io::Error> {
    match tokio::time::timeout(Duration::from_secs(30), bot.send_message(chat_id, text)).await {
        Ok(result) => result.map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Telegram API error: {}", e))
        }),
        Err(_) => {
            tracing::error!(chat_id = %chat_id, "bot.send_message timed out");
            Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Telegram API send_message timeout"))
        }
    }
}
```

**Reference:** `rings/SILVER-RING-TG00/src/utils.rs`

### navigation-router-ttl-eviction
Per-chat in-memory state accumulators (`HashMap<i64, Vec<SceneId>>`) must evict inactive entries. Without eviction, memory grows linearly with the total number of unique chats that ever interacted.

**How to apply:**
- Track `last_accessed: HashMap<i64, Instant>`.
- Update on every read/write operation (`enter`, `go_back`).
- Run `evict_inactive(Duration::from_secs(24 * 60 * 60))` periodically (e.g., every N operations or from a background interval).

**Reference:** `rings/SILVER-RING-TG00/src/navigation.rs`

### stub-honesty
A no-op handler that sends a success message misleads users and creates false operational signals. A stub must be honest: send a clear "not supported" message, log the limitation, and return an error upstream so callers know no work was performed.

**Reference:** `rings/SILVER-RING-SN00/src/cancel_predictions.rs`

### unused-dangerous-field-removal
Any schema field that is a bypass flag, override, or privilege escalation toggle must be removed if it is unused. Unused dangerous fields are accident-prone: a future developer may wire them up without security review.

**Reference:** `rings/GOLD-RING-PR00/src/payment.rs`

### env-var-admin-identity
Never hardcode admin, staff, or super-user identifiers in source code. Load them from environment variables via `LazyLock` and fail fast (panic) if mandatory IDs are missing.

**How to apply:**
```rust
use std::sync::LazyLock;

pub static SUPER_ADMIN_ID: LazyLock<i64> = LazyLock::new(|| {
    let raw = std::env::var("SUPER_ADMIN_ID")
        .expect("FATAL: SUPER_ADMIN_ID is required");
    raw.parse::<i64>()
        .expect("FATAL: SUPER_ADMIN_ID must be a valid i64")
});
```

**Reference:** `rings/SILVER-RING-TG00/src/access.rs`

### log-only-metadata-never-content
Never log attacker-controlled or user-provided text content into structured logs. Log only metadata: length, identifier, timestamp, type.

**How to apply:**
```rust
// WRONG
tracing::info!(message = %truncate_for_log(text, 200), "Support request");

// RIGHT
tracing::info!(message_len = text.len(), telegram_id, "Support request received");
```

**Reference:** `rings/SILVER-RING-SN00/src/tech_support.rs`

### email-validation-rfc-like
Never validate email with naive `contains('@') && contains('.')`. Use length limits, exact `@` count, non-empty parts, and domain-dot rules.

**How to apply:**
```rust
fn validate_email(email: &str) -> bool {
    if email.len() > 254 { return false; }
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 { return false; }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || local.len() > 64 { return false; }
    if !domain.contains('.') { return false; }
    if local.starts_with('.') || local.ends_with('.') || domain.starts_with('.') || domain.ends_with('.') { return false; }
    if email.contains("..") { return false; }
    true
}
```

**Reference:** `rings/SILVER-RING-SN00/src/email.rs`

### rate-limit-fail-closed
Rate-limit configuration errors must be fatal at startup. Continuing without rate limiting is a silent security bypass.

**How to apply:**
```rust
match build_rate_limit_layer() {
    Some(layer) => router.layer(layer),
    None => panic!(
        "FATAL: Failed to build rate-limit layer. \
         Misconfigured rate limiting is a security risk. Aborting startup."
    ),
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### webhook-secrets-startup-loading
Load webhook secrets into `AppState` at server startup instead of reading `std::env::var` per request. This eliminates race conditions, reduces latency, and prevents env-var-not-set errors from leaking in per-request responses.

**How to apply:**
```rust
pub struct AppState {
    webhook_secrets: HashMap<String, String>,
    // ...
}

fn load_webhook_secret(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => {
            tracing::error!("{} is not set; webhooks will reject requests", name);
            None
        }
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`, `rings/BRONZE-RING-SRV/src/webhooks.rs`

### per-key-ttl-cache
Replace global cache freshness with per-entry `(value, Instant)` tuples. Callers only reload secrets for keys that are actually stale, reducing upstream load and improving freshness.

**How to apply:**
```rust
struct SecretCache {
    secrets: HashMap<String, (String, Instant)>,
}

fn is_entry_fresh(&self, key: &str) -> bool {
    match self.secrets.get(key) {
        Some((_, loaded)) => loaded.elapsed() < SECRET_CACHE_TTL,
        None => false,
    }
}
```

**Reference:** `rings/SILVER-RING-SC00/src/store.rs`

### cache-size-cap-eviction
In-memory caches must have a hard `MAX_ENTRIES` ceiling. When exceeded, evict the oldest entry by insertion timestamp. Emit a `tracing::warn!` so operators know the cache is under pressure.

**How to apply:**
```rust
const MAX_SECRET_CACHE_ENTRIES: usize = 1000;

fn insert(&mut self, key: String, value: String) {
    if self.secrets.len() >= MAX_SECRET_CACHE_ENTRIES {
        let oldest = self.secrets
            .iter()
            .min_by_key(|(_, (_, loaded))| *loaded)
            .map(|(k, _)| k.clone());
        if let Some(k) = oldest {
            self.secrets.remove(&k);
            tracing::warn!(evicted_key = %k, "Cache at capacity; evicted oldest entry");
        }
    }
    self.secrets.insert(key, (value, Instant::now()));
}
```

**Reference:** `rings/SILVER-RING-SC00/src/store.rs`

### cursor-pagination-uuid
Add `cursor: Option<Uuid>` to list endpoints. Resolve the cursor to a `created_at` timestamp, then filter `created_at < cursor_timestamp` with `order_by_desc(created_at)`. Cap `LIMIT` aggressively (e.g. 100). This prevents offset-based deep-paging DoS and gives clients reliable pagination.

**How to apply:**
```rust
async fn get_transactions_by_telegram_id(
    &self,
    telegram_id: i64,
    cursor: Option<Uuid>,
    limit: i64,
) -> Result<Vec<Transaction>, AppError> {
    let safe_limit = if limit <= 0 { 1 } else if limit > 100 { 100 } else { limit };
    let mut query = Entity::find()
        .filter(Column::TelegramId.eq(telegram_id))
        .order_by_desc(Column::CreatedAt);
    if let Some(c) = cursor {
        let cursor_row = Entity::find().filter(Column::Id.eq(c)).one(pool).await?;
        if let Some(row) = cursor_row {
            query = query.filter(Column::CreatedAt.lt(row.created_at));
        }
    }
    query.limit(Some(safe_limit as u64)).all(pool).await
}
```

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`, `rings/GOLD-RING-TR00/src/database.rs`

### hashmap-max-entry-cap
Every in-memory cache that accumulates entries by key must have a hard `MAX_ENTRIES` ceiling. When exceeded, evict the oldest entry by timestamp. Emit `tracing::warn!` so operators know the cache is under pressure.

**How to apply:**
```rust
const MAX_NAVIGATION_ENTRIES: usize = 50_000;

if self.history.len() > MAX_NAVIGATION_ENTRIES {
    if let Some((oldest_chat, _)) = self.last_accessed
        .iter()
        .min_by_key(|(_, &instant)| instant)
        .map(|(&k, &v)| (k, v))
    {
        self.history.remove(&oldest_chat);
        self.last_accessed.remove(&oldest_chat);
        tracing::warn!(
            evicted_chat = oldest_chat,
            remaining = self.history.len(),
            "NavigationRouter at capacity; evicted oldest chat"
        );
    }
}
```

**Reference:** `rings/SILVER-RING-TG00/src/navigation.rs`

### f64-is-finite-provider-param
Before inserting a `f64` value parsed from user JSON params into an external API payload, verify `is_finite() && value > 0.0` (or appropriate range). Filter invalid values to `None` rather than propagating them to downstream providers.

**How to apply:**
```rust
if let Some(d) = request.params.get("duration").and_then(|v| v.as_f64()) {
    if d.is_finite() && d > 0.0 {
        payload.insert("duration".to_string(), serde_json::json!(d));
    }
}
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/fal.rs`, `kie.rs`, `replicate.rs`, `openai.rs`

### input-length-cap-before-float-parse
Never call `text.parse::<f64>()` on attacker-controlled strings without capping `text.len()` first. A multi-megabyte numeric string can exhaust CPU during float parsing.

**How to apply:**
```rust
const MAX_PAYMENT_TEXT_LEN: usize = 32;
if text.len() > MAX_PAYMENT_TEXT_LEN {
    return Err(AppError::Validation("input too long".into()));
}
if let Ok(amount) = text.parse::<f64>() {
    // ...
}
```

**Reference:** `rings/SILVER-RING-SN00/src/payment.rs`

### deny-unknown-fields-inbound-structs
Add `#[serde(deny_unknown_fields)]` to all structs that deserialize data from external sources (payment gateways, webhooks, provider APIs). Unknown fields indicate schema mismatches or injection attempts and must fail loudly rather than being silently ignored.

**How to apply:**
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RobokassaCallback {
    pub out_sum: f64,
    pub inv_id: i64,
    pub signature_value: String,
}
```

**Reference:** `rings/GOLD-RING-PR00/src/payment.rs`

### telegram-id-sentinel-guard
Every handler that extracts `telegram_id` from `msg.from.id` or `q.from.id` must check `tid <= 0` at the earliest boundary and return early with a `tracing::warn!` before any DB interaction.

**How to apply:**
```rust
let tid = q.from.id.0 as i64;
if tid <= 0 {
    tracing::warn!("Callback query missing valid telegram_id; aborting handler");
    return Ok(());
}
let lang = load_lang_by_id(&db, tid).await;
```

**Reference:** `rings/SILVER-RING-SN00/src/handlers.rs`

### tracing-instrument-dispatcher-functions
Every `pub async fn` that serves user requests or dispatches to downstream handlers must carry `#[tracing::instrument]` so spans propagate across await points. Use `skip_all` when parameters include types without `Debug` (dyn traits, `teloxide::Bot`).

**How to apply:**
```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
async fn handle_command(
    bot: teloxide::Bot,
    db: Arc<dyn Database>,
    dialogue: MyDialogue,
    msg: Message,
    cmd: Command,
) -> HandlerResult {
    // ...
}
```

**Reference:** `rings/SILVER-RING-SN00/src/handlers.rs`

### provider-response-deny-unknown-fields
All inbound provider response structs must use `#[serde(deny_unknown_fields)]`. Provider API changes that add unexpected fields will then fail deserialization immediately, giving operators early warning and preventing downstream logic from acting on silently truncated data.

**How to apply:**
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatResponse {
    pub id: String,
    pub choices: Vec<ChatChoice>,
}
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`, `fal.rs`, `replicate.rs`, `heygen.rs`, `elevenlabs.rs`, `hedra.rs`, `kie.rs`

### callback-telegram-id-sentinel-guard
Every Telegram callback query handler must validate `telegram_id` from `q.from.id` before any DB operation. Reject non-positive values early to prevent queries against sentinel/corrupted identity data.

**How to apply:**
```rust
let tid = q.from.id.0 as i64;
if tid <= 0 {
    tracing::warn!("Callback query missing valid telegram_id; aborting handler");
    return Ok(());
}
```

**Reference:** `rings/SILVER-RING-SN00/src/select_model.rs`, `text_to_image.rs`, `text_to_video.rs`, `neuro_photo.rs`, `lip_sync.rs`, `morphing.rs`, `voice_training.rs`, `ai_cover.rs`, `ai_reels.rs`, `music_generation.rs`

### tracing-instrument-scene-handlers
Every user-facing scene entrypoint (`handle_start`, `handle_payment`, `handle_{scene}`) must carry `#[tracing::instrument(skip_all)]` so production traces capture latency and error propagation across the full handler lifecycle.

**How to apply:**
```rust
#[tracing::instrument(skip_all)]
pub async fn handle_start(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    db: Arc<dyn Database>,
) -> HandlerResult {
    // ...
}
```

**Reference:** `rings/SILVER-RING-SN00/src/start.rs`, `payment.rs`, `balance.rs`, `neuro_photo.rs`, `text_to_image.rs`, `text_to_video.rs`, `face_swap.rs`, `morphing.rs`, `image_to_video.rs`, `train_flux_model.rs`

### fail-gracefully-never-panic
Production initialization and configuration code must return structured errors (`Result`) rather than calling `panic!`. The caller (supervisor, init system, or test harness) decides whether to retry, degrade, or exit. An uncontrolled abort transforms a configuration mistake into a total service outage.

**How to apply:**
```rust
fn apply_rate_limit<S>(router: Router<S>, per_second: u64, burst_size: u32) -> Result<Router<S>, String> {
    match rate_limit_layer(per_second, burst_size) {
        Some(layer) => Ok(router.layer(layer)),
        None => {
            let msg = format!("Failed to build rate-limit layer (per_second={}, burst_size={})", per_second, burst_size);
            tracing::error!("{}", msg);
            Err(msg)
        }
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### fsm-state-loss-guard
Never use `unwrap_or_default()` on dialogue-state fields that were populated in earlier steps of a multi-step flow. After `InMemStorage` TTL expiry, these fields become `None`; `unwrap_or_default()` silently yields empty strings or empty vectors, producing invalid output that the user cannot distinguish from genuine input.

**How to apply:**
```rust
let company = match state.name.as_ref() {
    Some(n) if !n.is_empty() => n.clone(),
    _ => {
        let err = if lang.is_russian() { "❌ Сессия устарела. Начните заново." } else { "❌ Session expired. Please start again." };
        bot.send_message(msg.chat.id, err).await?;
        return return_to_menu(&bot, &dialogue, msg.chat.id, lang).await;
    }
};
```

**Reference:** `rings/SILVER-RING-SN00/src/avatar_brain.rs`, `train_flux_model.rs`, `morphing.rs`

### provider-inbound-deny-unknown-fields-core
All inbound deserialization structs in core provider modules (`GOLD-RING-PR00`) must use `#[serde(deny_unknown_fields)]`. Provider API changes that add unexpected fields will then fail deserialization immediately, giving operators early warning and preventing downstream logic from acting on silently truncated data.

**How to apply:**
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderResponse {
    pub id: String,
    pub status: String,
}
```

**Reference:** `rings/GOLD-RING-PR00/src/fal.rs`, `infisical.rs`, `kie.rs`, `openai.rs`, `payment.rs`, `providers.rs`, `replicate.rs`, `telegram.rs`

### tracing-instrument-scene-handlers-batch
Every user-facing scene entrypoint (`handle_{scene}_msg`, `handle_{scene}_callback`) must carry `#[tracing::instrument(skip_all)]` so production traces capture latency and error propagation across the full handler lifecycle.

**How to apply:**
```rust
#[tracing::instrument(skip_all)]
pub async fn handle_ai_photoshop_msg(
    bot: Bot,
    db: Arc<dyn Database>,
    dialogue: MyDialogue,
    mut state: AiPhotoshopState,
    msg: Message,
) -> HandlerResult {
    // ...
}
```

**Reference:** `rings/SILVER-RING-SN00/src/ai_photoshop.rs`, `avatar_brain.rs`, `chat_with_avatar.rs`, `digital_avatar_body.rs`, `fal_render.rs`, `flux_kontext.rs`, `hedra_render.rs`, `heygen_render.rs`, `image_to_prompt.rs`, `remove_bg.rs`


### shared-types-deny-unknown-fields
All inbound deserialization structs in the shared types crate must use deny_unknown_fields to prevent silent field truncation across bot, config, generation, money, payment, scene, and user modules.

How to apply:
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BotConfig {
    pub token: String,
    pub webhook_url: Option<String>,
}
```

Important: Place the serde attribute AFTER the derive to avoid derive helper attribute is used before it is introduced.

Reference: rings/GOLD-RING-TY00/src/bot.rs, config.rs, generation.rs, money.rs, payment.rs, scene.rs, user.rs

### state-mutation-ordering-guard
Never mutate external state (dialogue, database, file system) before validating identity or authorization prerequisites. In handle_main_menu_msg, the telegram_id sentinel guard was moved to the function entry so that dialogue.update(scene) only executes after validation passes.

How to apply:
```rust
let tid = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
if tid == 0 {
    tracing::warn!("Missing telegram_id; aborting main menu handler");
    return Ok(());
}
// only now proceed to load_lang, text matching, and state transition
```

Reference: rings/SILVER-RING-SN00/src/handlers.rs


### trait-crate-deny-unknown-fields
All inbound deserialization structs and enums in the traits crate (GOLD-RING-TR00) must use deny_unknown_fields to prevent silent field truncation across trait boundary implementations (job queue, payment gateway, database).

How to apply:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub id: uuid::Uuid,
    pub job_type: String,
    // ...
}
```

Important: Place the serde attribute AFTER the derive to avoid derive helper attribute is used before it is introduced.

Reference: rings/GOLD-RING-TR00/src/job_queue.rs, payment_gateway.rs

### webhook-per-field-length-caps
Untrusted form-data and webhook payload fields must have per-field length validation immediately after deserialization, before any processing, logging, or storage.

How to apply:
```rust
const MAX_INV_ID_LEN: usize = 128;
const MAX_SIGNATURE_LEN: usize = 512;
const MAX_OUT_SUM_LEN: usize = 32;

if form.inv_id.len() > MAX_INV_ID_LEN {
    tracing::warn!(len = form.inv_id.len(), "Webhook rejected: inv_id too long");
    return "ERROR: invalid inv_id".to_string();
}
```

Log only the length, not the field content, to prevent log injection.

Reference: rings/BRONZE-RING-SRV/src/payment_webhooks.rs

### server-error-body-sanitization
Edge-hardening middleware must sanitize BOTH 4xx client-error and 5xx server-error response bodies to prevent information disclosure from internal failures (DB outages, panics, unhandled exceptions).

How to apply:
```rust
if (code.is_client_error() || code.is_server_error()) && code != axum::http::StatusCode::TOO_MANY_REQUESTS {
    return build_sanitized_response(code);
}
```

Preserve the 429 exemption because rate-limit responses are generated by a dedicated Tower layer and carry no internal details.

Reference: rings/BRONZE-RING-SRV/src/router.rs


### payment-external-id-lookup
Payment completion code must use the correct database lookup method for the ID type. External gateway transaction IDs (Robokassa InvId, blockchain hashes, Telegram charge IDs) are NOT UUIDs and must be queried via `get_transaction_by_external_id`. Internal UUIDs should use `get_transaction`.

How to apply:
```rust
let tx = self.db.get_transaction_by_external_id(&verification.transaction_id)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("transaction {}", verification.transaction_id)))?;
```

Reference: rings/SILVER-RING-PY00/src/services/payment_processor.rs

### macro-generated-deny-unknown-fields
Macro-generated structs that implement Deserialize must include `#[serde(deny_unknown_fields)]` inside the macro body. This ensures ALL generated instances reject unknown fields.

How to apply:
```rust
macro_rules! scene_state {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            pub step: u8,
            $( pub $field: Option<$ty>, )*
        }
    };
}
```

Reference: rings/SILVER-RING-TG00/src/state.rs

### constant-time-comparison-no-length-branch
Constant-time comparison functions must NEVER branch on input length before the comparison loop. A length-check branch leaks the expected value length through timing.

How to apply:
```rust
let mut diff = (expected.len() != provided.len()) as u8;
for (a, b) in expected.bytes().zip(provided.bytes()) {
    diff |= a ^ b;
}
if diff != 0 {
    return Err((StatusCode::UNAUTHORIZED, "Invalid webhook secret".to_string()));
}
```

Reference: rings/BRONZE-RING-SRV/src/webhooks.rs


### ssrf-unspecified-and-ipv4-mapped-ipv6
URL validation for webhook result URLs must reject unspecified addresses (0.0.0.0, ::) and IPv4-mapped IPv6 addresses (::ffff:127.0.0.1, ::ffff:192.168.x.x) that bypass IPv4 filters.

How to apply:
```rust
if ip.is_unspecified() {
    return Err("URL points to an unspecified address".to_string());
}
match ip {
    std::net::IpAddr::V6(v6) => {
        if let Some(v4) = v6.to_ipv4_mapped() {
            if v4.is_loopback() || v4.is_private() || v4.is_link_local() {
                return Err("URL points to an IPv4-mapped internal address".to_string());
            }
        }
        // ... existing ULA and link-local checks
    }
    // ...
}
```

Reference: rings/BRONZE-RING-SRV/src/webhooks.rs

### health-endpoint-no-version
Unauthenticated health endpoints must never expose the application version (env!("CARGO_PKG_VERSION")) or build identifiers. These enable version fingerprinting and targeted attacks.

How to apply:
```rust
Json(json!({
    "status": "ok",
    "timestamp": chrono::Utc::now().to_rfc3339(),
}))
```

Reference: rings/BRONZE-RING-SRV/src/health.rs

### payment-gateway-constant-time-signature
Payment gateway callback signature verification must use constant-time comparison without early-exit length checks. The length-check branch leaks the expected signature length through timing.

How to apply:
```rust
let mut diff = (expected.len() != signature_value.len()) as u8;
for (a, b) in expected.bytes().zip(signature_value.bytes()) {
    diff |= a ^ b;
}
if diff != 0 {
    return Err(AppError::Validation("Signature mismatch".into()));
}
```

Reference: rings/SILVER-RING-PY00/src/robokassa.rs


### startup-panic-elimination
Environment variable loaders and other startup functions must never use panic! for recoverable errors. Missing env vars or invalid integers should return sentinel values with logging rather than crashing the process.

How to apply:
```rust
fn load_mandatory_i64(env_var: &str) -> i64 {
    let raw = match std::env::var(env_var) {
        Ok(v) => v,
        Err(_) => {
            tracing::error!("{} environment variable is required but not set; defaulting to 0", env_var);
            return 0;
        }
    };
    match raw.parse::<i64>() {
        Ok(v) => v,
        Err(_) => {
            tracing::error!("{} is not a valid i64; defaulting to 0", env_var);
            0
        }
    }
}
```

Reference: rings/SILVER-RING-TG00/src/access.rs

### fsm-ttl-expiry-guards
Callback handlers that dispatch jobs must verify required Option state fields are Some before deducting balance or enqueueing work. If InMemStorage TTL expires between steps, the state resets to default and all Option fields become None.

How to apply:
```rust
let prompt = match state.prompt.as_ref() {
    Some(p) => p.clone(),
    None => {
        let err = if lang.is_russian() { "Сессия устарела" } else { "Session expired" };
        bot.send_message(chat_id, err).await?;
        return return_to_menu(&bot, &dialogue, chat_id, lang).await;
    }
};
```

Reference: rings/SILVER-RING-SN00/src/text_to_image.rs, text_to_video.rs, neuro_photo.rs

### db-string-truncation
Database insert/update operations must cap string fields that come from external sources (provider responses, webhooks, user input) to prevent unbounded storage.

How to apply:
```rust
fn truncate_string(s: &str, max: usize, context: &str) -> String {
    if s.len() > max {
        tracing::warn!(%context, len = s.len(), max, "Truncating string");
        s[..max].to_string()
    } else {
        s.to_string()
    }
}
```

Reference: rings/SILVER-RING-DB00/src/repository.rs

### sql-parameter-type-safety
Never cast wide integer types (`u64`, `usize`) to narrow database bindings (`i32`) when using raw SQL parameterization. Use the widest matching SeaORM `Value` variant and clamp before casting.

How to apply:
```rust
let secs_i64 = older_than_secs.min(i64::MAX as u64) as i64;
Value::BigInt(Some(secs_i64))
```

Reference: rings/SILVER-RING-JB00/src/queue.rs

### panic-isolation-spawn-and-inspect
Never wrap an async future in `std::panic::catch_unwind`. Instead, `tokio::spawn` the handler as a separate task and await the `JoinHandle`. If `JoinError` occurs, mark the job failed and continue.

How to apply:
```rust
let mut task = tokio::spawn(handler(job));
let join_result = tokio::select! {
    r = &mut task => Some(r),
    _ = tokio::time::sleep(timeout) => {
        task.abort();
        None
    }
};
match join_result {
    Some(Ok(Ok(()))) => { /* success */ }
    Some(Ok(Err(e))) => { /* handler error */ }
    Some(Err(join_err)) => {
        tracing::error!("Job handler panicked");
        // mark job failed
    }
    None => { /* timeout */ }
}
```

Reference: rings/SILVER-RING-JB00/src/worker.rs

### atomic-status-cas-guard
Every `UPDATE` that transitions a job to a new status must include `AND status = 'expected_current_status'` in the `WHERE` clause. Prevents blind overwrites from duplicate workers or race conditions.

How to apply:
```rust
UPDATE job_queue
SET status = 'completed',
    completed_at = NOW()
WHERE id = $1
  AND status = 'running'
```

Reference: rings/SILVER-RING-JB00/src/queue.rs

### webhook-idempotency-after-signature
Never record a webhook event in the idempotency table before verifying its cryptographic signature. A forged callback could poison the cache and cause the legitimate callback to be dropped on replay.

How to apply:
```rust
match gateway.verify_callback(&payload).await {
    Ok(verification) => {
        record_webhook_event(&payload.inv_id, &payload.signature_value).await?;
        // ... process verified callback
    }
    Err(e) => { tracing::warn!("Signature verification failed: {}", e); }
}
```

Reference: rings/BRONZE-RING-SRV/src/payment_webhooks.rs

### sanitize-user-facing-errors
Never send raw `AppError` / `anyhow` / `Display` strings to end users. Use generic localized messages. Log the full error internally via `tracing::error!` to preserve ops visibility.

How to apply:
```rust
let err_msg = if lang.is_russian() {
    "❌ Не удалось отправить задачу. Попробуйте позже.".to_string()
} else {
    "❌ Could not submit task. Please try again later.".to_string()
};
tracing::error!("Internal enqueue failure: {}", e);
bot.send_message(chat_id, err_msg).await?;
```

Reference: rings/SILVER-RING-SN00/src/generation_utils.rs

### handler-entry-authorization
Even when a scene is registered with `AccessLevel::Admin`, add a runtime authorization check at the handler entry point after the `telegram_id` sentinel guard. Defense-in-depth against deep-link bypasses, FSM state restoration bugs, and routing errors.

How to apply:
```rust
let me = match bot.get_me().await {
    Ok(u) => u,
    Err(e) => { tracing::warn!("Failed to get bot info: {}", e); return Ok(()); }
};
let bot_name = me.user.username.as_deref().unwrap_or("");
if !trios_mb_tg::access::has_parsing_access(telegram_id, bot_name) {
    tracing::warn!(%telegram_id, %bot_name, "User lacks parsing access");
    bot.send_message(chat_id, "Access denied.").await?;
    return Ok(());
}
```

Reference: rings/SILVER-RING-SN00/src/instagram_scraping.rs, instagram_parser.rs

### enum-fail-closed
Never use `_ => NonTerminalState` as a catch-all for unrecognized database enum values. Map unknown/corrupted values to a terminal state (`Failed` or `Unknown`) and log a warning.

How to apply:
```rust
status: match row.status.as_str() {
    "queued" => JobStatus::Queued,
    "running" => JobStatus::Running,
    "completed" => JobStatus::Completed,
    "failed" => JobStatus::Failed,
    "cancelled" => JobStatus::Cancelled,
    other => {
        tracing::warn!(unknown_status = %other, job_id = ?row.id, "Unrecognized status; treating as failed");
        JobStatus::Failed
    }
},
```

Reference: rings/SILVER-RING-JB00/src/queue.rs

### dequeue-retry-budget
The `SELECT ... WHERE status = 'queued'` query must also include `AND attempts < max_attempts` to prevent exhausted jobs from being dequeued again.

How to apply:
```rust
SELECT id FROM job_queue
WHERE status = 'queued'
  AND (scheduled_at IS NULL OR scheduled_at <= NOW())
  AND job_type IN (...)
  AND attempts < max_attempts
ORDER BY created_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED
```

Reference: rings/SILVER-RING-JB00/src/queue.rs

### url-parse-before-check
Never use `contains("domain.com")` for URL validation. Always parse with `url::Url`, enforce exact scheme whitelist, exact host whitelist, and reject embedded credentials.

How to apply:
```rust
let parsed = url::Url::parse(&input)?;
match parsed.scheme() {
    "http" | "https" => {},
    _ => return Err("Invalid scheme".into()),
}
let host_ok = parsed.host_str().map_or(false, |h| {
    let lower = h.to_lowercase();
    lower == "instagram.com" || lower == "www.instagram.com"
});
if !host_ok { return Err("Invalid host".into()); }
if !parsed.username().is_empty() || parsed.password().is_some() {
    return Err("Credentials not allowed".into());
}
```

Reference: rings/SILVER-RING-SN00/src/instagram_parser.rs

### stuck-job-threshold-gt-max-timeout
The `retry_stuck` threshold must be >= `max(timeout_secs)` across all job types. A threshold shorter than any legitimate timeout causes false-positive stuck detection, duplicate execution, and wasted work.

How to apply:
```rust
// Threshold must cover the longest legitimate job timeout (ModelTraining = 7200s).
match tokio::time::timeout(QUEUE_IO_TIMEOUT, queue.retry_stuck(7200)).await { ... }
```

Reference: rings/SILVER-RING-JB00/src/worker.rs

### supervisor-time-decay-reset
Never use a monotonic panic counter. Track `last_failure` and reset the counter after a stable period (e.g., 5 minutes). This follows the Erlang OTP `intensity` + `period` and Akka `withinTimeRange` patterns.

How to apply:
```rust
let mut consecutive_failures: u32 = 0;
let mut last_failure: Option<Instant> = None;
const FAILURE_RESET_SECS: u64 = 300;

let now = Instant::now();
if last_failure.map_or(false, |t| now.duration_since(t).as_secs() >= FAILURE_RESET_SECS) {
    consecutive_failures = 0;
}
// ... on failure ...
consecutive_failures += 1;
last_failure = Some(now);
```

Reference: rings/SILVER-RING-JB00/src/worker.rs

### truncate-attacker-input-before-log
Any value derived from external input (webhook payloads, query params, headers, user messages) must be passed through `truncate_for_log` before emission in `tracing::*` or `#[tracing::instrument]` spans.

How to apply:
```rust
tracing::warn!(
    input = %truncate_for_log(s, 256),
    error = %e,
    "Invalid UUID in webhook payload"
);
```

Reference: rings/BRONZE-RING-SRV/src/webhooks.rs, payment_webhooks.rs

### spawn-traced-time-decay-app-entry
Any `spawn_traced` supervisor in the main application entry point (not just the worker) must implement the same time-decay reset as the worker supervisor. A monotonic counter in the app-level dispatcher causes premature panic on transient Telegram API errors.

How to apply:
```rust
let mut last_failure: Option<Instant> = None;
const FAILURE_RESET_SECS: u64 = 300;

loop {
    let now = Instant::now();
    if last_failure.map_or(false, |t| now.duration_since(t).as_secs() >= FAILURE_RESET_SECS) {
        consecutive_failures = 0;
    }
    // ... run supervised task ...
    // on failure:
    consecutive_failures += 1;
    last_failure = Some(now);
}
```

Reference: rings/BRONZE-RING-APP/src/main.rs

### telegram-api-timeout-wrapper
Never let `bot.send_message(...)` or `dialogue.update(...)` hang indefinitely in a Telegram handler. Wrap both with `tokio::time::timeout(Duration::from_secs(30), ...)` to prevent FSM state loss under API congestion or network partition. On timeout, log a warning and return `Ok(())`.

How to apply:
```rust
pub async fn send_message_timeout(
    bot: &Bot,
    chat_id: ChatId,
    text: String,
) -> Result<Message, AppError> {
    match tokio::time::timeout(TELEGRAM_API_TIMEOUT, bot.send_message(chat_id, text)).await {
        Ok(Ok(msg)) => Ok(msg),
        Ok(Err(e)) => Err(AppError::from(e)),
        Err(_) => {
            tracing::warn!(chat_id = %chat_id, "Telegram send_message timed out");
            Err(AppError::Internal("telegram timeout".into()))
        }
    }
}
```

Reference: rings/SILVER-RING-SN00/src/generation_utils.rs

### tracing-instrument-provider-traits
Add `#[tracing::instrument(skip_all)]` to every `async fn` in AI provider and payment gateway trait implementations (`generate`, `check_status`, `get_result`, `create_payment`, `verify_callback`, `refund`, `get_payment_url`). This maintains distributed trace correlation across provider boundaries during production incidents without capturing sensitive payloads.

How to apply:
```rust
#[async_trait::async_trait]
impl AiProvider for FalProvider {
    #[tracing::instrument(skip_all)]
    async fn generate(&self, request: &GenerationRequest) -> Result<GenerationResult, AppError> { ... }

    #[tracing::instrument(skip_all)]
    async fn check_status(&self, generation_id: &str) -> Result<GenerationStatus, AppError> { ... }

    #[tracing::instrument(skip_all)]
    async fn get_result(&self, generation_id: &str) -> Result<Option<String>, AppError> { ... }
}
```

Reference: rings/SILVER-RING-AI00/src/providers/{fal,openai,replicate,elevenlabs,heygen,hedra,kie,midjourney}.rs, rings/SILVER-RING-PY00/src/{robokassa,ton,x402,telegram_stars}.rs, rings/SILVER-RING-AI00/src/orchestrator.rs

### provider-success-body-cap
Never call `.json()` or `.bytes()` on an external HTTP success response without first checking `resp.content_length()` against a hard cap. A compromised or misbehaving provider can stream a multi-gigabyte payload and OOM the process.

How to apply:
```rust
pub(super) fn check_json_body_size(resp: &reqwest::Response, provider: &str, max_bytes: u64) -> Result<(), AppError> {
    if let Some(len) = resp.content_length() {
        if len > max_bytes {
            return Err(AppError::Ai(AiError::Provider {
                provider: provider.to_string(),
                message: format!("response body too large: {} bytes (max {})", len, max_bytes),
            }));
        }
    }
    Ok(())
}

// At every success-path .json() call site:
super::check_json_body_size(&resp, "fal", 64_000_000)?;
resp.json::<QueueResponse>().await...
```

Reference: rings/SILVER-RING-AI00/src/providers/mod.rs, rings/SILVER-RING-AI00/src/providers/{fal,heygen,hedra,kie,openai,elevenlabs,replicate}.rs

### webhook-idempotency-after-mutation
Record the webhook idempotency key **after** the state-mutating DB update succeeds, never before. If the idempotency record is written first and the DB update then fails, the provider retry will be deduplicated and skipped, permanently leaving the generation in an inconsistent state.

How to apply:
```rust
// 1. Verify signature
// 2. Parse payload
// 3. Terminal-state check
// 4. Update generation status (DB) — return 503 on failure
// 5. Record idempotency
// 6. Return 200 OK
```

Reference: rings/BRONZE-RING-SRV/src/webhooks.rs

### balance-deduction-state-guard
Never call `deduct_balance` before validating that all prerequisite dialogue-state fields are present and non-empty. A stale callback (session timeout, expired inline keyboard) can trigger the confirm handler without the user having completed the prerequisite step.

How to apply:
```rust
"ac:confirm" => {
    if state.audio_url.is_none() || state.audio_url.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        let text = "❌ Please send an audio file first.";
        bot.send_message(chat_id, text).await?;
        return Ok(());
    }
    if let Err(err_msg) = deduct_balance(&db, tid, COST, lang).await { ... }
}
```

Reference: rings/SILVER-RING-SN00/src/{ai_cover.rs,voice_training.rs,ai_reels.rs,music_generation.rs}

### telegram-api-timeout-wrapper
Any Telegram API call that blocks a user-interaction path (answer_callback_query, send_message, edit_message_text) must be wrapped with `tokio::time::timeout` to prevent handler starvation under network congestion. Prefer a shared helper that logs a clear error and returns a typed error instead of hanging forever.

How to apply:
```rust
pub async fn answer_callback_query_timeout(
    bot: &Bot,
    query_id: &str,
) -> Result<teloxide::types::True, std::io::Error> {
    match tokio::time::timeout(TELEGRAM_API_TIMEOUT, bot.answer_callback_query(query_id)).await {
        Ok(result) => result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Telegram API error: {}", e))),
        Err(_) => {
            tracing::error!(query_id = %query_id, "bot.answer_callback_query timed out");
            Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Telegram API answer_callback_query timeout"))
        }
    }
}
```

Reference: `rings/SILVER-RING-TG00/src/utils.rs`, `rings/SILVER-RING-SN00/src/*.rs`

### worker-failure-state-recovery
After a handler error, the worker must **always** attempt to write a terminal job status. Never let a secondary failure (e.g., `queue.get` timing out) skip `queue.update_status`. When the attempt count is unknown, default to `JobStatus::Queued` with a warning log.

How to apply:
```rust
let status = match tokio::time::timeout(QUEUE_IO_TIMEOUT, queue.get(job_id)).await {
    Ok(Ok(Some(j))) => {
        if j.attempts >= j.max_attempts { JobStatus::Failed } else { JobStatus::Queued }
    }
    Ok(Ok(None)) => {
        tracing::warn!("Job not found; defaulting to Queued");
        JobStatus::Queued
    }
    Ok(Err(e)) | Err(_) => {
        tracing::warn!("get failed; defaulting to Queued");
        JobStatus::Queued
    }
};
if let Err(e) = tokio::time::timeout(QUEUE_IO_TIMEOUT, queue.update_status(job_id, status, Some(&err_str))).await {
    tracing::error!("Failed to update job status after failure");
}
```

Reference: `rings/SILVER-RING-JB00/src/worker.rs`

### retry-stuck-threshold-margin
Queue maintenance thresholds (`retry_stuck`) must exceed the longest worker abort timeout by a deterministic safety margin. A threshold equal to the timeout creates a race where maintenance flags a job as stuck before the worker aborts it, causing duplicate execution.

How to apply:
```rust
const STUCK_JOB_MARGIN_SECS: u64 = 300;
let max_timeout = JobType::all_types()
    .iter()
    .map(|t| t.timeout_secs())
    .max()
    .unwrap_or(7200);
let stuck_threshold = max_timeout + STUCK_JOB_MARGIN_SECS;
queue.retry_stuck(stuck_threshold).await
```

Reference: `rings/SILVER-RING-JB00/src/worker.rs`

### retry-stuck-safety-net
Queue maintenance (`retry_stuck`) must cover ALL terminal states that could be orphaned, not just `status = 'running'`. After a handler failure, if the worker cannot verify the attempt count, it may default to `Queued` — but if `attempts >= max_attempts`, the job will never be dequeued again because `dequeue` filters by `attempts < max_attempts`. Add an orphan-cleanup query to `retry_stuck` that scans `status = 'queued' AND attempts >= max_attempts`.

How to apply:
```rust
let sql_orphan = r#"
    UPDATE job_queue
    SET status = 'failed',
        error = 'Queued but attempts exhausted (orphaned)',
        started_at = NULL,
        completed_at = NOW(),
        updated_at = NOW()
    WHERE id IN (
        SELECT id FROM job_queue
        WHERE status = 'queued'
          AND attempts >= max_attempts
        ORDER BY created_at ASC
        LIMIT $1
        FOR UPDATE SKIP LOCKED
    )
"#;
```

Reference: `rings/SILVER-RING-JB00/src/queue.rs`

### telegram-send-message-timeout
All `bot.send_message` calls in the central dispatcher must be wrapped with `tokio::time::timeout`. The helper should accept `Option<ReplyMarkup>` (not just `InlineKeyboardMarkup`) so it works with both inline keyboards and reply keyboards.

How to apply:
```rust
pub async fn send_message_timeout(
    bot: &Bot,
    chat_id: ChatId,
    text: impl Into<String>,
    reply_markup: Option<ReplyMarkup>,
) -> Result<Message, std::io::Error> {
    let text = text.into();
    let fut = match reply_markup {
        Some(markup) => bot.send_message(chat_id, text).reply_markup(markup),
        None => bot.send_message(chat_id, text),
    };
    match tokio::time::timeout(TELEGRAM_API_TIMEOUT, fut).await { ... }
}
```

Reference: `rings/SILVER-RING-TG00/src/utils.rs`, `rings/SILVER-RING-SN00/src/handlers.rs`

### cancellation-token-propagation
When spawning nested tasks for panic isolation, propagate the shutdown token through every layer. Dropping a `JoinHandle` without aborting the inner task leaks in-flight work, which can continue mutating state after shutdown is requested.

How to apply:
```rust
let join_result = tokio::select! {
    r = &mut task => Some(r),
    _ = cancel.cancelled() => {
        task.abort();
        tracing::warn!("Job aborted due to shutdown signal");
        None
    }
    _ = tokio::time::sleep(timeout) => {
        task.abort();
        None
    }
};
```

Reference: `rings/SILVER-RING-JB00/src/worker.rs`

### parse-json-limited-body
Replace `check_json_body_size(&resp, ...)?; resp.json::<T>().await?` with a combined helper that reads bytes under a hard timeout, checks actual length, then parses JSON. This closes the chunked-transfer bypass where `content_length()` is `None` and `.json().await` buffers an infinite stream into memory.

How to apply:
```rust
pub(super) async fn parse_json_limited<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
    provider: &str,
    max_bytes: u64,
) -> Result<T, AppError> {
    let bytes = match tokio::time::timeout(Duration::from_secs(30), resp.bytes()).await {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => return Err(AppError::Ai(AiError::Provider {
            provider: provider.to_string(),
            message: format!("failed to read response body: {}", e),
        })),
        Err(_) => return Err(AppError::Ai(AiError::Provider {
            provider: provider.to_string(),
            message: "response body read timed out".to_string(),
        })),
    };
    if bytes.len() as u64 > max_bytes {
        return Err(AppError::Ai(AiError::Provider {
            provider: provider.to_string(),
            message: format!("response body too large: {} bytes (max {})", bytes.len(), max_bytes),
        }));
    }
    serde_json::from_slice(&bytes).map_err(|e| {
        AppError::Ai(AiError::InvalidResponse {
            provider: provider.to_string(),
            message: format!("json parse: {}", e),
        })
    })
}
```

Reference: `rings/SILVER-RING-AI00/src/providers/mod.rs`

### read-error-body-timeout
Error response bodies must also be read under a timeout. A malicious endpoint can stream an infinite error payload and hang the Tokio worker.

How to apply:
```rust
match tokio::time::timeout(Duration::from_secs(10), resp.bytes()).await {
    Ok(Ok(b)) => { /* check len, convert to string */ }
    Ok(Err(e)) => format!("(failed to read error body: {})", e),
    Err(_) => "(error body read timed out)".to_string(),
}
```

Reference: `rings/SILVER-RING-AI00/src/providers/mod.rs`

### dialogue-update-timeout-wrapper
All `dialogue.update` and `dialogue.exit` calls in the central dispatcher must be wrapped with `tokio::time::timeout`. A stalled storage update leaves the user in an inconsistent FSM state and blocks the handler thread indefinitely.

How to apply:
```rust
pub async fn dialogue_update_timeout(
    dialogue: &Dialogue<Scene, InMemStorage<Scene>>,
    scene: Scene,
) -> Result<(), std::io::Error> {
    match tokio::time::timeout(TELEGRAM_API_TIMEOUT, dialogue.update(scene)).await {
        Ok(result) => result.map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Telegram API error: {}", e))
        }),
        Err(_) => {
            tracing::error!("dialogue.update timed out after {}s", TELEGRAM_API_TIMEOUT.as_secs());
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Telegram API dialogue.update timeout"
            ))
        }
    }
}

pub async fn dialogue_exit_timeout(
    dialogue: &Dialogue<Scene, InMemStorage<Scene>>,
) -> Result<(), std::io::Error> {
    match tokio::time::timeout(TELEGRAM_API_TIMEOUT, dialogue.exit()).await {
        Ok(result) => result.map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Telegram API error: {}", e))
        }),
        Err(_) => {
            tracing::error!("dialogue.exit timed out after {}s", TELEGRAM_API_TIMEOUT.as_secs());
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Telegram API dialogue.exit timeout"
            ))
        }
    }
}
```

Reference: `rings/SILVER-RING-TG00/src/utils.rs`, `rings/SILVER-RING-SN00/src/handlers.rs`

### max-attempts-clamp-at-enqueue
Queue retry limits must be validated at enqueue time. A `max_attempts` of 0 or negative creates a permanently undequeuable job because the SQL condition `attempts < max_attempts` is always false.

How to apply:
```rust
let raw_max = request.max_attempts.unwrap_or(3);
let max_attempts = raw_max.max(1);
if raw_max <= 0 {
    tracing::warn!(%id, raw_max_attempts = raw_max, "max_attempts clamped to 1; invalid value rejected at enqueue time");
}
```

Reference: `rings/SILVER-RING-JB00/src/queue.rs`

### no-debug-format-in-user-facing-text
Never use `format!("{:?}", value)` in messages sent to end users. Debug formatting leaks internal Rust enum variant names and struct layout, aiding attacker reconnaissance (CWE-209). Map internal enums to localized human-readable names via explicit `match`.

How to apply:
```rust
let name = match st {
    SubscriptionType::NeuroPhoto => "NeuroPhoto",
    SubscriptionType::NeuroVideo => "NeuroVideo",
    SubscriptionType::Stars => "Stars",
    SubscriptionType::NeuroTester => "NeuroTester",
};
```

Reference: `rings/SILVER-RING-SN00/src/subscription.rs`

### instagram-handler-send-message-timeout
Any handler file that imports `Bot` and calls `bot.send_message` must also import `send_message_timeout` from `trios_mb_tg`. This includes non-dispatcher files such as `instagram_scraping.rs`, `instagram_parser.rs`, and `subscription.rs`.

Reference: `rings/SILVER-RING-SN00/src/instagram_scraping.rs`, `rings/SILVER-RING-SN00/src/instagram_parser.rs`, `rings/SILVER-RING-SN00/src/subscription.rs`

### centralized-dialogue-timeout-helper
Never use inline `tokio::time::timeout(Duration::from_secs(30), dialogue.update(...)).await` scattered across scene handlers. Centralize into `dialogue_update_timeout` and `dialogue_exit_timeout` helpers in `trios_mb_tg::utils`. This guarantees uniform timeout behavior, consistent logging, and prevents state-loss when the Telegram API stalls.

```rust
pub async fn dialogue_update_timeout(
    dialogue: &Dialogue<Scene, InMemStorage<Scene>>,
    scene: Scene,
) -> HandlerResult {
    match tokio::time::timeout(TELEGRAM_API_TIMEOUT, dialogue.update(scene)).await {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("dialogue.update timed out");
            Ok(())
        }
    }
}
```

Usage at call site:
```rust
use trios_mb_tg::dialogue_update_timeout;
dialogue_update_timeout(&dialogue, Scene::NeuroPhoto(state)).await?;
```

**Reference:** `rings/SILVER-RING-TG00/src/utils.rs`, `rings/SILVER-RING-SN00/src/neuro_photo.rs`, `text_to_image.rs`, `text_to_video.rs`, `lip_sync.rs`, `generation_utils.rs`

### generation-utils-timeout-hygiene
Utility functions that wrap `bot.send_message` and `dialogue.update` must import the centralized timeout helpers (`send_message_timeout`, `dialogue_update_timeout`) rather than re-declaring local `Duration` constants and inline `tokio::time::timeout` blocks. This removes duplication and ensures all call sites inherit helper improvements (e.g., logging, metric increments) automatically.

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs` (`return_to_menu`, `dispatch_and_reply`)

### batch-scene-handler-timeout-migration
When migrating a large number of scene handlers, prioritize files with the fewest bare `dialogue.update` / `bot.send_message` calls first. This creates momentum and clears the simplest files before tackling complex multi-state handlers. Files with exactly 1–2 calls can be done in bulk; files with 4+ calls require careful review of each state transition to ensure the helper is applied correctly.

**Reference:** `rings/SILVER-RING-SN00/src/menu.rs`, `change_language.rs`, `balance.rs`, `help.rs`, `size.rs`, `video_duration.rs`

### unused-state-mutation-warning-as-logic-bug
A `value assigned to state is never read` warning often signals that a state mutation is orphaned: the field is set but never persisted to the dialogue or the database. Do not simply suppress the warning. Instead:
1. Check if the value is already persisted elsewhere (e.g., DB update call).
2. If yes, remove the redundant state mutation and the `mut` qualifier.
3. If no, the feature may be incomplete — add a TODO or a DB persistence call, or remove the dead assignment.

**Reference:** `rings/SILVER-RING-SN00/src/select_model.rs`, `email.rs`

### multi-step-fsm-timeout-wrappers
Complex multi-step FSM handlers (image collection, audio collection, inline keyboard selection) are the highest-risk sites for state-loss during Telegram API stalls because they involve multiple `dialogue.update` transitions. Prioritize these over simple one-step handlers when doing batch timeout wrapper migrations.

**Reference:** `rings/SILVER-RING-SN00/src/train_flux_model.rs`, `morphing.rs`, `avatar_transform.rs`

### duplicate-tracing-instrument-cleanup
A duplicate `#[tracing::instrument(skip_all)]` attribute does not cause a compiler error but creates redundant span nesting and slightly increases overhead. Remove duplicates immediately when spotted during file edits:

```rust
// BEFORE:
#[tracing::instrument(skip_all)]
#[tracing::instrument(skip_all)]
pub async fn handle_...(

// AFTER:
#[tracing::instrument(skip_all)]
pub async fn handle_...(
```

**Reference:** `rings/SILVER-RING-SN00/src/morphing.rs`

### media-collection-handler-timeout-migration
Audio-collection and image-collection handlers (`ai_cover.rs`, `voice_training.rs`, `image_to_video.rs`, `flux_kontext.rs`) share a common pattern: multi-step FSM with inline keyboard selection → media upload → prompt confirmation → dispatch. All `dialogue.update` transitions and all `bot.send_message` calls in these flows must be wrapped with the centralized timeout helpers. The `reply_markup` parameter requires `.into()` conversion from `InlineKeyboardMarkup` to `ReplyMarkup`.

**Reference:** `rings/SILVER-RING-SN00/src/ai_cover.rs`, `voice_training.rs`, `image_to_video.rs`, `flux_kontext.rs`

### wizard-handler-timeout-migration
Wizard-style handlers (`avatar_brain.rs`) collect multiple text fields in sequence (company name → position → skills). Each step has validation error branches, session-expiry guards, and state transitions. All error replies, prompt requests, and `dialogue.update` calls must be wrapped. The initial step often includes a back/cancel keyboard, so `send_message_timeout` receives `Some(keyboard.into())`.

**Reference:** `rings/SILVER-RING-SN00/src/avatar_brain.rs`

### render-dispatch-handler-timeout-migration
AI render dispatch handlers (`digital_avatar_body.rs`, `face_swap.rs`, `ai_photoshop.rs`) share a common pattern: inline keyboard selection or intro with back_cancel keyboard → media upload (one or two images) → optional prompt → balance deduction → `dispatch_and_reply`. All `dialogue.update` transitions and all `bot.send_message` calls in these flows must be wrapped. Files with multiple identical error branches (e.g., `face_swap.rs` has the same image-retrieval error in step 1 and step 2) can safely use `replace_all` when applying the wrapper.

**Reference:** `rings/SILVER-RING-SN00/src/digital_avatar_body.rs`, `face_swap.rs`, `ai_photoshop.rs`

### back-cancel-keyboard-timeout-pattern
Many render handlers use `crate::generation_utils::back_cancel_keyboard(lang)` as the initial message keyboard. This returns an `InlineKeyboardMarkup`, which must be converted via `.into()` to `ReplyMarkup` before passing to `send_message_timeout`.

**Reference:** `rings/SILVER-RING-SN00/src/face_swap.rs`, `ai_photoshop.rs`, `avatar_brain.rs`

### final-dispatch-handler-timeout-migration
The last batch of AI render dispatch handlers (`music_generation.rs`, `remove_bg.rs`, `ai_reels.rs`, `fal_render.rs`, `hedra_render.rs`, `heygen_render.rs`) complete the dialogue.update timeout wrapper migration. These files share patterns seen in earlier waves but include notable variants:
- **Multi-modal input:** `hedra_render.rs` accepts text OR voice in step 2; both branches need independent wrappers.
- **Model/style inline keyboards:** `music_generation.rs` (Suno/Udio) and `ai_reels.rs` (cinematic/anime/realistic) use `InlineKeyboardMarkup` for selection.
- **Variable prompt length caps:** `music_generation.rs` uses 4000 chars; most others use 2000 chars.
- **Avatar ID length cap:** `heygen_render.rs` enforces 128-character avatar ID before proceeding.

**Reference:** `rings/SILVER-RING-SN00/src/music_generation.rs`, `remove_bg.rs`, `ai_reels.rs`, `fal_render.rs`, `hedra_render.rs`, `heygen_render.rs`

### dialogue-update-migration-complete
After ~8 waves, zero bare `dialogue.update` calls remain across all scene handlers. The centralized `dialogue_update_timeout` helper prevents FSM state-loss during Telegram API stalls. Any new handler file must use `dialogue_update_timeout` for every state transition.

**Reference:** `rings/SILVER-RING-TG00/src/utils.rs` (`dialogue_update_timeout`)

### entry-handler-timeout-migration
Entry handlers (`handle_*_entry`) often use `trios_mb_i18n::t(lang, "send_photo")` or similar i18n strings as the initial prompt, then immediately transition to step 1 via `dialogue_update_timeout`. Both the `send_message_timeout` and `dialogue_update_timeout` calls in entry handlers must be wrapped.

**Reference:** `rings/SILVER-RING-SN00/src/neuro_photo.rs` (`handle_neuro_photo_entry`), `text_to_image.rs` (`handle_text_to_image_entry`), `text_to_video.rs` (`handle_text_to_video_entry`)

### main-menu-keyboard-timeout-pattern
Done/retry callbacks that return the user to the main menu use `main_menu_keyboard(lang)` as the reply markup. This returns an `InlineKeyboardMarkup`, which must be converted via `.into()` to `ReplyMarkup` before passing to `send_message_timeout`.

**Reference:** `rings/SILVER-RING-SN00/src/neuro_photo.rs`, `text_to_image.rs`, `text_to_video.rs`

### duplicate-tracing-instrument-scan
When editing handler files for timeout wrappers, always scan for duplicate `#[tracing::instrument(skip_all)]` attributes. Duplicates do not cause compiler errors but create redundant span nesting. `neuro_photo.rs` had duplicates on both `handle_neuro_photo_entry` and `handle_neuro_photo_msg` that were missed in earlier waves.

**Reference:** `rings/SILVER-RING-SN00/src/neuro_photo.rs`

### single-step-handler-timeout-migration
Single-step handlers (`tech_support.rs`, `neuro_coder.rs`, `invite.rs`) have minimal FSM state but still send validation error replies and confirmation messages. All `bot.send_message` calls in these files must be wrapped with `send_message_timeout`, including inline conditional strings like `if lang.is_russian() { ... } else { ... }`.

**Reference:** `rings/SILVER-RING-SN00/src/tech_support.rs`, `neuro_coder.rs`, `invite.rs`

### multi-modal-lipsync-timeout-migration
`lip_sync.rs` accepts video → audio OR voice → model selection. Both audio and voice branches share the same inline keyboard pattern (SyncLabs/HeyGen/Hedra/Fal). When applying wrappers, `replace_all` can safely handle both identical branches simultaneously.

**Reference:** `rings/SILVER-RING-SN00/src/lip_sync.rs`

### telegram-api-timeout-migration-complete
After 10 waves (194–203), both the `dialogue.update` → `dialogue_update_timeout` and `bot.send_message` → `send_message_timeout` migrations are fully complete across all scene handlers. Every Telegram API outbound call now has a hard 30-second timeout, preventing worker hangs and FSM state-loss during API stalls. Any new handler file must import both helpers and use them for all API calls.

**Reference:** `rings/SILVER-RING-TG00/src/utils.rs` (`send_message_timeout`, `dialogue_update_timeout`)

### unbounded-response-body-cap
Never use bare `resp.text().await` or `resp.json().await` on untrusted external endpoints. Both buffer the entire response stream into memory without a length check, allowing a malicious peer to OOM the service. Replace with helpers that apply a hard byte cap and timeout:

```rust
async fn read_body_limited(resp: reqwest::Response, max_bytes: usize) -> Result<String, AppError> {
    let bytes = match tokio::time::timeout(Duration::from_secs(10), resp.bytes()).await {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => return Err(...),
        Err(_) => return Err(...),
    };
    if bytes.len() > max_bytes {
        return Err(...);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

async fn read_json_limited<T: serde::de::DeserializeOwned>(resp: reqwest::Response, max_bytes: usize) -> Result<T, AppError> {
    let bytes = match tokio::time::timeout(Duration::from_secs(10), resp.bytes()).await { ... };
    if bytes.len() > max_bytes { return Err(...); }
    serde_json::from_slice(&bytes).map_err(...)
}
```

**Reference:** `rings/SILVER-RING-SC00/src/store.rs` (`authenticate`, `load_secrets`)

### error-body-truncate-before-propagation
When reading an HTTP error response body, truncate it before embedding it into `AppError` strings or log lines. Even with a byte cap (e.g., 1 MiB), a large HTML error page can bloat error messages and logs:

```rust
let body = read_body_limited(resp, MAX_BODY_BYTES).await?;
let truncated = trios_mb_types::truncate_for_log(&body, 4096);
return Err(AppError::Secrets(SecretsError::Auth(format!("{}: {}", status, truncated))));
```

**Reference:** `rings/SILVER-RING-SC00/src/store.rs` (`authenticate`, `load_secrets` error paths)

### secret-store-debug-redaction
Implement `std::fmt::Debug` manually for any struct that stores credentials (`client_id`, `client_secret`, `access_token`, `api_key`). Do not rely on `#[derive(Debug)]` — it will leak secrets into logs, crash dumps, and telemetry. Redact sensitive fields to `[REDACTED]` and show only counts or safe metadata for collections:

```rust
impl std::fmt::Debug for InfisicalStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InfisicalStore")
            .field("client_id", &"[REDACTED]")
            .field("client_secret", &"[REDACTED]")
            .field("project_id", &"[REDACTED]")
            .field("environment", &self.environment)
            .field("http", &self.http)
            .field("cache", &self.cache)
            .finish()
    }
}
```

**Reference:** `rings/SILVER-RING-SC00/src/store.rs` (`InfisicalStore`, `SecretCache`)

### reqwest-pool-max-idle-per-host
Every `reqwest::Client::builder()` chain must include `.pool_max_idle_per_host(N)` (default 10). Without it, reqwest keeps an unbounded number of idle connections per host, which exhausts file descriptors under sustained load or connection churn.

```rust
reqwest::Client::builder()
    .timeout(Duration::from_secs(60))
    .connect_timeout(Duration::from_secs(10))
    .redirect(reqwest::redirect::Policy::none())
    .pool_max_idle_per_host(10)
    .build()
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`, `rings/SILVER-RING-SC00/src/store.rs`

### tracing-instrument-on-critical-helpers
All centralized utility functions that are called from many sites (e.g., `send_message_timeout`, `dialogue_update_timeout`, `edge_hardening`) must have `#[tracing::instrument(skip_all)]`. Untraced critical paths create incident-response blind spots.

```rust
#[tracing::instrument(skip_all, fields(chat_id = %chat_id))]
pub async fn send_message_timeout(...) -> Result<Message, std::io::Error> { ... }
```

**Reference:** `rings/SILVER-RING-TG00/src/utils.rs`, `rings/BRONZE-RING-SRV/src/router.rs`

### tracing-instrument-on-provider-methods
Every public async method on an AI provider that makes an outbound HTTP call must have `#[tracing::instrument(skip_all)]` with selective, non-secret fields (e.g., `model`, `voice_id`, `avatar_id`). This ensures outbound API latency and errors are visible in distributed traces.

```rust
#[tracing::instrument(skip_all, fields(model = %model))]
pub async fn chat_completion(&self, model: &str, ...) -> Result<String, AppError> { ... }
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/elevenlabs.rs`, `heygen.rs`, `openai.rs`

### tracing-instrument-on-ai-service-generate
AI service `generate` methods that bridge Telegram handlers to the orchestrator must have `#[tracing::instrument(skip_all)]`. Without these spans the trace chain breaks at the boundary between handler logic and provider dispatch, making latency attribution impossible.

```rust
#[tracing::instrument(skip_all)]
pub async fn generate(&self, telegram_id: i64, ...) -> Result<GenerationResult, AppError> { ... }
```

**Reference:** `rings/SILVER-RING-AI00/src/services/face_swap.rs`, `image_to_video.rs`, `lip_sync.rs`, `morphing.rs`, `neuro_photo.rs`, `text_to_image.rs`, `text_to_video.rs`, `tts.rs`, `upscaler.rs`

### tracing-instrument-on-db-repository-methods
Every business-logic async method in the database repository (`impl DbTrait for PostgresDatabase`) must have `#[tracing::instrument(skip_all)]`. DB operations are invisible in traces without them, preventing query latency histograms and slow-query detection.

**Scope:** `get_user_by_telegram_id`, `create_user`, `update_user_*`, `get_balance`, `deduct_balance`, `add_balance`, `create_transaction`, `get_transaction*`, `update_transaction_status`, `check_subscription`, `renew_subscription`, `save_prompt`, `get_prompt`, `create_generation`, `update_generation_status`, `get_generation`, `complete_robokassa_payment`, `record_webhook_event`, `has_webhook_event`, and all other async trait methods.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### tracing-instrument-on-job-queue-methods
All `impl JobQueue` methods (`enqueue`, `dequeue`, `update_status`, `get`, `cancel`, `retry_stuck`) must have `#[tracing::instrument(skip_all)]`. Background job processing is entirely invisible without these spans.

**Reference:** `rings/SILVER-RING-JB00/src/queue.rs`

### tracing-crate-dependency-check
Before adding `#[tracing::instrument(...)]` to a crate, verify `tracing = { workspace = true }` is present in the crate's `Cargo.toml`. The attribute macro compiles without an explicit `use tracing;` import, but the crate dependency must exist.

### remove-duplicate-tracing-instrument
Never leave duplicate `#[tracing::instrument(...)]` attributes on the same function. A proc-macro attribute wraps the function body; applying it twice creates nested spans on every invocation, inflating span counts and distorting latency attribution.

**Reference:** `rings/SILVER-RING-SN00/src/start.rs`

### tracing-instrument-on-secret-store-methods
All async methods on a secret store implementation (`authenticate`, `load_secrets`, `get`, `get_all`, `reload`, `health_check`) must have `#[tracing::instrument(skip_all)]`. Secret stores sit on the critical path for credential fetching. Use `skip_all` exclusively — never add `fields(key = %key)` because key names may reveal deployment topology or sensitive configuration names.

**Reference:** `rings/SILVER-RING-SC00/src/store.rs`

### tracing-instrument-on-provider-internal-helpers
Internal provider helper methods that make HTTP calls (`queue_submission`, `check_queue_status`, `fetch_result`, `create_animation`, `fetch_animation_status`, `send_request`, `check_task_status`, `create_prediction`, `fetch_prediction`) must have `#[tracing::instrument(skip_all)]`. These helpers are called from already-traced public trait methods, but without their own spans the individual RPC latencies are invisible. Thin wrappers (e.g., `submit_video` which just builds a string and calls `send_request`) do not need separate spans — instrument the common helper instead.

**Reference:** `rings/SILVER-RING-AI00/src/providers/fal.rs`, `hedra.rs`, `kie.rs`, `replicate.rs`

### tracing-instrument-on-inner-retry-loop
When a public method (e.g., `dispatch`) has tracing and wraps an inner method (`dispatch_inner`) in `tokio::time::timeout`, the inner method must also be traced. Otherwise the retry attempts, circuit breaker evaluations, and per-provider error recordings are invisible inside the parent span.

```rust
#[tracing::instrument(skip_all)]
async fn dispatch_inner(&self, request: &GenerationRequest) -> Result<GenerationResult, AppError> {
    // provider retry loop
}
```

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### tracing-instrument-on-frequently-called-utilities
Utility functions called from many handlers (e.g., `load_lang`, `load_lang_by_id`, `load_lang_cb`, `return_to_menu`) must have `#[tracing::instrument(skip_all)]`. Even though they are "just helpers," they execute DB queries or external API calls on every critical path. Without spans they create unexplained latency holes in handler traces.

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`

### tracing-instrument-on-startup-lifecycle
Database initialization methods (`connect`, `run_migrations`) and other boot-time async operations must have `#[tracing::instrument(skip_all)]`. When startup fails, these spans are often the only structured diagnostic data available because the service never reaches steady-state telemetry pipelines.

**Reference:** `rings/SILVER-RING-DB00/src/repository.rs`

### no-ok-flatten-silent-swallow
Never use `.ok().flatten().map(...).unwrap_or_default()` on a `Result` from a fallible subsystem (DB, cache, external API). The chain converts `Err` to `None` silently, making outages invisible. Use an explicit `match` with an `Err(e)` arm that logs before falling back.

```rust
match db.get_user_by_telegram_id(tid).await {
    Ok(Some(user)) => user.language,
    Ok(None) => Language::default(),
    Err(e) => {
        tracing::warn!(telegram_id = tid, error = %e, "Failed to load user language from DB; falling back to default");
        Language::default()
    }
}
```

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`

### no-let-underscore-discard
Never use `let _ = fallible_call().await` to discard a `Result`. This is a silent failure pattern. Replace with `if let Err(e) = fallible_call().await { tracing::warn!(..., error = %e, "...") }`.

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`

### timeout-branch-must-log
Every `tokio::time::timeout(...).await` must have a `tracing::warn!` in its `Err(_)` branch. Timeout is a degradation symptom; without a log, operators cannot distinguish timeout from logic failure without inspecting trace span durations.

```rust
match tokio::time::timeout(Duration::from_secs(120), self.dispatch_inner(request)).await {
    Ok(result) => result,
    Err(_) => {
        tracing::warn!(media_type = ?request.media_type, "Orchestrator dispatch timed out after 120s");
        Err(AppError::Ai(...))
    }
}
```

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### join-handle-result-inspection-shutdown
During graceful shutdown, never `let _ = tokio::time::timeout(..., handle).await`. A panicked task will be silently dropped. Match the nested `Result` and log panics at `error!`, timeouts at `warn`, clean exits at `debug`:

```rust
match tokio::time::timeout(Duration::from_secs(5), handle).await {
    Ok(Ok(())) => debug!("bot task exited cleanly"),
    Ok(Err(join_err)) => {
        if join_err.is_panic() {
            error!("bot task panicked during shutdown: {}", join_err);
        } else {
            error!("bot task cancelled during shutdown: {}", join_err);
        }
    }
    Err(_elapsed) => warn!("bot task shutdown timed out after 5s"),
}
```

**Reference:** `rings/BRONZE-RING-APP/src/main.rs`

### ok-flatten-db-error-swallowing
Never chain `db.some_query().await.ok().flatten()` on a query that returns `Result<Option<T>>`. The `.ok()` converts DB errors into `None`, which then collapses into "no data found" instead of "database outage". Use an explicit `match` and propagate the error:

```rust
let current_sub = match db.check_subscription(tid).await {
    Ok(opt) => opt,
    Err(e) => {
        tracing::error!(telegram_id = tid, error = %e, "DB error checking subscription");
        return Err(e.into());
    }
};
```

**Reference:** `rings/SILVER-RING-SN00/src/subscription.rs`

### serde-parse-ok-silence
Never use `serde_json::from_str(data).ok()` on untrusted input such as Telegram callback payloads. Deserialization errors are swallowed, hiding probing, corruption, or schema drift. Log the failure with a truncated payload preview:

```rust
match serde_json::from_str::<CallbackData>(data) {
    Ok(v) => Some(v),
    Err(e) => {
        let preview: String = data.chars().take(128).collect();
        tracing::warn!(error = %e, payload = %preview, "CallbackData parse failed");
        None
    }
}
```

**Reference:** `rings/GOLD-RING-PR00/src/telegram.rs`

### per-route-webhook-body-limit
Never apply a global 2 MB body limit to webhook endpoints. Webhook payloads are small JSON blobs; a 2 MB limit opens a trivial DoS vector. Add a per-route layer to the webhook sub-router:

```rust
let webhooks = Router::new()
    .route("/api/webhooks/replicate", post(replicate_webhook))
    .route("/api/webhooks/kie-ai", post(kie_ai_webhook))
    .layer(axum::extract::DefaultBodyLimit::max(256 * 1024));
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### health-endpoint-error-differentiation
Never conflate `Ok(false)` and `Err(e)` in health checks. Both return 503, but the operator must know whether the DB is reachable-but-unhealthy or unreachable:

```rust
match state.db.health_check().await {
    Ok(true) => (StatusCode::OK, Json(json!({"status": "ok", "db": "connected"}))),
    Ok(false) => {
        tracing::warn!("DB health check returned false; service degraded");
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"status": "degraded", "db": "unhealthy"})))
    }
    Err(e) => {
        tracing::error!(error = %e, "DB health check failed");
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"status": "degraded", "db": "disconnected"})))
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/health.rs`

### worker-lifecycle-tracing
Every public lifecycle method (`new`, `register`, `spawn`, `shutdown`) on worker pools should carry `#[tracing::instrument]` so pool configuration is visible in distributed traces. Use `skip_all` when `Box<dyn Fn>` arguments prevent automatic `Debug` capture:

```rust
#[tracing::instrument(skip_all, fields(job_type = %job_type.as_str()))]
pub fn register(&mut self, job_type: JobType, _handler: JobHandler) { ... }

#[tracing::instrument(skip_all, fields(handlers = self.handlers.len()))]
pub fn spawn(self: &Arc<Self>) { ... }
```

**Reference:** `rings/SILVER-RING-JB00/src/worker.rs`

### csp-fail-closed-default
Set a restrictive `Content-Security-Policy` on every HTTP response from an API server. The safe baseline for JSON-only APIs is `default-src 'none'; frame-ancestors 'none'; base-uri 'none'`. Any future relaxation (e.g., for a Telegram Mini-Web-App) must be explicit and reviewed.

```rust
headers.insert(
    "Content-Security-Policy",
    HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'; base-uri 'none'"),
);
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### referrer-policy-universal
Inject `Referrer-Policy: strict-origin-when-cross-origin` on every HTTP response. This sends only the origin (no path or query) to cross-origin referrers, preventing accidental leakage of signed URLs, tokens, or PII in the `Referer` header.

```rust
headers.insert(
    "Referrer-Policy",
    HeaderValue::from_static("strict-origin-when-cross-origin"),
);
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### secretstring-webhook-secret-storage
Webhook signing keys must be stored as `secrecy::SecretString` so the backing buffer is zeroised on drop and redacted in `Debug` output. Expose the raw value only at the HMAC verification boundary via `.expose_secret()`.

```rust
fn load_webhook_secret(env_var: &str) -> Option<SecretString> {
    match std::env::var(env_var) {
        Ok(v) if !v.is_empty() => Some(SecretString::new(v)),
        Ok(_) => { tracing::warn!(env_var, "Webhook secret is empty"); None }
        Err(_) => { tracing::warn!(env_var, "Webhook secret env var not set"); None }
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### cache-control-error-responses
Sanitized fallback error responses must carry `Cache-Control: no-cache, no-store, must-revalidate` and `Pragma: no-cache` so browsers and CDNs never cache 4xx/5xx bodies. Stale error pages complicate incident recovery.

```rust
Response::builder()
    .status(code)
    .header("Content-Type", "text/plain; charset=utf-8")
    .header("Content-Security-Policy", "default-src 'none'; frame-ancestors 'none'; base-uri 'none'")
    .header("Referrer-Policy", "strict-origin-when-cross-origin")
    .header("Cache-Control", "no-cache, no-store, must-revalidate")
    .header("Pragma", "no-cache")
    .body(Body::from("Bad Request"))
    .unwrap_or_else(|_| { /* fallback */ })
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### ssrf-ipv4-mapped-guard
IPv6 `::ffff:x.x.x.x` addresses bypass IPv4-only filters because the parser normalizes them before string checks run. Always check `v6.to_ipv4_mapped()` inside URL validators and run the same guards (loopback, private, link-local, unspecified) on the mapped IPv4.

```rust
std::net::IpAddr::V6(v6) => {
    if let Some(mapped) = v6.to_ipv4_mapped() {
        if mapped.is_loopback()
            || mapped.is_private()
            || mapped.is_link_local()
            || mapped.is_unspecified()
        {
            return Err("IPv4-mapped internal address".into());
        }
    }
    // ... native IPv6 checks ...
}
```

**Reference:** `rings/GOLD-RING-TY00/src/utils.rs`

### unspecified-address-block
`0.0.0.0` and `::` are not loopback but still resolve to the local machine. Always pair `ip.is_loopback()` with `ip.is_unspecified()` in URL validators.

```rust
if ip.is_loopback() || ip.is_unspecified() {
    return Err("URL points to a loopback or unspecified address".into());
}
```

**Reference:** `rings/GOLD-RING-TY00/src/utils.rs`

### vetted-constant-time-comparison
Never hand-roll XOR-and-OR loops for comparing secrets. The Rust/LLVM optimizer may dead-store-eliminate or branch-simplify the accumulator pattern. Use `subtle::ConstantTimeEq` which applies volatile barriers to resist compiler optimization.

```rust
use subtle::ConstantTimeEq;

let eq = expected.as_bytes().ct_eq(provided.as_bytes());
if eq.unwrap_u8() == 0 {
    return Err("signature mismatch".into());
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`, `rings/SILVER-RING-PY00/src/robokassa.rs`

### payment-secret-zeroization
Payment gateway signing keys must be stored as `secrecy::SecretString` and exposed only at the HMAC boundary via `.expose_secret()`.

```rust
pub struct Gateway {
    password: SecretString,
}

fn sign(&self, data: &str) {
    let mut mac = HmacSha256::new_from_slice(self.password.expose_secret().as_bytes()).unwrap();
    mac.update(data.as_bytes());
}
```

**Reference:** `rings/SILVER-RING-PY00/src/robokassa.rs`

### hsts-on-sanitized-errors
`Strict-Transport-Security` must appear on both success and error responses. Omitting it from `build_sanitized_response` creates a downgrade window for clients that first hit an error path.

```rust
Response::builder()
    .status(code)
    .header("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
    // ... other security headers ...
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### truncate-before-logging
Any field extracted from an external webhook or user payload must pass through `truncate_for_log` before interpolation in a `tracing` macro.

```rust
let weights_truncated = trios_mb_types::truncate_for_log(&weights, 256);
tracing::info!(weights = %weights_truncated, "Training completed");
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### secret-store-self-protection
A secret-management backend must protect its own credentials with the same rigor it protects client secrets. Store OAuth client secrets as `SecretString`.

```rust
pub struct InfisicalStore {
    client_secret: SecretString,
}

// Expose only at the HTTP boundary
"clientSecret": self.client_secret.expose_secret(),
```

**Reference:** `rings/SILVER-RING-SC00/src/store.rs`

### secret-cache-zeroization
In-memory caches that hold secrets (OAuth tokens, API keys, DB passwords) must store values as `SecretString` so buffers are zeroised on drop. The cache access_token and every cached secret value must be wrapped.

```rust
struct SecretCache {
    access_token: Option<SecretString>,
    secrets: HashMap<String, (SecretString, Instant)>,
}
```

**Reference:** `rings/SILVER-RING-SC00/src/store.rs`

### remove-serde-from-config-structs
Config structs containing credentials must not derive `Serialize` or `Deserialize`. Removing them prevents accidental JSON serialization of secrets and sidesteps the `SecretBox<str>` serde incompatibility in `secrecy` v0.10 (which lacks `SerializableSecret` for `?Sized` types).

```rust
#[derive(Clone)]
pub struct AppConfig {
    pub infisical_client_secret: SecretString,
    // ... no Serialize / Deserialize ...
}
```

**Reference:** `rings/GOLD-RING-TY00/src/config.rs`

### secrecy-v010-box-str-api
`secrecy` v0.10 changed `SecretString::new()` to accept `Box<str>` instead of `String`. Wrap all call sites with `.into_boxed_str()`:

```rust
SecretString::new(value.to_string().into_boxed_str())
```

**Reference:** `rings/SILVER-RING-SC00/src/store.rs`, `rings/SILVER-RING-PY00/src/robokassa.rs`, `rings/BRONZE-RING-SRV/src/router.rs`

### orchestrator-error-truncation
When logging provider errors that may contain arbitrarily large response bodies (up to 64 KB in some codebases), always truncate before `tracing` interpolation:

```rust
let err_msg = e.to_string();
let err_short = trios_mb_types::truncate_for_log(&err_msg, 256);
tracing::warn!(provider = %name, error = %err_short, "provider failed");
```

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### no-explicit-length-before-ct-eq
Never perform an explicit `expected.len() != provided.len()` check before `subtle::ConstantTimeEq::ct_eq`. The early return leaks the expected length via timing. `subtle::ct_eq` already handles different lengths by returning `Choice(0)`.

**Anti-pattern:**
```rust
if expected.len() != provided.len() {
    return Err("mismatch".into());  // TIMING LEAK
}
let eq = expected.as_bytes().ct_eq(provided.as_bytes());
```

**Correct:**
```rust
let eq = expected.as_bytes().ct_eq(provided.as_bytes());
if eq.unwrap_u8() == 0 {
    return Err("mismatch".into());
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`, `rings/SILVER-RING-PY00/src/robokassa.rs`

### minimise-secret-exposure-window
Functions that compare secrets should accept `SecretString` by reference, not `&str`. The caller passes the secret directly; `.expose_secret()` is called only inside the comparison body.

**Anti-pattern:**
```rust
fn verify(headers: &HeaderMap, expected: &str) { ... }
let secret = store.get("key").expose_secret();
verify(&headers, secret);  // secret exposed in caller's scope
```

**Correct:**
```rust
fn verify(headers: &HeaderMap, expected: &SecretString) {
    let raw = expected.expose_secret();  // exposure only here
    // ... compare ...
}
let secret = store.get("key");
verify(&headers, secret);  // no exposure in caller
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### reqwest-pool-idle-timeout-required
Every `reqwest::Client::builder()` chain that sets `pool_max_idle_per_host` must also set `pool_idle_timeout`. The default is **no idle timeout**, meaning connections can linger indefinitely and reuse revoked API keys after rotation.

```rust
let http = reqwest::Client::builder()
    .timeout(Duration::from_secs(120))
    .connect_timeout(Duration::from_secs(10))
    .redirect(reqwest::redirect::Policy::none())
    .pool_max_idle_per_host(10)
    .pool_idle_timeout(Duration::from_secs(90))  // REQUIRED
    .build()?;
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/*.rs`, `rings/SILVER-RING-SC00/src/store.rs`, `rings/SILVER-RING-PY00/src/x402.rs`

### instrument-security-critical-functions
Authentication and verification functions (constant-time comparison, HMAC verification, secret validation) should be traced so security teams can detect probe patterns in distributed traces.

```rust
#[tracing::instrument(skip_all)]
fn verify_webhook_secret(headers: &HeaderMap, expected: &SecretString) { ... }
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### monetary-amount-upper-bound
Every monetary amount parsed from external input must have both a lower bound (> 0, `is_finite()`) and an upper bound. The upper bound prevents business-logic abuse and unexpected downstream behavior.

```rust
const MAX_PAYMENT_AMOUNT: f64 = 100_000.0;

if amount > MAX_PAYMENT_AMOUNT {
    tracing::warn!(amount, max = %MAX_PAYMENT_AMOUNT, "Amount exceeds maximum allowed");
    return Err(AppError::Validation(format!("Amount exceeds maximum allowed: {}", amount)));
}
```

**Reference:** `rings/SILVER-RING-SN00/src/payment.rs`, `rings/BRONZE-RING-SRV/src/payment_webhooks.rs`

### never-unwrap-or-on-result
`db.operation().await.unwrap_or(0)` silently substitutes a sentinel on DB errors, masking outages and producing misleading UX. Always `match` the `Result`, log the error at `error!` level, and return a localized fallback message.

```rust
match db.get_referral_count(telegram_id).await {
    Ok(count) => format!("Referrals: {}", count),
    Err(e) => {
        tracing::error!(error = %e, "Failed to load referral count");
        "Unable to load referral count. Try again later.".to_string()
    }
}
```

**Reference:** `rings/SILVER-RING-SN00/src/handlers.rs`, `rings/SILVER-RING-SN00/src/invite.rs`

### provider-empty-prompt-rejection
Every AI provider handler must reject empty or whitespace-only prompts before dispatching to the external API. Passing an empty prompt wastes API credits and causes cryptic downstream errors:

```rust
let prompt = request.prompt.as_deref().unwrap_or("").trim();
if prompt.is_empty() {
    return Err(AppError::Validation("Empty prompt is not allowed".to_string()));
}
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`, `elevenlabs.rs`, `heygen.rs`, `kie.rs`, `fal.rs`, `replicate.rs`, `hedra.rs`

### webhook-error-info-disclosure-sanitization
Webhook and callback handlers must never return diagnostic strings that reveal internal configuration (missing secrets, uninitialized subsystems, database timeouts) to unauthenticated callers. Return `"Internal server error"` in the HTTP body and log the full diagnostic server-side via `tracing::error!`:

```rust
// BAD — leaks infra state
return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Webhook secret not configured"})));

// GOOD — generic to caller, detailed in logs
tracing::error!("REPLICATE_WEBHOOK_SECRET not loaded at startup; rejecting webhook");
return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Internal server error"})));
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`, `payment_webhooks.rs`

### scene-handler-empty-text-rejection
Every Telegram scene handler that accepts free-text input must reject empty or whitespace-only strings before the length-cap check. A whitespace-only message passes a naive length check but wastes user balance and provider API credits:

```rust
if let Some(text) = msg.text() {
    if text.trim().is_empty() {
        let err = if lang.is_russian() { "❌ Пустой текст недопустим." } else { "❌ Empty text is not allowed." };
        send_message_timeout(&bot, msg.chat.id, err, None).await?;
        return Ok(());
    }
    if text.len() > MAX_DIALOGUE_TEXT_LEN { /* ... */ }
}
```

**Reference:** `rings/SILVER-RING-SN00/src/chat_with_avatar.rs`, `improve_prompt.rs`, `neuro_coder.rs`, `tech_support.rs`, `avatar_transform.rs`, `flux_kontext.rs`

### db-timeout-wrapper
Any DB call inside a user-facing async handler must be wrapped in `tokio::time::timeout` to prevent connection-pool exhaustion from stalling the dispatcher indefinitely:

```rust
const DB_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

match tokio::time::timeout(DB_TIMEOUT, db.get_user_by_telegram_id(tid)).await {
    Ok(Ok(Some(user))) => user.language,
    Ok(Ok(None)) => Language::default(),
    Ok(Err(e)) => {
        tracing::warn!(telegram_id = tid, error = %e, "DB error loading user; falling back");
        Language::default()
    }
    Err(_) => {
        tracing::warn!(telegram_id = tid, "DB timeout loading user; falling back");
        Language::default()
    }
}
```

**Reference:** `rings/SILVER-RING-SN00/src/generation_utils.rs`, `handlers.rs`, `invite.rs`

### provider-http-send-timeout
Even when the reqwest client has a builder-level `.timeout()`, add an outer `tokio::time::timeout` around `.send().await` as defense-in-depth. If reqwest's internal timer fails (TLS stall, pool exhaustion), the tokio timeout still aborts the future:

```rust
const PROVIDER_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

let resp = match tokio::time::timeout(
    PROVIDER_HTTP_TIMEOUT,
    self.http.post(url).header("Authorization", ...).json(&body).send(),
).await {
    Ok(Ok(resp)) => resp,
    Ok(Err(e)) => {
        return Err(AiError::Provider {
            provider: "openai".into(),
            message: format!("request failed: {}", e),
        }.into());
    }
    Err(_) => {
        return Err(AiError::Provider {
            provider: "openai".into(),
            message: "request timed out".to_string(),
        }.into());
    }
};
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`, `heygen.rs`, `elevenlabs.rs`, `kie.rs`, `fal.rs`, `replicate.rs`, `hedra.rs`

### tracing-instrument-internal-helpers
When a public trait method (`generate`) delegates to private/internal helpers (`generate_image`, `text_to_speech`), instrument the helpers too. Without inner spans, distributed traces show a single long span with no visibility into which helper ran or where it failed:

```rust
#[tracing::instrument(skip_all)]
async fn generate_image(&self, request: &GenerationRequest) -> Result<GenerationResult, AppError> { ... }

#[tracing::instrument(skip_all)]
async fn text_to_speech(&self, request: &GenerationRequest) -> Result<GenerationResult, AppError> { ... }
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/openai.rs`

### env-var-explicit-match
Never use `std::env::var(...).map(...).unwrap_or_default()` to silently discard missing environment variables. Replace with an explicit `match` that logs `tracing::warn!` at the exact call site so configuration drift is visible:

```rust
let origins: Vec<HeaderValue> = match std::env::var("FRONTEND_URL") {
    Ok(s) => {
        if s.is_empty() {
            tracing::warn!("FRONTEND_URL is set but empty; CORS requests denied");
            return Vec::new();
        }
        s.split(',')
            .filter_map(|o| o.trim().parse::<HeaderValue>().ok())
            .collect()
    }
    Err(e) => {
        tracing::warn!(error = %e, "FRONTEND_URL not set; CORS requests denied");
        Vec::new()
    }
};
```

**Reference:** `rings/BRONZE-RING-SRV/src/router.rs`

### inmemstorage-resource-cap
Any handler that accumulates user-provided items (images, files, messages) into a `Vec` stored in `InMemStorage`-backed dialogue state must enforce a hard maximum count. `InMemStorage` lacks TTL eviction; unbounded vectors are a memory-exhaustion DoS vector. Place the guard **before** pushing the new item and return a localized error without mutating state:

```rust
const MAX_MORPHING_IMAGES: usize = 10;
if images.len() >= MAX_MORPHING_IMAGES {
    let err = if lang.is_russian() {
        format!("❌ Максимум {} изображений для морфинга.", MAX_MORPHING_IMAGES)
    } else {
        format!("❌ Maximum {} images allowed for morphing.", MAX_MORPHING_IMAGES)
    };
    send_message_timeout(&bot, chat_id, err, None).await?;
    return Ok(());
}
images.push(file_id);
```

**Reference:** `rings/SILVER-RING-SN00/src/morphing.rs`, `train_flux_model.rs`

### empty-prompt-early-rejection
Reject empty or whitespace-only prompts at the earliest text handler boundary, **before** length-cap checks and **before** storing the prompt in state or dispatching to downstream AI providers. Empty prompts waste provider credits and pollute the job queue:

```rust
let text = match msg.text() {
    Some(t) => t,
    None => return Ok(()),
};

if text.trim().is_empty() {
    let err = if lang.is_russian() { "❌ Пустой промпт не допускается." } else { "❌ Empty prompt is not allowed." };
    send_message_timeout(&bot, msg.chat.id, err, None).await?;
    return Ok(());
}

if text.len() > 4000 { /* ... */ }
```

**Reference:** `rings/SILVER-RING-SN00/src/text_to_image.rs`, `text_to_video.rs`

### telegram-api-timeout-all-methods
Every Telegram Bot API method — not just `send_message` and `answer_callback_query` — must be wrapped in `tokio::time::timeout`. `bot.get_me().await` and any other infrequently-used methods are easy to miss during systematic migrations:

```rust
const TELEGRAM_API_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

let me = match timeout(TELEGRAM_API_TIMEOUT, bot.get_me()).await {
    Ok(Ok(u)) => u,
    Ok(Err(e)) => {
        tracing::warn!("Failed to get bot info: {}", e);
        return Ok(());
    }
    Err(_) => {
        tracing::warn!("bot.get_me() timed out");
        return Ok(());
    }
};
```

**Reference:** `rings/SILVER-RING-SN00/src/instagram_scraping.rs`, `instagram_parser.rs`

### provider-model-digest-env-var
Hardcoded AI provider model digests create deployment fragility: when the provider rotates a version, the baked-in digest becomes invalid and all generation requests fail until code is rebuilt. Use `std::sync::LazyLock<String>` to read the digest from an environment variable with the current known-good digest as a fallback:

```rust
use std::sync::LazyLock;

static REPLICATE_SDXL_MODEL: LazyLock<String> = LazyLock::new(|| {
    std::env::var("REPLICATE_SDXL_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "stability-ai/sdxl:7762fd07cf82c948538e41f63f77d685e02b063e37e496e96eefd46c929f9bdc".to_string())
});
```

Then reference the static in the model resolver rather than the literal string:

```rust
"sdxl" => Some(REPLICATE_SDXL_MODEL.clone()),
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/replicate.rs`

### named-timeout-constants
Never embed bare `Duration::from_secs(N)` literals inline. Timeout values are security controls (CWE-1088) and must be named constants so they are visible, grep-able, and tunable without recompilation:

```rust
const DISPATCH_TIMEOUT_SECS: u64 = 120;
const CHECK_STATUS_TIMEOUT_SECS: u64 = 30;
const GET_RESULT_TIMEOUT_SECS: u64 = 30;

match tokio::time::timeout(Duration::from_secs(DISPATCH_TIMEOUT_SECS), self.dispatch_inner(request)).await {
    Ok(result) => result,
    Err(_) => {
        tracing::warn!("dispatch timed out after {}s", DISPATCH_TIMEOUT_SECS);
        Err(AppError::Ai(AiError::AllProvidersFailed { ... }))
    }
}
```

**Reference:** `rings/SILVER-RING-AI00/src/orchestrator.rs`

### semantic-media-duration-validation
Enforce product-specified duration bounds on media uploads, not just presence/absence. Read `audio.duration.seconds()` or `voice.duration.seconds()` and reject files outside the acceptable range before persisting state or deducting balance:

```rust
const MIN_VOICE_TRAINING_DURATION: u32 = 30;
const MAX_VOICE_TRAINING_DURATION: u32 = 180;

let (file_id, duration) = if let Some(audio) = msg.audio() {
    (Some(audio.file.id.clone()), audio.duration.seconds())
} else if let Some(voice) = msg.voice() {
    (Some(voice.file.id.clone()), voice.duration.seconds())
} else {
    (None, 0)
};

if let Some(fid) = file_id {
    if duration < MIN_VOICE_TRAINING_DURATION || duration > MAX_VOICE_TRAINING_DURATION {
        let err = format!("Audio duration must be between {} and {} seconds.", MIN_VOICE_TRAINING_DURATION, MAX_VOICE_TRAINING_DURATION);
        send_message_timeout(&bot, chat_id, err, None).await?;
        return Ok(());
    }
    // ... proceed with training
}
```

**Reference:** `rings/SILVER-RING-SN00/src/voice_training.rs`

### health-db-timeout-constant
Wrap DB connectivity checks inside health endpoints with a named timeout constant. Without a timeout, a stalled DB connection pool causes health checks to hang indefinitely, defeating load-balancer failover and amplifying outages:

```rust
const HEALTH_DB_TIMEOUT: Duration = Duration::from_secs(5);

let db_ok = match timeout(HEALTH_DB_TIMEOUT, state.db.health_check()).await {
    Ok(Ok(true)) => true,
    Ok(Ok(false)) | Ok(Err(_)) | Err(_) => {
        tracing::warn!("Health check DB connection failed or timed out");
        false
    }
};
```

On timeout, return `503 Service Unavailable` with a structured JSON body (`{"db": "timeout"}`) so monitoring systems can distinguish DB degradation from other failures.

**Reference:** `rings/BRONZE-RING-SRV/src/health.rs`

### webhook-owned-method-telegram_id
When a webhook handler updates a DB row identified by a user-controlled UUID, always preserve the result of the initial `get_generation` lookup to extract `telegram_id`. Pass that `telegram_id` into the `_owned` variant of the update method (`update_generation_status_owned`) so the DB layer enforces row ownership. This adds defense-in-depth IDOR prevention even after signature verification:

```rust
let gen_opt = match state.db.get_generation(generation_id).await {
    Ok(Ok(gen)) => gen,
    Ok(Err(_)) | Err(_) => None,
};

let telegram_id = gen_opt.as_ref().map(|g| g.telegram_id);
if let Some(tid) = telegram_id {
    if let Err(e) = state.db.update_generation_status_owned(generation_id, GenerationStatus::Completed, result_url, None, tid).await {
        tracing::error!(error = %e, generation_id = %generation_id, "Failed to update generation status");
    }
}
```

**Reference:** `rings/BRONZE-RING-SRV/src/webhooks.rs`

### trim-empty-prompt-rejection
Reject whitespace-only input immediately after extracting `msg.text()`, before URL parsing, length checks, or state storage. A `trim().is_empty()` guard prevents wasted CPU and confusing downstream error messages from empty strings reaching validators:

```rust
let text = match msg.text() {
    Some(t) => t,
    None => return Ok(()),
};

if text.trim().is_empty() {
    let err = if lang.is_russian() { "❌ Пустая ссылка не допускается." } else { "❌ Empty link is not allowed." };
    bot.send_message(msg.chat.id, err).await?;
    return Ok(());
}
```

**Reference:** `rings/SILVER-RING-SN00/src/instagram_scraping.rs`, `instagram_parser.rs`

### dispatcher-text-empty-and-length-guard
Every text-dispatch handler that routes free-form user input to downstream scene handlers must reject empty/whitespace text and enforce a maximum length before any string comparisons or state mutations:

```rust
const MAX_MENU_TEXT_LEN: usize = 500;

let text = match msg.text() {
    Some(t) => t,
    None => { /* return localized error */ }
};

if text.trim().is_empty() {
    let err = if lang.is_russian() { "❌ Пустое сообщение не допускается." } else { "❌ Empty message is not allowed." };
    send_message_timeout(&bot, msg.chat.id, err, Some(main_menu_keyboard(lang).into())).await?;
    return Ok(());
}

if text.len() > MAX_MENU_TEXT_LEN {
    let err = if lang.is_russian() { "❌ Сообщение слишком длинное." } else { "❌ Message is too long." };
    send_message_timeout(&bot, msg.chat.id, err, Some(main_menu_keyboard(lang).into())).await?;
    return Ok(());
}
```

**Reference:** `rings/SILVER-RING-SN00/src/handlers.rs`

### telegram-media-size-cap
All handlers that accept Telegram media uploads (`msg.video()`, `msg.audio()`, `msg.voice()`) must validate `file.size` against a named constant before storing the file_id or deducting balance. The `file.size` field is a `u32` representing bytes (not `Option<u32>` in teloxide-core 0.10.1). A multi-gigabyte upload would waste provider quota and exhaust bandwidth:

```rust
const MAX_LIP_SYNC_VIDEO_BYTES: u64 = 100 * 1024 * 1024;
const MAX_LIP_SYNC_AUDIO_BYTES: u64 = 50 * 1024 * 1024;

if let Some(video) = msg.video() {
    if state.step <= 1 {
        if video.file.size as u64 > MAX_LIP_SYNC_VIDEO_BYTES {
            let err = if lang.is_russian() { "❌ Видео слишком большое. Максимум 100 МБ." } else { "❌ Video is too large. Maximum 100 MB." };
            send_message_timeout(&bot, msg.chat.id, err, None).await?;
            return Ok(());
        }
        // ... proceed
    }
}
```

**Reference:** `rings/SILVER-RING-SN00/src/lip_sync.rs`

### provider-helper-timeout-constants
Shared provider helper functions (`read_error_body`, `parse_json_limited`) must use named timeout constants, not bare `Duration::from_secs(N)` literals. Timeout values are security controls (CWE-1088) and must be visible and grep-able:

```rust
const ERROR_BODY_READ_TIMEOUT: Duration = Duration::from_secs(10);
const JSON_BODY_READ_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) async fn read_error_body(resp: reqwest::Response, max_bytes: usize) -> String {
    match resp.content_length() {
        Some(len) if len > max_bytes as u64 => {
            format!("(error body too large: {} bytes)", len)
        }
        _ => match tokio::time::timeout(ERROR_BODY_READ_TIMEOUT, resp.bytes()).await {
            // ...
        },
    }
}
```

**Reference:** `rings/SILVER-RING-AI00/src/providers/mod.rs`

### media-upload-size-cap-every-handler
Every handler that accepts Telegram media uploads (`msg.audio()`, `msg.photo()`, `msg.voice()`, `msg.video()`) must validate `file.size` against a named constant. Defense-in-depth requires size caps at every ingestion point, not just the first handler audited:

```rust
const MAX_AI_COVER_AUDIO_BYTES: u64 = 50 * 1024 * 1024;

if let Some(audio) = msg.audio() {
    if audio.file.size as u64 > MAX_AI_COVER_AUDIO_BYTES {
        let err = if lang.is_russian() { "❌ Аудиофайл слишком большой. Максимум 50 МБ." } else { "❌ Audio file too large. Maximum 50 MB." };
        send_message_timeout(&bot, msg.chat.id, err, None).await?;
        return Ok(());
    }
    // ... proceed
}
```

**Reference:** `rings/SILVER-RING-SN00/src/ai_cover.rs`, `hedra_render.rs`

### cancel-keyword-exact-match-with-trim
Replace substring matching (`text.contains("Cancel")`) with exact case-insensitive comparison (`trimmed.eq_ignore_ascii_case("cancel")`) for command intents. Substring matching creates false-positive cancellation risks (e.g., "cancellation policy" triggers an unintended cancel). Also guard empty/whitespace input and add a catch-all error for unrecognized text:

```rust
let trimmed = text.trim();
if trimmed.is_empty() {
    let err = if lang.is_russian() { "❌ Пустое сообщение не допускается." } else { "❌ Empty message is not allowed." };
    send_message_timeout(&bot, msg.chat.id, err, None).await?;
    return Ok(());
}

if text.len() > MAX_MORPHING_TEXT_LEN {
    let err = if lang.is_russian() { "❌ Сообщение слишком длинное." } else { "❌ Message is too long." };
    send_message_timeout(&bot, msg.chat.id, err, None).await?;
    return Ok(());
}

if trimmed.eq_ignore_ascii_case("отмена") || trimmed.eq_ignore_ascii_case("cancel") {
    return return_to_menu(&bot, &dialogue, msg.chat.id, lang).await;
}

let err = if lang.is_russian() { "❌ Неизвестная команда." } else { "❌ Unknown command." };
send_message_timeout(&bot, msg.chat.id, err, None).await?;
```

**Reference:** `rings/SILVER-RING-SN00/src/morphing.rs`
