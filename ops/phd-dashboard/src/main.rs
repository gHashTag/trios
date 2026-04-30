use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Html},
    routing::{get, post},
    Router,
};
use deadpool_postgres::{Config, Pool, Runtime};
use postgres_native_tls::MakeTlsConnector;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_postgres::NoTls;

#[derive(Clone)]
struct AppState {
    pool: Pool,
}

#[derive(Serialize, Deserialize)]
struct Chapter {
    ch_num: String,
    title: String,
    status: String,
    issue_url: Option<String>,
    word_count: Option<i32>,
    theorems_count: Option<i32>,
    evidence_axis: Option<i32>,
    priority: Option<String>,
    body_pdf_url: Option<String>,
    last_compiled: Option<chrono::DateTime<chrono::Utc>>,
    compile_ok: Option<bool>,
}

#[derive(Serialize)]
struct OneShot {
    id: i32,
    ch_num: String,
    directive: String,
    status: String,
    progress_pct: i32,
    claimed_by: Option<String>,
    heartbeat_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
struct ClaimReq {
    agent: String,
}

#[derive(Deserialize)]
struct HeartbeatReq {
    agent: String,
    status: String,
    message: Option<String>,
    progress_pct: Option<i32>,
}

#[derive(Deserialize)]
struct CompleteReq {
    agent: String,
    pr_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Pooled tokio-postgres + deadpool (sqlx REJECTED on Neon serverless per session contract)
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://neondb_owner:npg_NHBC5hdbM0Kx@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require".to_string());
    let mut cfg = Config::new();
    cfg.url = Some(database_url);
    let connector = native_tls::TlsConnector::builder().build()?;
    let tls = MakeTlsConnector::new(connector);
    let pool = cfg.create_pool(Some(Runtime::Tokio1), tls)?;

    let state = AppState { pool };

    let app = Router::new()
        .route("/", get(home))
        .route("/api/chapters", get(get_chapters))
        .route("/api/seeds", get(get_seeds))
        .route("/api/theorems", get(get_theorems))
        .route("/api/oneshots", get(get_oneshots))
        .route("/api/oneshot/:ch", get(get_oneshot))
        .route("/api/status", get(get_status))
        .route("/api/available", get(get_available))
        .route("/api/runs", get(get_runs))
        .route("/claim/:ch", post(claim))
        .route("/heartbeat/:id", post(heartbeat))
        .route("/complete/:id", post(complete))
        .route("/compile/:ch", post(compile_one))
        .route("/compile/all", post(compile_all))
        .route("/tex/:ch", get(get_tex))
        .route("/tex/full", get(get_tex_full))
        .route("/pdf/:ch", get(get_pdf))
        .route("/pdf/full", get(get_pdf_full))
        .with_state(state);

    let port: u16 = std::env::var("PORT").unwrap_or_else(|_| "5000".to_string()).parse()?;
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("phd-dashboard listening on :{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn home() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn get_chapters(State(s): State<AppState>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let rows = client.query("SELECT ch_num, title, status, issue_url, word_count, theorems_count, evidence_axis, priority, body_pdf_url, last_compiled, compile_ok FROM ssot.chapters ORDER BY ch_num", &[]).await.unwrap();
    let chapters: Vec<Chapter> = rows.iter().map(|r| Chapter {
        ch_num: r.get(0), title: r.get(1), status: r.get(2),
        issue_url: r.get(3), word_count: r.get(4), theorems_count: r.get(5),
        evidence_axis: r.get(6), priority: r.get(7), body_pdf_url: r.get(8),
        last_compiled: r.get(9), compile_ok: r.get(10),
    }).collect();
    Json(chapters)
}

async fn get_seeds(State(s): State<AppState>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let rows = client.query("SELECT seed_name, seed_type, repo, ref_url, status FROM ssot.seeds ORDER BY phi_weight DESC NULLS LAST", &[]).await.unwrap();
    let out: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "seed_name": r.get::<_,String>(0), "seed_type": r.get::<_,Option<String>>(1),
        "repo": r.get::<_,Option<String>>(2), "ref_url": r.get::<_,Option<String>>(3),
        "status": r.get::<_,Option<String>>(4),
    })).collect();
    Json(out)
}

async fn get_theorems(State(s): State<AppState>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let rows = client.query("SELECT name, canonical_file, qed_status, inv_num FROM ssot.theorems ORDER BY canonical_file, name LIMIT 1000", &[]).await.unwrap();
    let out: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "name": r.get::<_,String>(0), "file": r.get::<_,String>(1),
        "status": r.get::<_,String>(2), "inv": r.get::<_,Option<String>>(3),
    })).collect();
    Json(out)
}

async fn get_oneshots(State(s): State<AppState>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let rows = client.query("SELECT os.id, c.ch_num, os.directive, os.status, os.progress_pct, os.claimed_by, os.heartbeat_at FROM ssot.one_shots os JOIN ssot.chapters c ON c.id=os.chapter_id ORDER BY c.ch_num", &[]).await.unwrap();
    let out: Vec<OneShot> = rows.iter().map(|r| OneShot {
        id: r.get(0), ch_num: r.get(1), directive: r.get(2), status: r.get(3),
        progress_pct: r.get(4), claimed_by: r.get(5), heartbeat_at: r.get(6),
    }).collect();
    Json(out)
}

async fn get_oneshot(State(s): State<AppState>, Path(ch): Path<String>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let row = client.query_opt("SELECT os.id, c.ch_num, os.directive, os.status, os.progress_pct, os.claimed_by, os.heartbeat_at, os.preconditions, os.deliverables FROM ssot.one_shots os JOIN ssot.chapters c ON c.id=os.chapter_id WHERE c.ch_num=$1", &[&ch]).await.unwrap();
    if let Some(r) = row {
        let v = serde_json::json!({
            "id": r.get::<_,i32>(0), "ch_num": r.get::<_,String>(1),
            "directive": r.get::<_,String>(2), "status": r.get::<_,String>(3),
            "progress_pct": r.get::<_,i32>(4), "claimed_by": r.get::<_,Option<String>>(5),
            "heartbeat_at": r.get::<_,Option<chrono::DateTime<chrono::Utc>>>(6),
            "preconditions": r.get::<_,Option<serde_json::Value>>(7),
            "deliverables": r.get::<_,Option<serde_json::Value>>(8),
        });
        Json(v).into_response()
    } else {
        (StatusCode::NOT_FOUND, "chapter not found").into_response()
    }
}

async fn get_status(State(s): State<AppState>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let _ = client.execute("SELECT ssot.release_stale_locks(10)", &[]).await;
    let rows = client.query(
        "SELECT c.ch_num, os.status, os.claimed_by, os.progress_pct, os.heartbeat_at FROM ssot.one_shots os JOIN ssot.chapters c ON c.id=os.chapter_id WHERE os.claimed_by IS NOT NULL ORDER BY os.heartbeat_at DESC NULLS LAST", &[]).await.unwrap();
    let out: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "ch_num": r.get::<_,String>(0), "status": r.get::<_,String>(1),
        "agent": r.get::<_,Option<String>>(2), "pct": r.get::<_,i32>(3),
        "heartbeat_at": r.get::<_,Option<chrono::DateTime<chrono::Utc>>>(4),
    })).collect();
    Json(out)
}

async fn get_available(State(s): State<AppState>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let _ = client.execute("SELECT ssot.release_stale_locks(10)", &[]).await;
    let rows = client.query(
        "SELECT c.ch_num, c.title, c.priority, os.id FROM ssot.one_shots os JOIN ssot.chapters c ON c.id=os.chapter_id WHERE os.status='pending' AND os.claimed_by IS NULL ORDER BY c.priority NULLS LAST, c.ch_num", &[]).await.unwrap();
    let out: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "ch_num": r.get::<_,String>(0), "title": r.get::<_,String>(1),
        "priority": r.get::<_,Option<String>>(2), "oneshot_id": r.get::<_,i32>(3),
    })).collect();
    Json(out)
}

async fn get_runs(State(s): State<AppState>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let rows = client.query("SELECT ar.id, c.ch_num, ar.agent_id, ar.started_at, ar.finished_at, ar.pr_url, ar.ci_green FROM ssot.agent_runs ar LEFT JOIN ssot.one_shots os ON os.id=ar.one_shot_id LEFT JOIN ssot.chapters c ON c.id=os.chapter_id ORDER BY ar.started_at DESC LIMIT 50", &[]).await.unwrap();
    let out: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "id": r.get::<_,i32>(0), "ch_num": r.get::<_,Option<String>>(1),
        "agent": r.get::<_,Option<String>>(2),
        "started_at": r.get::<_,Option<chrono::DateTime<chrono::Utc>>>(3),
        "finished_at": r.get::<_,Option<chrono::DateTime<chrono::Utc>>>(4),
        "pr_url": r.get::<_,Option<String>>(5), "ci_green": r.get::<_,Option<bool>>(6),
    })).collect();
    Json(out)
}

async fn claim(State(s): State<AppState>, Path(ch): Path<String>, Json(req): Json<ClaimReq>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let row = client.query_one("SELECT * FROM ssot.claim_one_shot($1, $2)", &[&ch, &req.agent]).await.unwrap();
    let id: Option<i32> = row.get(0);
    let directive: Option<String> = row.get(1);
    let success: bool = row.get(2);
    Json(serde_json::json!({"id": id, "directive": directive, "success": success}))
}

async fn heartbeat(State(s): State<AppState>, Path(id): Path<i32>, Json(req): Json<HeartbeatReq>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let row = client.query_one("SELECT ssot.heartbeat($1, $2, $3, $4, $5)",
        &[&id, &req.agent, &req.status, &req.message, &req.progress_pct]).await.unwrap();
    let ok: bool = row.get(0);
    Json(serde_json::json!({"ok": ok}))
}

async fn complete(State(s): State<AppState>, Path(id): Path<i32>, Json(req): Json<CompleteReq>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    let row = client.query_one("SELECT ssot.complete_one_shot($1, $2, $3)",
        &[&id, &req.agent, &req.pr_url]).await.unwrap();
    let ok: bool = row.get(0);
    Json(serde_json::json!({"ok": ok}))
}

async fn compile_one(State(_): State<AppState>, Path(ch): Path<String>) -> impl IntoResponse {
    // Spawn python compiler for this chapter
    let out = std::process::Command::new("python3")
        .args(&["/app/compile.py","--chapter",&ch])
        .output();
    match out {
        Ok(o) => Json(serde_json::json!({
            "ok": o.status.success(),
            "stdout": String::from_utf8_lossy(&o.stdout),
            "stderr": String::from_utf8_lossy(&o.stderr),
            "pdf_url": format!("/pdf/{ch}"),
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("compile failed: {e}")).into_response(),
    }
}

async fn compile_all(State(_): State<AppState>) -> impl IntoResponse {
    let out = std::process::Command::new("python3").args(&["/app/compile.py"]).output();
    match out {
        Ok(o) => Json(serde_json::json!({
            "ok": o.status.success(),
            "pdf_url": "/pdf/full",
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("compile failed: {e}")).into_response(),
    }
}

async fn get_tex(State(s): State<AppState>, Path(ch): Path<String>) -> impl IntoResponse {
    let client = s.pool.get().await.unwrap();
    if let Some(r) = client.query_opt("SELECT body_tex FROM ssot.chapters WHERE ch_num=$1", &[&ch]).await.unwrap() {
        let tex: Option<String> = r.get(0);
        match tex {
            Some(t) => ([("content-type", "application/x-tex")], t).into_response(),
            None => (StatusCode::NOT_FOUND, "no tex; run /compile/:ch first").into_response(),
        }
    } else { (StatusCode::NOT_FOUND, "chapter not found").into_response() }
}

async fn get_tex_full(State(_): State<AppState>) -> impl IntoResponse {
    match std::fs::read_to_string("/app/build/main.tex") {
        Ok(t) => ([("content-type","application/x-tex")], t).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "main.tex not built; run /compile/all").into_response(),
    }
}

async fn get_pdf(State(_): State<AppState>, Path(ch): Path<String>) -> impl IntoResponse {
    let p = format!("/app/build/ch_{}.pdf", ch.replace('.','_').to_lowercase());
    match std::fs::read(&p) {
        Ok(b) => ([("content-type","application/pdf")], b).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "PDF not built for this chapter").into_response(),
    }
}

async fn get_pdf_full(State(_): State<AppState>) -> impl IntoResponse {
    match std::fs::read("/app/build/main.pdf") {
        Ok(b) => ([("content-type","application/pdf")], b).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "main.pdf not built; run POST /compile/all").into_response(),
    }
}
