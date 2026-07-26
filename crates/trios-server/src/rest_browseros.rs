//! Wave 6 (TS retirement): the last portable Hono routes from
//! `browseros/packages/browseros-agent/apps/server`, re-implemented in Rust.
//!
//! Wire-compatible with the TS server the BrowserOS UI already speaks to:
//!   GET/PUT  /memory        — core memory file ({content} / {success:true})
//!   GET/PUT  /soul          — SOUL.md with 150-line truncation contract
//!   CRUD     /skills        — SKILL.md dirs with YAML frontmatter (gray-matter)
//!   GET/PUT  /acl-rules     — global ACL rule list (acl-rules.json)
//!   GET      /status        — {status:"ok"}
//!   POST     /shutdown      — {status:"ok"} then graceful exit
//!   GET      /credits       — proxy to gateway (503 when unconfigured)
//!   POST     /test-provider — OpenAI-compatible provider connectivity probe
//!   POST     /refine-prompt — prompt refinement via the configured provider
//!   GET      /monitoring/runs, /monitoring/runs/:id — lazy-monitoring reader
//!   POST     /monitoring/debug/runs, .../:id/finalize — debug session writer
//!
//! Host-bound Hono routes (openclaw VM, terminal PTY, oauth, klavis, the
//! agent chat loop) are intentionally NOT ported here — they die with the TS
//! server or move behind dedicated crates later.

use axum::extract::{Path as AxumPath, Query};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ws_handler::AppState;

const MAX_CONTENT_LENGTH: usize = 50_000;
const SOUL_MAX_LINES: usize = 150;
const SKILL_NAME_MAX: usize = 100;
const SKILL_DESC_MAX: usize = 500;

// ---------------------------------------------------------------------------
// ~/.browseros path helpers (parity with lib/browseros-dir.ts)
// ---------------------------------------------------------------------------

pub fn browseros_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("BROWSEROS_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let name = if std::env::var("NODE_ENV").as_deref() == Ok("development") {
        ".browseros-dev"
    } else {
        ".browseros"
    };
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(name)
}

fn memory_dir() -> PathBuf {
    browseros_dir().join("memory")
}

fn core_memory_path() -> PathBuf {
    memory_dir().join("CORE.md")
}

fn soul_path() -> PathBuf {
    browseros_dir().join("SOUL.md")
}

fn skills_dir() -> PathBuf {
    browseros_dir().join("skills")
}

fn builtin_skills_dir() -> PathBuf {
    skills_dir().join("builtin")
}

fn acl_rules_path() -> PathBuf {
    browseros_dir().join("acl-rules.json")
}

fn lazy_runs_dir() -> PathBuf {
    browseros_dir().join("lazy-monitoring").join("runs")
}

type ApiResult = Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)>;

fn ok(v: Value) -> ApiResult {
    Ok((StatusCode::OK, Json(v)))
}

fn err(status: StatusCode, msg: &str) -> ApiResult {
    Err((status, Json(json!({ "error": msg }))))
}

// ---------------------------------------------------------------------------
// /memory — routes/memory.ts
// ---------------------------------------------------------------------------

async fn get_memory() -> ApiResult {
    let content = tokio::fs::read_to_string(core_memory_path())
        .await
        .unwrap_or_default();
    ok(json!({ "content": content }))
}

async fn put_memory(Json(body): Json<Value>) -> ApiResult {
    let Some(content) = body.get("content").and_then(Value::as_str) else {
        return err(StatusCode::BAD_REQUEST, "content must be a string");
    };
    if content.chars().count() > MAX_CONTENT_LENGTH {
        return err(StatusCode::BAD_REQUEST, "content too large");
    }
    if let Err(e) = tokio::fs::create_dir_all(memory_dir()).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    if let Err(e) = tokio::fs::write(core_memory_path(), content).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    ok(json!({ "success": true }))
}

// ---------------------------------------------------------------------------
// /soul — routes/soul.ts + lib/soul.ts (150-line truncation contract)
// ---------------------------------------------------------------------------

async fn get_soul() -> ApiResult {
    let content = tokio::fs::read_to_string(soul_path())
        .await
        .unwrap_or_default();
    ok(json!({ "content": content }))
}

async fn put_soul(Json(body): Json<Value>) -> ApiResult {
    let Some(content) = body.get("content").and_then(Value::as_str) else {
        return err(StatusCode::BAD_REQUEST, "content must be a string");
    };
    let lines: Vec<&str> = content.split('\n').collect();
    let kept = &lines[..lines.len().min(SOUL_MAX_LINES)];
    let dropped = &lines[lines.len().min(SOUL_MAX_LINES)..];
    if let Err(e) = tokio::fs::create_dir_all(browseros_dir()).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    if let Err(e) = tokio::fs::write(soul_path(), kept.join("\n")).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    ok(json!({
        "truncated": !dropped.is_empty(),
        "linesWritten": kept.len(),
        "linesDropped": dropped.len(),
        "droppedContent": dropped.join("\n"),
    }))
}

// ---------------------------------------------------------------------------
// /skills — routes/skills.ts + skills/service.ts + skills/loader.ts
// SKILL.md files carry gray-matter YAML frontmatter:
//   ---
//   name: <id>
//   description: <text>
//   metadata:
//     display-name: <text>
//     enabled: 'true' | 'false'
//     version: <text>
//   ---
//   <content>
// ---------------------------------------------------------------------------

pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true; // suppress leading dash
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Reject ids that could escape the skills dir (parity with safeSkillDir).
fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains("..")
        && id != "."
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2
        && ((v.starts_with('\'') && v.ends_with('\''))
            || (v.starts_with('"') && v.ends_with('"')))
    {
        let inner = &v[1..v.len() - 1];
        if v.starts_with('\'') {
            inner.replace("''", "'")
        } else {
            inner.replace("\\\"", "\"")
        }
    } else {
        v.to_string()
    }
}

fn yaml_quote(v: &str) -> String {
    format!("'{}'", v.replace('\'', "''"))
}

#[derive(Debug, Default, Clone)]
struct Frontmatter {
    top: HashMap<String, String>,
    metadata: HashMap<String, String>,
}

/// Minimal gray-matter-compatible parser for the flat skill frontmatter
/// schema (top-level string keys + one nested `metadata:` string map).
fn parse_frontmatter(raw: &str) -> Option<(Frontmatter, String)> {
    let rest = raw.strip_prefix("---")?;
    let rest = rest.strip_prefix("\r\n").or_else(|| rest.strip_prefix('\n'))?;
    // find closing delimiter at start of a line
    let mut fm_end = None;
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let t = line.trim_end();
        if t == "---" {
            fm_end = Some(offset);
            offset += line.len();
            break;
        }
        offset += line.len();
    }
    let fm_end = fm_end?;
    let fm_block = &rest[..fm_end];
    let content = rest[offset..].to_string();

    let mut fm = Frontmatter::default();
    let mut in_metadata = false;
    for line in fm_block.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        let trimmed = line.trim();
        let Some((k, v)) = trimmed.split_once(':') else {
            continue;
        };
        let key = k.trim().to_string();
        let val = unquote(v);
        if !indented {
            if key == "metadata" && v.trim().is_empty() {
                in_metadata = true;
            } else {
                in_metadata = false;
                fm.top.insert(key, val);
            }
        } else if in_metadata {
            fm.metadata.insert(key, val);
        }
    }
    if fm.top.get("name").map_or(true, |n| n.is_empty())
        || fm.top.get("description").map_or(true, |d| d.is_empty())
    {
        return None;
    }
    Some((fm, content))
}

fn build_skill_md(fm: &Frontmatter, content: &str) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!(
        "name: {}\n",
        yaml_quote(fm.top.get("name").map(String::as_str).unwrap_or(""))
    ));
    out.push_str(&format!(
        "description: {}\n",
        yaml_quote(fm.top.get("description").map(String::as_str).unwrap_or(""))
    ));
    if !fm.metadata.is_empty() {
        out.push_str("metadata:\n");
        let mut keys: Vec<&String> = fm.metadata.keys().collect();
        keys.sort();
        for k in keys {
            out.push_str(&format!("  {}: {}\n", k, yaml_quote(&fm.metadata[k])));
        }
    }
    out.push_str("---\n");
    out.push_str(content);
    if !content.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn skill_meta_json(id: &str, fm: &Frontmatter, location: &Path, built_in: bool) -> Value {
    let display = fm
        .metadata
        .get("display-name")
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| fm.top.get("name").cloned().unwrap_or_default());
    let mut meta = json!({
        "id": id,
        "name": display,
        "description": fm.top.get("description").cloned().unwrap_or_default(),
        "location": location.to_string_lossy(),
        "enabled": fm.metadata.get("enabled").map(String::as_str) != Some("false"),
        "builtIn": built_in,
    });
    if let Some(version) = fm.metadata.get("version") {
        meta["version"] = json!(version);
    }
    meta
}

async fn read_skill_dir(dir: &Path, id: &str) -> Option<(Frontmatter, String, PathBuf)> {
    let md_path = dir.join(id).join("SKILL.md");
    let raw = tokio::fs::read_to_string(&md_path).await.ok()?;
    let (fm, content) = parse_frontmatter(&raw)?;
    Some((fm, content, md_path))
}

/// Resolve a skill id to (frontmatter, content, path, builtIn), user dir first.
async fn resolve_skill(id: &str) -> Option<(Frontmatter, String, PathBuf, bool)> {
    if !is_safe_id(id) {
        return None;
    }
    if let Some((fm, c, p)) = read_skill_dir(&skills_dir(), id).await {
        return Some((fm, c, p, false));
    }
    if let Some((fm, c, p)) = read_skill_dir(&builtin_skills_dir(), id).await {
        return Some((fm, c, p, true));
    }
    None
}

async fn scan_skills(dir: &Path, built_in: bool, skip: Option<&str>) -> Vec<Value> {
    let mut out = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return out;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if Some(name.as_str()) == skip {
            continue;
        }
        let Ok(ft) = entry.file_type().await else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }
        if let Some((fm, _c, p)) = read_skill_dir(dir, &name).await {
            out.push(skill_meta_json(&name, &fm, &p, built_in));
        }
    }
    out
}

async fn list_skills() -> ApiResult {
    let mut skills = scan_skills(&builtin_skills_dir(), true, None).await;
    skills.extend(scan_skills(&skills_dir(), false, Some("builtin")).await);
    ok(json!({ "skills": skills }))
}

async fn get_skill(AxumPath(id): AxumPath<String>) -> ApiResult {
    let Some((fm, content, path, built_in)) = resolve_skill(&id).await else {
        return err(StatusCode::NOT_FOUND, "Skill not found");
    };
    let mut skill = skill_meta_json(&id, &fm, &path, built_in);
    skill["content"] = json!(content.trim());
    ok(json!({ "skill": skill }))
}

fn str_field(body: &Value, key: &str) -> Option<String> {
    body.get(key).and_then(Value::as_str).map(str::to_string)
}

async fn create_skill(Json(body): Json<Value>) -> ApiResult {
    let (Some(name), Some(description), Some(content)) = (
        str_field(&body, "name"),
        str_field(&body, "description"),
        str_field(&body, "content"),
    ) else {
        return err(StatusCode::BAD_REQUEST, "name, description and content are required");
    };
    if name.is_empty()
        || name.chars().count() > SKILL_NAME_MAX
        || description.is_empty()
        || description.chars().count() > SKILL_DESC_MAX
        || content.is_empty()
        || content.chars().count() > MAX_CONTENT_LENGTH
    {
        return err(StatusCode::BAD_REQUEST, "invalid field length");
    }
    let id = slugify(&name);
    if id.is_empty() || !is_safe_id(&id) {
        return err(StatusCode::BAD_REQUEST, "Invalid skill name");
    }
    for base in [skills_dir(), builtin_skills_dir()] {
        if tokio::fs::metadata(base.join(&id).join("SKILL.md")).await.is_ok() {
            return err(StatusCode::BAD_REQUEST, &format!("Skill \"{id}\" already exists"));
        }
    }
    let dir = skills_dir().join(&id);
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        return err(StatusCode::BAD_REQUEST, &e.to_string());
    }
    let mut fm = Frontmatter::default();
    fm.top.insert("name".into(), id.clone());
    fm.top.insert("description".into(), description.clone());
    fm.metadata.insert("display-name".into(), name.clone());
    fm.metadata.insert("enabled".into(), "true".into());
    let md_path = dir.join("SKILL.md");
    if let Err(e) = tokio::fs::write(&md_path, build_skill_md(&fm, &content)).await {
        return err(StatusCode::BAD_REQUEST, &e.to_string());
    }
    Ok((
        StatusCode::CREATED,
        Json(json!({ "skill": {
            "id": id,
            "name": name,
            "description": description,
            "location": md_path.to_string_lossy(),
            "enabled": true,
            "builtIn": false,
        }})),
    ))
}

async fn update_skill(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> ApiResult {
    for (key, max) in [("name", SKILL_NAME_MAX), ("description", SKILL_DESC_MAX), ("content", MAX_CONTENT_LENGTH)] {
        if let Some(v) = str_field(&body, key) {
            if v.chars().count() > max || (key != "content" && v.is_empty()) {
                return err(StatusCode::BAD_REQUEST, "invalid field length");
            }
        }
    }
    let Some((mut fm, old_content, path, built_in)) = resolve_skill(&id).await else {
        return err(StatusCode::NOT_FOUND, &format!("Skill \"{id}\" not found"));
    };
    let display = str_field(&body, "name").unwrap_or_else(|| {
        fm.metadata
            .get("display-name")
            .cloned()
            .unwrap_or_else(|| fm.top.get("name").cloned().unwrap_or_default())
    });
    let description = str_field(&body, "description")
        .unwrap_or_else(|| fm.top.get("description").cloned().unwrap_or_default());
    let content = str_field(&body, "content").unwrap_or_else(|| old_content.trim().to_string());
    let enabled = body
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| fm.metadata.get("enabled").map(String::as_str) != Some("false"));

    fm.top.insert("name".into(), id.clone());
    fm.top.insert("description".into(), description.clone());
    fm.metadata.insert("display-name".into(), display.clone());
    fm.metadata.insert("enabled".into(), enabled.to_string());

    if let Err(e) = tokio::fs::write(&path, build_skill_md(&fm, &content)).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    let mut skill = json!({
        "id": id,
        "name": display,
        "description": description,
        "location": path.to_string_lossy(),
        "enabled": enabled,
        "builtIn": built_in,
    });
    if let Some(version) = fm.metadata.get("version") {
        skill["version"] = json!(version);
    }
    ok(json!({ "skill": skill }))
}

async fn delete_skill(AxumPath(id): AxumPath<String>) -> ApiResult {
    let Some((_fm, _c, path, built_in)) = resolve_skill(&id).await else {
        return err(StatusCode::NOT_FOUND, &format!("Skill \"{id}\" not found"));
    };
    if built_in {
        return err(StatusCode::FORBIDDEN, "Cannot delete built-in skill");
    }
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    if let Err(e) = tokio::fs::remove_dir_all(&dir).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    ok(json!({ "ok": true }))
}

// ---------------------------------------------------------------------------
// /acl-rules — routes/acl.ts (file-backed global ACL policy)
// ---------------------------------------------------------------------------

async fn get_acl_rules() -> ApiResult {
    let rules: Value = match tokio::fs::read_to_string(acl_rules_path()).await {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| json!([])),
        Err(_) => json!([]),
    };
    ok(json!({ "aclRules": rules }))
}

async fn put_acl_rules(Json(body): Json<Value>) -> ApiResult {
    let Some(rules) = body.get("aclRules").and_then(Value::as_array) else {
        return err(StatusCode::BAD_REQUEST, "aclRules must be an array");
    };
    for rule in rules {
        let valid = rule.get("id").and_then(Value::as_str).is_some()
            && rule.get("sitePattern").and_then(Value::as_str).is_some()
            && rule.get("enabled").and_then(Value::as_bool).is_some();
        if !valid {
            return err(StatusCode::BAD_REQUEST, "invalid acl rule");
        }
    }
    if let Err(e) = tokio::fs::create_dir_all(browseros_dir()).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    let raw = serde_json::to_string_pretty(&Value::Array(rules.clone())).unwrap_or_default();
    if let Err(e) = tokio::fs::write(acl_rules_path(), raw).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    ok(json!({ "aclRules": rules }))
}

// ---------------------------------------------------------------------------
// /status + /shutdown — routes/status.ts, routes/shutdown.ts
// ---------------------------------------------------------------------------

async fn get_status() -> ApiResult {
    ok(json!({ "status": "ok" }))
}

async fn post_shutdown() -> ApiResult {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        tracing::info!("shutdown requested via POST /shutdown");
        std::process::exit(0);
    });
    ok(json!({ "status": "ok" }))
}

// ---------------------------------------------------------------------------
// /credits — routes/credits.ts (gateway proxy, 503 when unconfigured)
// ---------------------------------------------------------------------------

fn credits_config() -> Option<(String, String)> {
    let id = std::env::var("BROWSEROS_ID").ok().filter(|s| !s.trim().is_empty())?;
    let url = std::env::var("BROWSEROS_CONFIG_URL").ok().filter(|s| !s.trim().is_empty())?;
    let origin = reqwest::Url::parse(&url).ok().map(|u| u.origin().ascii_serialization())?;
    Some((origin, id))
}

async fn get_credits() -> ApiResult {
    let Some((origin, id)) = credits_config() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "Credits not configured");
    };
    let url = format!("{origin}/credits/{id}");
    match reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
            Ok(body) => ok(body),
            Err(_) => err(StatusCode::BAD_GATEWAY, "Failed to fetch credits"),
        },
        _ => err(StatusCode::BAD_GATEWAY, "Failed to fetch credits"),
    }
}

// ---------------------------------------------------------------------------
// /test-provider + /refine-prompt — OpenAI-compatible probe. The TS server
// fanned out through the Vercel AI SDK; the Rust port speaks the
// chat-completions dialect that every provider BrowserOS ships (openai,
// openrouter, ollama, lmstudio, zai, custom baseUrl) accepts.
// ---------------------------------------------------------------------------

pub(crate) fn provider_base_url(body: &Value) -> Option<String> {
    if let Some(u) = body
        .get("baseUrl")
        .or_else(|| body.get("base_url"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        return Some(u.trim_end_matches('/').to_string());
    }
    match body.get("provider").and_then(Value::as_str)? {
        "openai" => Some("https://api.openai.com/v1".into()),
        "openrouter" => Some("https://openrouter.ai/api/v1".into()),
        "ollama" => Some("http://127.0.0.1:11434/v1".into()),
        "lmstudio" | "lm_studio" => Some("http://127.0.0.1:1234/v1".into()),
        "zai" | "z.ai" => Some("https://api.z.ai/api/paas/v4".into()),
        _ => None,
    }
}

async fn chat_completion(body: &Value, messages: Value, max_tokens: u32) -> Result<String, String> {
    let provider = body.get("provider").and_then(Value::as_str).unwrap_or("unknown");
    let Some(model) = body.get("model").and_then(Value::as_str).filter(|s| !s.is_empty()) else {
        return Err("model is required".into());
    };
    let Some(base) = provider_base_url(body) else {
        return Err(format!("provider '{provider}' needs a baseUrl (OpenAI-compatible)"));
    };
    let mut req = reqwest::Client::new()
        .post(format!("{base}/chat/completions"))
        .timeout(std::time::Duration::from_secs(30))
        .json(&json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
        }));
    if let Some(key) = body
        .get("apiKey")
        .or_else(|| body.get("api_key"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        req = req.bearer_auth(key);
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let payload: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = payload
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("provider returned an error");
        return Err(format!("{} — {}", status, msg));
    }
    Ok(payload
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string())
}

async fn post_test_provider(Json(body): Json<Value>) -> ApiResult {
    let provider = body.get("provider").and_then(Value::as_str).unwrap_or("unknown").to_string();
    let start = std::time::Instant::now();
    let messages = json!([{ "role": "user", "content": "Respond with exactly: 'ok'" }]);
    match chat_completion(&body, messages, 16).await {
        Ok(text) => {
            let preview: String = text.chars().take(100).collect();
            ok(json!({
                "success": true,
                "message": format!("Connection successful. Response: \"{preview}\""),
                "responseTime": start.elapsed().as_millis() as u64,
            }))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": format!("[{provider}] {e}"),
                "responseTime": start.elapsed().as_millis() as u64,
            })),
        )),
    }
}

async fn post_refine_prompt(Json(body): Json<Value>) -> ApiResult {
    let prompt = body.get("prompt").and_then(Value::as_str).unwrap_or("");
    let name = body.get("name").and_then(Value::as_str).unwrap_or("");
    if prompt.is_empty() || name.is_empty() {
        return err(StatusCode::BAD_REQUEST, "prompt and name are required");
    }
    let provider = body.get("provider").and_then(Value::as_str).unwrap_or("unknown").to_string();
    let messages = json!([
        { "role": "system", "content": "You refine automation task prompts. Rewrite the user's prompt to be clear, specific and unambiguous. Reply with the refined prompt only." },
        { "role": "user", "content": format!("Task name: {name}\n\nPrompt:\n{prompt}") },
    ]);
    match chat_completion(&body, messages, 1024).await {
        Ok(refined) if !refined.is_empty() => ok(json!({ "success": true, "refined": refined })),
        Ok(_) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": format!("[{provider}] empty response") })),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": format!("[{provider}] {e}") })),
        )),
    }
}

// ---------------------------------------------------------------------------
// /monitoring — routes/monitoring.ts + monitoring/storage.ts (file-backed)
// ---------------------------------------------------------------------------

fn is_valid_run_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (i, b) in bytes.iter().enumerate() {
        match i {
            8 | 13 | 18 | 23 => {
                if *b != b'-' {
                    return false;
                }
            }
            14 => {
                if !(b'1'..=b'5').contains(b) {
                    return false;
                }
            }
            19 => {
                if !matches!(b.to_ascii_lowercase(), b'8' | b'9' | b'a' | b'b') {
                    return false;
                }
            }
            _ => {
                if !b.is_ascii_hexdigit() {
                    return false;
                }
            }
        }
    }
    true
}

async fn read_json_file(path: &Path) -> Option<Value> {
    let raw = tokio::fs::read_to_string(path).await.ok()?;
    serde_json::from_str(&raw).ok()
}

async fn read_tool_calls(run_dir: &Path) -> Vec<Value> {
    let Ok(raw) = tokio::fs::read_to_string(run_dir.join("tool-calls.jsonl")).await else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .collect()
}

async fn list_run_ids() -> Vec<String> {
    let mut runs: Vec<(String, std::time::SystemTime)> = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(lazy_runs_dir()).await else {
        return Vec::new();
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_valid_run_id(&name) {
            continue;
        }
        let Ok(meta) = entry.metadata().await else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }
        let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
        runs.push((name, mtime));
    }
    runs.sort_by(|a, b| b.1.cmp(&a.1));
    runs.into_iter().map(|(id, _)| id).collect()
}

async fn monitoring_list_runs(Query(params): Query<HashMap<String, String>>) -> ApiResult {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(50);
    let mut summaries = Vec::new();
    for run_id in list_run_ids().await.into_iter().take(limit) {
        let run_dir = lazy_runs_dir().join(&run_id);
        let Some(context) = read_json_file(&run_dir.join("context.json")).await else {
            continue;
        };
        let tool_calls = read_tool_calls(&run_dir).await;
        let finalization = read_json_file(&run_dir.join("finalization.json")).await;
        let mut summary = json!({
            "monitoringSessionId": context.get("monitoringSessionId"),
            "agentId": context.get("agentId"),
            "sessionKey": context.get("sessionKey"),
            "originalPrompt": context.get("originalPrompt"),
            "startedAt": context.get("startedAt"),
            "source": context.get("source"),
            "toolCallCount": tool_calls.len(),
        });
        if let Some(fin) = finalization {
            summary["finalization"] = json!({
                "status": fin.get("status"),
                "finalizedAt": fin.get("finalizedAt"),
                "error": fin.get("error"),
            });
        }
        summaries.push(summary);
    }
    ok(json!({ "runs": summaries }))
}

async fn monitoring_get_run(AxumPath(id): AxumPath<String>) -> ApiResult {
    if !is_valid_run_id(&id) {
        return err(StatusCode::BAD_REQUEST, "Invalid monitoring run id");
    }
    let run_dir = lazy_runs_dir().join(&id);
    let Some(context) = read_json_file(&run_dir.join("context.json")).await else {
        return err(StatusCode::NOT_FOUND, "Monitoring run not found");
    };
    let tool_calls = read_tool_calls(&run_dir).await;
    let finalization = read_json_file(&run_dir.join("finalization.json")).await;
    let mut envelope = json!({ "run": context, "toolCalls": tool_calls });
    if let Some(fin) = finalization {
        envelope["finalization"] = fin;
    }
    ok(json!({ "run": envelope }))
}

async fn monitoring_debug_start(Json(body): Json<Value>) -> ApiResult {
    for key in ["agentId", "sessionKey", "originalPrompt"] {
        if body.get(key).and_then(Value::as_str).map(str::trim).unwrap_or("").is_empty() {
            return err(StatusCode::BAD_REQUEST, &format!("{key} is required"));
        }
    }
    let chat_history: Vec<Value> = body
        .get("chatHistory")
        .and_then(Value::as_array)
        .map(|turns| {
            turns
                .iter()
                .filter(|t| {
                    matches!(t.get("role").and_then(Value::as_str), Some("user") | Some("assistant"))
                        && t.get("content").and_then(Value::as_str).is_some()
                })
                .map(|t| json!({ "role": t["role"], "content": t["content"] }))
                .collect()
        })
        .unwrap_or_default();
    let session = json!({
        "monitoringSessionId": uuid::Uuid::new_v4().to_string(),
        "agentId": body["agentId"].as_str().unwrap().trim(),
        "sessionKey": body["sessionKey"].as_str().unwrap().trim(),
        "originalPrompt": body["originalPrompt"].as_str().unwrap().trim(),
        "chatHistory": chat_history,
        "startedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "source": "debug",
    });
    let run_id = session["monitoringSessionId"].as_str().unwrap().to_string();
    let run_dir = lazy_runs_dir().join(&run_id);
    if let Err(e) = tokio::fs::create_dir_all(&run_dir).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    let raw = format!("{}\n", serde_json::to_string_pretty(&session).unwrap_or_default());
    if let Err(e) = tokio::fs::write(run_dir.join("context.json"), raw).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    Ok((StatusCode::CREATED, Json(json!({ "session": session }))))
}

async fn monitoring_debug_finalize(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> ApiResult {
    if !is_valid_run_id(&id) {
        return err(StatusCode::BAD_REQUEST, "Invalid monitoring run id");
    }
    for key in ["agentId", "sessionKey"] {
        if body.get(key).and_then(Value::as_str).map(str::trim).unwrap_or("").is_empty() {
            return err(StatusCode::BAD_REQUEST, &format!("{key} is required"));
        }
    }
    let status = body.get("status").and_then(Value::as_str).unwrap_or("");
    if !matches!(status, "completed" | "failed" | "aborted" | "incomplete") {
        return err(StatusCode::BAD_REQUEST, "status is invalid");
    }
    let run_dir = lazy_runs_dir().join(&id);
    let Some(context) = read_json_file(&run_dir.join("context.json")).await else {
        return err(StatusCode::NOT_FOUND, "Monitoring run not found");
    };
    let mut finalization = json!({
        "monitoringSessionId": id,
        "agentId": body["agentId"].as_str().unwrap().trim(),
        "sessionKey": body["sessionKey"].as_str().unwrap().trim(),
        "status": status,
        "finalizedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    });
    if let Some(msg) = body.get("finalAssistantMessage").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        finalization["finalAssistantMessage"] = json!(msg);
    }
    if let Some(e) = body.get("error").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        finalization["error"] = json!(e);
    }
    let raw = format!("{}\n", serde_json::to_string_pretty(&finalization).unwrap_or_default());
    if let Err(e) = tokio::fs::write(run_dir.join("finalization.json"), raw).await {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    let tool_calls = read_tool_calls(&run_dir).await;
    let envelope = json!({ "run": context, "toolCalls": tool_calls, "finalization": finalization });
    let raw = format!("{}\n", serde_json::to_string_pretty(&envelope).unwrap_or_default());
    let _ = tokio::fs::write(run_dir.join("audit-envelope.json"), raw).await;
    ok(json!({ "run": envelope }))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/memory", get(get_memory).put(put_memory))
        .route("/soul", get(get_soul).put(put_soul))
        .route("/skills", get(list_skills).post(create_skill))
        .route(
            "/skills/:id",
            get(get_skill).put(update_skill).delete(delete_skill),
        )
        .route("/acl-rules", get(get_acl_rules).put(put_acl_rules))
        .route("/status", get(get_status))
        .route("/shutdown", post(post_shutdown))
        .route("/credits", get(get_credits))
        .route("/test-provider", post(post_test_provider))
        .route("/refine-prompt", post(post_refine_prompt))
        .route("/monitoring/runs", get(monitoring_list_runs))
        .route("/monitoring/runs/:id", get(monitoring_get_run))
        .route("/monitoring/debug/runs", post(monitoring_debug_start))
        .route("/monitoring/debug/runs/:id/finalize", post(monitoring_debug_finalize))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use std::sync::Mutex;
    use tower::ServiceExt;

    // Env vars are process-global: serialize tests that repoint BROWSEROS_DIR.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn app() -> Router {
        Router::new()
            .merge(router())
            .with_state(AppState::new())
    }

    async fn call(app: &Router, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(match &body {
                Some(v) => Body::from(v.to_string()),
                None => Body::empty(),
            })
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let value = if bytes.is_empty() {
            json!(null)
        } else {
            serde_json::from_slice(&bytes).unwrap_or(json!(null))
        };
        (status, value)
    }

    #[tokio::test]
    async fn memory_soul_roundtrip_and_truncation() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("BROWSEROS_DIR", dir.path());
        let app = app();

        // memory: empty -> write -> read back
        let (s, b) = call(&app, "GET", "/memory", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["content"], "");
        let (s, b) = call(&app, "PUT", "/memory", Some(json!({"content": "# CORE\nhello"}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["success"], true);
        let (_, b) = call(&app, "GET", "/memory", None).await;
        assert_eq!(b["content"], "# CORE\nhello");
        // oversize rejected
        let big = "x".repeat(MAX_CONTENT_LENGTH + 1);
        let (s, _) = call(&app, "PUT", "/memory", Some(json!({"content": big}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);

        // soul: 150-line truncation contract
        let long: Vec<String> = (0..200).map(|i| format!("line {i}")).collect();
        let (s, b) = call(&app, "PUT", "/soul", Some(json!({"content": long.join("\n")}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["truncated"], true);
        assert_eq!(b["linesWritten"], 150);
        assert_eq!(b["linesDropped"], 50);
        assert!(b["droppedContent"].as_str().unwrap().starts_with("line 150"));
        let (_, b) = call(&app, "GET", "/soul", None).await;
        assert_eq!(b["content"].as_str().unwrap().lines().count(), 150);

        std::env::remove_var("BROWSEROS_DIR");
    }

    #[tokio::test]
    async fn skills_crud_lifecycle() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("BROWSEROS_DIR", dir.path());
        let app = app();

        // empty list
        let (s, b) = call(&app, "GET", "/skills", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["skills"].as_array().unwrap().len(), 0);

        // create -> slugified id, 201
        let (s, b) = call(&app, "POST", "/skills", Some(json!({
            "name": "My Test Skill",
            "description": "does things",
            "content": "# Steps\n1. do it",
        }))).await;
        assert_eq!(s, StatusCode::CREATED);
        assert_eq!(b["skill"]["id"], "my-test-skill");
        assert_eq!(b["skill"]["builtIn"], false);

        // duplicate -> 400
        let (s, _) = call(&app, "POST", "/skills", Some(json!({
            "name": "My Test Skill", "description": "x", "content": "y",
        }))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);

        // get -> frontmatter parsed back
        let (s, b) = call(&app, "GET", "/skills/my-test-skill", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["skill"]["name"], "My Test Skill");
        assert_eq!(b["skill"]["description"], "does things");
        assert_eq!(b["skill"]["content"], "# Steps\n1. do it");
        assert_eq!(b["skill"]["enabled"], true);

        // update: disable + rename
        let (s, b) = call(&app, "PUT", "/skills/my-test-skill", Some(json!({
            "enabled": false, "name": "Renamed",
        }))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["skill"]["enabled"], false);
        assert_eq!(b["skill"]["name"], "Renamed");

        // list shows one, disabled
        let (_, b) = call(&app, "GET", "/skills", None).await;
        let skills = b["skills"].as_array().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0]["enabled"], false);

        // built-in skill cannot be deleted
        let builtin = dir.path().join("skills/builtin/core-skill");
        std::fs::create_dir_all(&builtin).unwrap();
        std::fs::write(
            builtin.join("SKILL.md"),
            "---\nname: core-skill\ndescription: built in\nmetadata:\n  enabled: 'true'\n---\nbody\n",
        )
        .unwrap();
        let (s, _) = call(&app, "DELETE", "/skills/core-skill", None).await;
        assert_eq!(s, StatusCode::FORBIDDEN);

        // path traversal rejected
        let (s, _) = call(&app, "GET", "/skills/..%2F..%2Fetc", None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);

        // delete user skill
        let (s, b) = call(&app, "DELETE", "/skills/my-test-skill", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["ok"], true);
        let (s, _) = call(&app, "GET", "/skills/my-test-skill", None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);

        std::env::remove_var("BROWSEROS_DIR");
    }

    #[tokio::test]
    async fn acl_status_credits_and_monitoring() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("BROWSEROS_DIR", dir.path());
        std::env::remove_var("BROWSEROS_ID");
        std::env::remove_var("BROWSEROS_CONFIG_URL");
        let app = app();

        // acl: empty -> put -> get
        let (s, b) = call(&app, "GET", "/acl-rules", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["aclRules"].as_array().unwrap().len(), 0);
        let rules = json!({"aclRules": [{"id": "r1", "sitePattern": "*.example.com", "enabled": true}]});
        let (s, b) = call(&app, "PUT", "/acl-rules", Some(rules)).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["aclRules"][0]["id"], "r1");
        let (_, b) = call(&app, "GET", "/acl-rules", None).await;
        assert_eq!(b["aclRules"][0]["sitePattern"], "*.example.com");
        // invalid rule rejected
        let (s, _) = call(&app, "PUT", "/acl-rules", Some(json!({"aclRules": [{"id": 5}]}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);

        // status
        let (s, b) = call(&app, "GET", "/status", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["status"], "ok");

        // credits unconfigured -> 503
        let (s, b) = call(&app, "GET", "/credits", None).await;
        assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(b["error"], "Credits not configured");

        // monitoring: empty list -> debug start -> finalize -> read back
        let (s, b) = call(&app, "GET", "/monitoring/runs", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["runs"].as_array().unwrap().len(), 0);

        let (s, b) = call(&app, "POST", "/monitoring/debug/runs", Some(json!({
            "agentId": "agent-1",
            "sessionKey": "sess-1",
            "originalPrompt": "do the thing",
            "chatHistory": [
                {"role": "user", "content": "hi"},
                {"role": "bogus", "content": "dropped"},
            ],
        }))).await;
        assert_eq!(s, StatusCode::CREATED);
        let run_id = b["session"]["monitoringSessionId"].as_str().unwrap().to_string();
        assert!(is_valid_run_id(&run_id));
        assert_eq!(b["session"]["chatHistory"].as_array().unwrap().len(), 1);

        let (s, _) = call(&app, "POST", &format!("/monitoring/debug/runs/{run_id}/finalize"), Some(json!({
            "agentId": "agent-1", "sessionKey": "sess-1", "status": "completed",
        }))).await;
        assert_eq!(s, StatusCode::OK);

        let (s, b) = call(&app, "GET", &format!("/monitoring/runs/{run_id}"), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(b["run"]["run"]["agentId"], "agent-1");
        assert_eq!(b["run"]["finalization"]["status"], "completed");

        let (_, b) = call(&app, "GET", "/monitoring/runs", None).await;
        let runs = b["runs"].as_array().unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0]["finalization"]["status"], "completed");

        // invalid run id -> 400; unknown valid id -> 404
        let (s, _) = call(&app, "GET", "/monitoring/runs/not-a-uuid", None).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        let (s, _) = call(&app, "GET", "/monitoring/runs/11111111-1111-4111-8111-111111111111", None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);

        std::env::remove_var("BROWSEROS_DIR");
    }

    #[tokio::test]
    async fn provider_and_refine_validation() {
        // no BROWSEROS_DIR needed; pure validation paths
        let app = app();

        // unknown provider without baseUrl -> 400 {success:false}
        let (s, b) = call(&app, "POST", "/test-provider", Some(json!({
            "provider": "mystery", "model": "m-1",
        }))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(b["success"], false);
        assert!(b["message"].as_str().unwrap().starts_with("[mystery]"));

        // refine-prompt without prompt/name -> 400
        let (s, _) = call(&app, "POST", "/refine-prompt", Some(json!({
            "provider": "openai", "model": "gpt-x",
        }))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn slugify_and_frontmatter_roundtrip() {
        assert_eq!(slugify("My Test Skill"), "my-test-skill");
        assert_eq!(slugify("  Éxo!! 42 "), "xo-42");
        assert_eq!(slugify("---"), "");

        let mut fm = Frontmatter::default();
        fm.top.insert("name".into(), "demo".into());
        fm.top.insert("description".into(), "it's got 'quotes'".into());
        fm.metadata.insert("display-name".into(), "Demo".into());
        fm.metadata.insert("enabled".into(), "true".into());
        let md = build_skill_md(&fm, "body line\n");
        let (parsed, content) = parse_frontmatter(&md).unwrap();
        assert_eq!(parsed.top["name"], "demo");
        assert_eq!(parsed.top["description"], "it's got 'quotes'");
        assert_eq!(parsed.metadata["display-name"], "Demo");
        assert_eq!(content, "body line\n");

        // gray-matter (unquoted) files parse too
        let raw = "---\nname: legacy\ndescription: plain text\nmetadata:\n  display-name: Legacy Skill\n  enabled: 'false'\n---\ncontent here\n";
        let (parsed, content) = parse_frontmatter(raw).unwrap();
        assert_eq!(parsed.top["name"], "legacy");
        assert_eq!(parsed.metadata["display-name"], "Legacy Skill");
        assert_eq!(parsed.metadata["enabled"], "false");
        assert_eq!(content, "content here\n");

        // missing required fields -> None
        assert!(parse_frontmatter("---\nname: only-name\n---\nx").is_none());
        assert!(parse_frontmatter("no frontmatter").is_none());
    }

    #[test]
    fn run_id_validation() {
        assert!(is_valid_run_id("11111111-1111-4111-8111-111111111111"));
        assert!(is_valid_run_id("ABCDEF01-2345-1678-9ABC-DEF012345678"));
        assert!(!is_valid_run_id("11111111-1111-6111-8111-111111111111")); // version 6
        assert!(!is_valid_run_id("11111111-1111-4111-7111-111111111111")); // variant 7
        assert!(!is_valid_run_id("../../../etc/passwd"));
        assert!(!is_valid_run_id(""));
    }
}
