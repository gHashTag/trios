//! Trinity Mesh Node — Neon Postgres persistence layer
//! L-E2E-4 · trinity-fpga#26 · EPIC trinity-fpga#22
//! φ² + φ⁻² = 3
//!
//! When `DATABASE_URL` is set, the daemon mirrors every accepted announce
//! into Neon and reloads non-expired rows on boot. Writes are best-effort
//! (logged on failure, never blocking the HTTP response). On Railway
//! ephemeral FS this collapses convergence after a restart from
//! 30–120 s to < 5 s.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use tracing::{info, warn};

use trios_mesh::routing::RouteEntry;

/// Default TTL window when reading rows back on boot. Routes older than this
/// are skipped (treated as already expired). Tunable via `MESH_ROUTE_TTL_HOURS`.
const DEFAULT_RELOAD_TTL_HOURS: i64 = 1;

/// Open a connection pool. Returns Ok(None) when `DATABASE_URL` is unset —
/// callers fall back to pure in-memory mode without erroring out.
pub async fn try_open_from_env() -> Result<Option<PgPool>> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => {
            info!("💾 DATABASE_URL not set — running in-memory only (no persistence)");
            return Ok(None);
        }
    };
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .context("connecting to DATABASE_URL")?;
    info!("💾 connected to Neon Postgres");
    Ok(Some(pool))
}

/// Apply embedded migrations. Idempotent (CREATE TABLE IF NOT EXISTS).
pub async fn migrate(pool: &PgPool) -> Result<()> {
    let sql = include_str!("../migrations/001_route_table.sql");
    // Run the migration as a single statement batch.
    sqlx::raw_sql(sql)
        .execute(pool)
        .await
        .context("applying migrations/001_route_table.sql")?;
    info!("💾 migrations applied");
    Ok(())
}

/// Upsert one route row. Best-effort: errors are logged but not propagated
/// so the announce hot path never fails because of a transient DB hiccup.
pub async fn upsert_route(
    pool: &PgPool,
    self_dest: &[u8; 16],
    dest: &[u8; 16],
    next_hop: &[u8; 16],
    hops: u8,
    quality: u8,
    pub_key: Option<&[u8; 32]>,
) {
    let res = sqlx::query(
        r#"
        INSERT INTO route_table
            (self_dest_hash, dest_hash, next_hop, hops, quality, pub_key, last_seen, announced_at)
        VALUES ($1, $2, $3, $4, $5, $6, now(), now())
        ON CONFLICT (self_dest_hash, dest_hash) DO UPDATE SET
            next_hop  = EXCLUDED.next_hop,
            hops      = EXCLUDED.hops,
            quality   = EXCLUDED.quality,
            pub_key   = COALESCE(EXCLUDED.pub_key, route_table.pub_key),
            last_seen = now()
        "#,
    )
    .bind(&self_dest[..])
    .bind(&dest[..])
    .bind(&next_hop[..])
    .bind(i16::from(hops & 0x0F))
    .bind(i16::from(quality & 0x0F))
    .bind(pub_key.map(|p| p.to_vec()))
    .execute(pool)
    .await;

    if let Err(e) = res {
        warn!("💾 upsert_route failed (non-fatal): {e}");
    }
}

/// Load this node's surviving routes on boot.
pub async fn load_routes(
    pool: &PgPool,
    self_dest: &[u8; 16],
) -> Result<Vec<RouteEntry>> {
    let ttl_hours = std::env::var("MESH_ROUTE_TTL_HOURS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(DEFAULT_RELOAD_TTL_HOURS);

    let rows = sqlx::query(
        r#"
        SELECT dest_hash, next_hop, hops, quality, last_seen
        FROM route_table
        WHERE self_dest_hash = $1
          AND last_seen > now() - make_interval(hours => $2::int)
        ORDER BY last_seen DESC
        LIMIT 16
        "#,
    )
    .bind(&self_dest[..])
    .bind(ttl_hours as i32)
    .fetch_all(pool)
    .await
    .context("SELECT route_table on boot")?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let dest_vec:    Vec<u8> = row.try_get("dest_hash")?;
        let next_vec:    Vec<u8> = row.try_get("next_hop")?;
        let hops:        i16     = row.try_get("hops")?;
        let quality:     i16     = row.try_get("quality")?;
        let last_seen:   DateTime<Utc> = row.try_get("last_seen")?;

        if dest_vec.len() != 16 || next_vec.len() != 16 {
            return Err(anyhow!("route row has malformed hash length"));
        }
        let mut dest = [0u8; 16];     dest.copy_from_slice(&dest_vec);
        let mut next = [0u8; 16];     next.copy_from_slice(&next_vec);

        out.push(RouteEntry {
            dest,
            next_hop: next,
            hops:    (hops    & 0x0F) as u8,
            quality: (quality & 0x0F) as u8,
            // Reuse the wall-clock seconds modulo u32 — only relative order matters.
            last_seen: last_seen.timestamp() as u32,
        });
    }
    info!("💾 reloaded {} route(s) from Neon", out.len());
    Ok(out)
}

/// Sweep rows older than `hours` from the table.
/// Run periodically by the daemon (or as a Railway cron).
pub async fn sweep_stale(pool: &PgPool, hours: i64) -> Result<u64> {
    let res = sqlx::query(
        "DELETE FROM route_table WHERE last_seen < now() - make_interval(hours => $1::int)"
    )
    .bind(hours as i32)
    .execute(pool)
    .await
    .context("DELETE stale route_table rows")?;
    Ok(res.rows_affected())
}
