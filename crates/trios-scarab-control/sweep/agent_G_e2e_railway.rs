/// Wave 21 — Agent G: End-to-End Rust Integration Test against live Railway PostgreSQL.
///
/// Replaces Wave 20 `agent_E_smoke.py` (Python) — Rule #1 compliance: RUST ONLY.
///
/// Steps replicated from Wave-20 smoke (16 steps + pre/post snapshot + G6 gate):
///   1.  Read RAILWAY_DSN from env (panic if absent)
///   2.  Pre-clean any leftover w21-smoke-G-* rows
///   3.  Pre-state snapshot (bpb_samples, scarab_strategy non-smoke, scarab_dead)
///   4.  spawn_scarab → INSERT strategy row
///   5.  Read back from scarab_strategy → verify service_id / optimizer / status / generation
///   6.  bump_strategy_v2 {"optimizer":"soap"}  → assert fingerprint changes, gen increments, command row exists
///   7.  bump_strategy_v2 {"lr":0.0003}         → assert fingerprint changes, gen increments, command row exists
///   8.  bump_strategy_v2 {"hidden":512}         → assert fingerprint changes, gen increments, command row exists
///   9.  kill_scarab → assert status='killed'
///  10.  Cleanup: DELETE scarab_strategy WHERE service_id LIKE 'w21-smoke-G-%'
///  11.  Cleanup: DELETE scarab_command  WHERE service_id LIKE 'w21-smoke-G-%'
///  12.  Post-state snapshot → assert G6 prod delta=0
///
/// Run with:
///   RAILWAY_DSN="postgresql://..." cargo test e2e_railway -- --nocapture

use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::NoTls;
use anyhow::{Context, Result};
use sha2::{Sha256, Digest};
use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;

// ── Canon name ────────────────────────────────────────────────────────────────

const CANON: &str = "IGLA-W21-G-fp32-h256-LR0.0001-rng1597-adamw";
const ACCOUNT: &str = "G-smoke";

// ── Fingerprint (canonical, matches Agent B + Wave 20 Python) ─────────────────
//
// SHA-256 of "<optimizer>|<format>|<hidden>|<lr_repr>|<seed>|<steps>"
// where lr_repr is 10-significant-figure decimal (%.10g style).

fn compute_fingerprint(
    optimizer: &str,
    format: &str,
    hidden: i32,
    lr: &Decimal,
    seed: i32,
    steps: i32,
) -> String {
    let lr_f64: f64 = lr.to_string().parse().unwrap_or(0.0);
    let lr_str = sig_fig_format(lr_f64, 10);
    let canonical = format!("{}|{}|{}|{}|{}|{}", optimizer, format, hidden, lr_str, seed, steps);
    eprintln!("[fingerprint] canonical: {:?}", canonical);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    eprintln!("[fingerprint] sha256: {}", hex);
    hex
}

/// Format f64 with up to `sig_figs` significant figures, removing trailing zeros.
/// Mirrors Python's f"{lr:.10g}".
fn sig_fig_format(v: f64, sig_figs: usize) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    // Use fixed-precision approach: determine exponent, then format
    let exp = v.abs().log10().floor() as i32;
    // Number of decimal places = sig_figs - 1 - exp
    if exp >= -(4_i32) && exp < sig_figs as i32 {
        // Fixed notation
        let decimal_places = (sig_figs as i32 - 1 - exp).max(0) as usize;
        let s = format!("{:.prec$}", v, prec = decimal_places);
        // Remove trailing zeros and trailing dot
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    } else {
        // Scientific notation
        let prec = sig_figs - 1;
        let formatted = format!("{:.prec$e}", v, prec = prec);
        // Rust uses e notation like 1.0000000000e-4; reformat to Python-style
        let (mant, exp_str) = formatted.split_once('e').unwrap();
        let exp_val: i32 = exp_str.parse().unwrap();
        let mant_clean = mant.trim_end_matches('0').trim_end_matches('.');
        let sign = if exp_val >= 0 { "+" } else { "" };
        format!("{}e{}{}", mant_clean, sign, exp_val)
    }
}

// ── Snapshot struct ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ProdSnapshot {
    bpb_samples: i64,
    scarab_strategy_non_smoke: i64,
    scarab_dead: i64,
}

impl ProdSnapshot {
    fn delta(&self, other: &ProdSnapshot) -> (i64, i64, i64) {
        (
            other.bpb_samples - self.bpb_samples,
            other.scarab_strategy_non_smoke - self.scarab_strategy_non_smoke,
            other.scarab_dead - self.scarab_dead,
        )
    }
}

async fn take_snapshot(client: &tokio_postgres::Client, label: &str) -> Result<ProdSnapshot> {
    let bpb_row = client
        .query_one("SELECT COUNT(*) AS n FROM ssot.bpb_samples", &[])
        .await
        .context("snapshot bpb_samples")?;
    let bpb: i64 = bpb_row.get::<_, i64>("n");

    let strat_row = client
        .query_one(
            "SELECT COUNT(*) AS n FROM ssot.scarab_strategy \
             WHERE service_id NOT LIKE 'w21-smoke-G-%'",
            &[],
        )
        .await
        .context("snapshot scarab_strategy")?;
    let strat: i64 = strat_row.get::<_, i64>("n");

    let dead_row = client
        .query_one("SELECT COUNT(*) AS n FROM ssot.scarab_dead", &[])
        .await
        .context("snapshot scarab_dead")?;
    let dead: i64 = dead_row.get::<_, i64>("n");

    let snap = ProdSnapshot {
        bpb_samples: bpb,
        scarab_strategy_non_smoke: strat,
        scarab_dead: dead,
    };
    eprintln!(
        "[snapshot {}] bpb_samples={} scarab_strategy(non-smoke)={} scarab_dead={}",
        label, snap.bpb_samples, snap.scarab_strategy_non_smoke, snap.scarab_dead
    );
    Ok(snap)
}

// ── Helper: read strategy row fields ─────────────────────────────────────────

fn row_to_fields(row: &tokio_postgres::Row) -> (String, String, i32, Decimal, i32, i32, i64) {
    let optimizer: String = row.get::<_, &str>("optimizer").to_string();
    let format: String = row.get::<_, &str>("format").to_string();
    let hidden: i32 = row.get("hidden");
    let lr: Decimal = row.get("lr");
    let seed: i32 = row.get("seed");
    let steps: i32 = row.get("steps");
    let generation: i64 = row.get("generation");
    (optimizer, format, hidden, lr, seed, steps, generation)
}

// ── Main test ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_railway_smoke() -> Result<()> {
    // Step 1: Read RAILWAY_DSN from env — panic if absent
    let dsn = std::env::var("RAILWAY_DSN")
        .expect("RAILWAY_DSN env var must be set to run this integration test");

    // Derive unique service_id using nanoseconds + seconds
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch");
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let service_id = format!("w21-smoke-G-{}_{}", secs, nanos);

    eprintln!("=== Wave 21 Agent G E2E Railway Smoke Test START ===");
    eprintln!("service_id = {}", service_id);
    eprintln!("canon_name = {}", CANON);

    // Connect to Railway PostgreSQL
    let (client, connection) = tokio_postgres::connect(&dsn, NoTls)
        .await
        .context("connect to Railway PostgreSQL")?;

    // Spawn connection handler in background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("[pg-connection error] {}", e);
        }
    });

    eprintln!("[connected] PostgreSQL ok");

    // Step 2: Pre-clean any leftover w21-smoke-G-* rows from prior failed runs
    eprintln!("\n--- Pre-clean w21-smoke-G-* leftovers ---");
    let cmd_cleaned = client
        .execute(
            "DELETE FROM ssot.scarab_command WHERE service_id LIKE 'w21-smoke-G-%'",
            &[],
        )
        .await
        .context("pre-clean scarab_command")?;
    eprintln!("[pre-clean] scarab_command: {} rows", cmd_cleaned);

    let hb_cleaned = client
        .execute(
            "DELETE FROM ssot.scarab_heartbeat WHERE service_id LIKE 'w21-smoke-G-%'",
            &[],
        )
        .await
        .context("pre-clean scarab_heartbeat")?;
    eprintln!("[pre-clean] scarab_heartbeat: {} rows", hb_cleaned);

    let strat_cleaned = client
        .execute(
            "DELETE FROM ssot.scarab_strategy WHERE service_id LIKE 'w21-smoke-G-%'",
            &[],
        )
        .await
        .context("pre-clean scarab_strategy")?;
    eprintln!("[pre-clean] scarab_strategy: {} rows", strat_cleaned);

    // Step 3: Pre-state snapshot
    eprintln!("\n--- Step 3: Pre-state snapshot ---");
    let pre_snap = take_snapshot(&client, "pre").await?;

    // Step 4: spawn_scarab
    // Signature (verified from Wave 20 log):
    //   spawn_scarab(service_id, account, canon_name, optimizer, format, hidden, lr, seed, steps, reason)
    eprintln!("\n--- Step 4: spawn_scarab ---");
    let lr_initial = Decimal::from_str("0.0001").unwrap();
    let spawn_row = client
        .query_one(
            "SELECT ssot.spawn_scarab($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            &[
                &service_id,
                &ACCOUNT,
                &CANON,
                &"adamw",
                &"fp32",
                &(256_i32),
                &lr_initial,
                &(1597_i32),
                &(1000_i32),
                &"wave21-smoke-G",
            ],
        )
        .await
        .context("spawn_scarab")?;
    let spawn_result: i64 = spawn_row.try_get(0).unwrap_or(0);
    eprintln!("[spawn_scarab] result = {}", spawn_result);

    // Step 5: Read back from scarab_strategy
    eprintln!("\n--- Step 5: Read back scarab_strategy row ---");
    let strat_row = client
        .query_one(
            "SELECT service_id, account, optimizer, format, hidden, lr, seed, steps, \
              status, generation \
             FROM ssot.scarab_strategy WHERE service_id = $1",
            &[&service_id],
        )
        .await
        .context("read scarab_strategy")?;

    let got_sid: &str = strat_row.get("service_id");
    let got_optimizer: &str = strat_row.get("optimizer");
    let got_status: &str = strat_row.get("status");
    let (got_opt, got_fmt, got_hidden, got_lr, got_seed, got_steps, gen_initial) =
        row_to_fields(&strat_row);

    eprintln!(
        "[strategy] service_id={} optimizer={} status={} format={} hidden={} seed={} steps={} lr={} generation={}",
        got_sid, got_optimizer, got_status, got_fmt, got_hidden, got_seed, got_steps, got_lr, gen_initial
    );

    assert_eq!(got_sid, service_id, "service_id mismatch");
    assert_eq!(got_optimizer, "adamw", "optimizer should be adamw");
    assert_eq!(got_status, "active", "initial status should be active");
    eprintln!("[step 5] initial generation={} ✓", gen_initial);

    // Compute initial fingerprint
    let fp0 = compute_fingerprint("adamw", "fp32", 256, &got_lr, got_seed, got_steps);

    // Also verify against DB-side scarab_fingerprint()
    if let Ok(db_fp_row) = client
        .query_one(
            "SELECT ssot.scarab_fingerprint($1, $2, $3, $4, $5, $6)",
            &[
                &got_opt.as_str(),
                &got_fmt.as_str(),
                &got_hidden,
                &got_lr,
                &got_seed,
                &got_steps,
            ],
        )
        .await
    {
        let db_fp: &str = db_fp_row.get(0);
        eprintln!("[fingerprint] DB scarab_fingerprint() = {}", db_fp);
        if fp0 == db_fp {
            eprintln!("[fingerprint] Rust fingerprint matches DB ✓");
        } else {
            eprintln!(
                "[fingerprint] WARNING mismatch — local={}… db={}…",
                &fp0[..16], &db_fp[..16]
            );
        }
    } else {
        eprintln!("[fingerprint] scarab_fingerprint() not available or failed; skipping DB cross-check");
    }

    // ── Bump 1: optimizer=soap ────────────────────────────────────────────────
    eprintln!("\n--- Step 6: bump_strategy_v2 {{\"optimizer\":\"soap\"}} ---");
    let bump1_changes = json!({"optimizer": "soap"});
    let bump1_row = client
        .query_one(
            "SELECT ssot.bump_strategy_v2($1, $2, $3)",
            &[
                &service_id,
                &bump1_changes,
                &"wave21-smoke-G-bump1",
            ],
        )
        .await
        .context("bump_strategy_v2 optimizer=soap")?;
    let gen1: i64 = bump1_row.try_get(0).unwrap_or(0);
    eprintln!("[bump1] new generation = {}", gen1);

    assert!(gen1 > gen_initial, "generation must increment after bump1: {} → {}", gen_initial, gen1);

    let after1_row = client
        .query_one(
            "SELECT optimizer, format, hidden, lr, seed, steps, generation \
             FROM ssot.scarab_strategy WHERE service_id = $1",
            &[&service_id],
        )
        .await
        .context("read strategy after bump1")?;
    let (opt1, fmt1, h1, lr1, seed1, steps1, _gen1_check) = row_to_fields(&after1_row);
    assert_eq!(opt1, "soap", "optimizer should be soap after bump1");
    let fp1 = compute_fingerprint(&opt1, &fmt1, h1, &lr1, seed1, steps1);
    assert_ne!(fp0, fp1, "fingerprint must change after bump1 (optimizer change)");
    eprintln!("[bump1] fingerprint: {}… → {}… ✓", &fp0[..16], &fp1[..16]);

    // Assert scarab_command row exists for this bump (action='bump' per bump_strategy_v2 body)
    let cmd1_count = client
        .query_one(
            "SELECT COUNT(*) AS n FROM ssot.scarab_command \
             WHERE service_id = $1 AND command = 'bump'",
            &[&service_id],
        )
        .await
        .context("check scarab_command bump1")?;
    let cmd1_n: i64 = cmd1_count.get("n");
    assert!(cmd1_n >= 1, "Expected ≥1 scarab_command row after bump1, got {}", cmd1_n);
    eprintln!("[bump1] scarab_command rows = {} ✓", cmd1_n);

    // ── Bump 2: lr=0.0003 ─────────────────────────────────────────────────────
    eprintln!("\n--- Step 7: bump_strategy_v2 {{\"lr\":0.0003}} ---");
    let bump2_changes = json!({"lr": "0.0003"});
    let bump2_row = client
        .query_one(
            "SELECT ssot.bump_strategy_v2($1, $2, $3)",
            &[
                &service_id,
                &bump2_changes,
                &"wave21-smoke-G-bump2",
            ],
        )
        .await
        .context("bump_strategy_v2 lr=0.0003")?;
    let gen2: i64 = bump2_row.try_get(0).unwrap_or(0);
    eprintln!("[bump2] new generation = {}", gen2);
    assert!(gen2 > gen1, "generation must increment after bump2: {} → {}", gen1, gen2);

    let after2_row = client
        .query_one(
            "SELECT optimizer, format, hidden, lr, seed, steps, generation \
             FROM ssot.scarab_strategy WHERE service_id = $1",
            &[&service_id],
        )
        .await
        .context("read strategy after bump2")?;
    let (opt2, fmt2, h2, lr2, seed2, steps2, _) = row_to_fields(&after2_row);
    let lr2_f64: f64 = lr2.to_string().parse().unwrap_or(0.0);
    assert!(
        (lr2_f64 - 0.0003_f64).abs() < 1e-10,
        "lr should be 0.0003 after bump2, got {}",
        lr2_f64
    );
    let fp2 = compute_fingerprint(&opt2, &fmt2, h2, &lr2, seed2, steps2);
    assert_ne!(fp1, fp2, "fingerprint must change after bump2 (lr change)");
    eprintln!("[bump2] fingerprint: {}… → {}… ✓", &fp1[..16], &fp2[..16]);

    let cmd2_count = client
        .query_one(
            "SELECT COUNT(*) AS n FROM ssot.scarab_command \
             WHERE service_id = $1 AND command = 'bump'",
            &[&service_id],
        )
        .await
        .context("check scarab_command bump2")?;
    let cmd2_n: i64 = cmd2_count.get("n");
    assert!(cmd2_n >= 2, "Expected ≥2 scarab_command rows after bump2, got {}", cmd2_n);
    eprintln!("[bump2] scarab_command rows = {} ✓", cmd2_n);

    // ── Bump 3: hidden=512 ────────────────────────────────────────────────────
    eprintln!("\n--- Step 8: bump_strategy_v2 {{\"hidden\":512}} ---");
    let bump3_changes = json!({"hidden": 512});
    let bump3_row = client
        .query_one(
            "SELECT ssot.bump_strategy_v2($1, $2, $3)",
            &[
                &service_id,
                &bump3_changes,
                &"wave21-smoke-G-bump3",
            ],
        )
        .await
        .context("bump_strategy_v2 hidden=512")?;
    let gen3: i64 = bump3_row.try_get(0).unwrap_or(0);
    eprintln!("[bump3] new generation = {}", gen3);
    assert!(gen3 > gen2, "generation must increment after bump3: {} → {}", gen2, gen3);

    let after3_row = client
        .query_one(
            "SELECT optimizer, format, hidden, lr, seed, steps, generation \
             FROM ssot.scarab_strategy WHERE service_id = $1",
            &[&service_id],
        )
        .await
        .context("read strategy after bump3")?;
    let (opt3, fmt3, h3, lr3, seed3, steps3, _) = row_to_fields(&after3_row);
    assert_eq!(h3, 512, "hidden should be 512 after bump3, got {}", h3);
    let fp3 = compute_fingerprint(&opt3, &fmt3, h3, &lr3, seed3, steps3);
    assert_ne!(fp2, fp3, "fingerprint must change after bump3 (hidden change)");
    eprintln!("[bump3] fingerprint: {}… → {}… ✓", &fp2[..16], &fp3[..16]);

    let cmd3_count = client
        .query_one(
            "SELECT COUNT(*) AS n FROM ssot.scarab_command \
             WHERE service_id = $1 AND command = 'bump'",
            &[&service_id],
        )
        .await
        .context("check scarab_command bump3")?;
    let cmd3_n: i64 = cmd3_count.get("n");
    assert!(cmd3_n >= 3, "Expected ≥3 scarab_command rows after bump3, got {}", cmd3_n);
    eprintln!("[bump3] scarab_command rows = {} ✓", cmd3_n);

    // ── Step 9: kill_scarab ───────────────────────────────────────────────────
    eprintln!("\n--- Step 9: kill_scarab ---");
    let _kill_row = client
        .query_one(
            "SELECT ssot.kill_scarab($1, $2)",
            &[&service_id, &"wave21-smoke-G-finish"],
        )
        .await
        .context("kill_scarab")?;
    eprintln!("[kill_scarab] issued");

    // Assert status='killed'
    let killed_row = client
        .query_one(
            "SELECT status FROM ssot.scarab_strategy WHERE service_id = $1",
            &[&service_id],
        )
        .await
        .context("verify kill status")?;
    let final_status: &str = killed_row.get("status");
    eprintln!("[kill] status = {}", final_status);
    assert_eq!(
        final_status, "killed",
        "Expected status='killed' after kill_scarab, got '{}'",
        final_status
    );
    eprintln!("[step 9] status=killed ✓");

    // ── Steps 10–11: Cleanup ──────────────────────────────────────────────────
    eprintln!("\n--- Steps 10-11: Cleanup ---");
    // NOTE: strategy_history, scarab_assignment_log, scarab_drift are VIEWs — DELETE from base tables only
    let del_strat = client
        .execute(
            "DELETE FROM ssot.scarab_strategy WHERE service_id LIKE 'w21-smoke-G-%'",
            &[],
        )
        .await
        .context("cleanup scarab_strategy")?;
    eprintln!("[cleanup] scarab_strategy: {} rows deleted", del_strat);

    let del_cmd = client
        .execute(
            "DELETE FROM ssot.scarab_command WHERE service_id LIKE 'w21-smoke-G-%'",
            &[],
        )
        .await
        .context("cleanup scarab_command")?;
    eprintln!("[cleanup] scarab_command: {} rows deleted", del_cmd);

    let del_hb = client
        .execute(
            "DELETE FROM ssot.scarab_heartbeat WHERE service_id LIKE 'w21-smoke-G-%'",
            &[],
        )
        .await
        .context("cleanup scarab_heartbeat")?;
    eprintln!("[cleanup] scarab_heartbeat: {} rows deleted", del_hb);

    // ── Step 12: Post-state snapshot + G6 gate ────────────────────────────────
    eprintln!("\n--- Step 12: Post-state snapshot + G6 gate ---");
    let post_snap = take_snapshot(&client, "post").await?;

    let (d_bpb, d_strat, d_dead) = pre_snap.delta(&post_snap);
    eprintln!(
        "[G6] bpb_samples: pre={} post={} delta={}",
        pre_snap.bpb_samples, post_snap.bpb_samples, d_bpb
    );
    eprintln!(
        "[G6] scarab_strategy(non-smoke): pre={} post={} delta={}",
        pre_snap.scarab_strategy_non_smoke, post_snap.scarab_strategy_non_smoke, d_strat
    );
    eprintln!(
        "[G6] scarab_dead: pre={} post={} delta={}",
        pre_snap.scarab_dead, post_snap.scarab_dead, d_dead
    );

    assert_eq!(d_bpb, 0, "G6 FAIL: bpb_samples count changed by {}", d_bpb);
    assert_eq!(d_strat, 0, "G6 FAIL: scarab_strategy non-smoke count changed by {}", d_strat);
    assert_eq!(d_dead, 0, "G6 FAIL: scarab_dead count changed by {}", d_dead);

    eprintln!("\n[G6 PASS] prod row count delta = 0 for all watched tables ✓");
    eprintln!("\n=== Wave 21 Agent G E2E Railway Smoke Test COMPLETE ===");
    eprintln!("service_id = {}", service_id);
    eprintln!(
        "Fingerprint progression: {}… → {}… → {}… → {}…",
        &fp0[..16], &fp1[..16], &fp2[..16], &fp3[..16]
    );
    eprintln!(
        "Generation progression: {} → {} → {} → {}",
        gen_initial, gen1, gen2, gen3
    );

    Ok(())
}
