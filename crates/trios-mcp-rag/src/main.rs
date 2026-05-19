use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const DSN: &str = "postgresql://postgres:eqARhdezhmbRICINRvKuvRGKUbStobad@trolley.proxy.rlwy.net:52162/railway";

fn query_rows(sql: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> anyhow::Result<Vec<tokio_postgres::Row>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let (client, conn) = tokio_postgres::connect(DSN, tokio_postgres::NoTls).await?;
        tokio::spawn(async move { let _ = conn.await; });
        let rows = client.query(sql, params).await?;
        Ok(rows)
    })
}

fn rs(r: &tokio_postgres::Row, col: &str) -> String { r.get::<_, String>(col) }
fn ri(r: &tokio_postgres::Row, col: &str) -> i32 { r.get::<_, i32>(col) }

fn tool_search(query: &str, limit: usize) -> anyhow::Result<Value> {
    let pattern = format!("%{}%", query.replace('%', "\\%"));
    let rows = query_rows(
        "SELECT slug, kind, title, LEFT(body_md, 500) AS snippet \
         FROM ssot_brochure.chapters \
         WHERE body_md ILIKE $1 OR title ILIKE $1 \
         ORDER BY order_key LIMIT $2",
        &[&pattern as &(dyn tokio_postgres::types::ToSql + Sync), &(limit as i64) as &(dyn tokio_postgres::types::ToSql + Sync)],
    )?;
    let out: Vec<Value> = rows.iter().map(|r| json!({
        "slug": rs(r,"slug"), "kind": rs(r,"kind"), "title": rs(r,"title"), "snippet": rs(r,"snippet")
    })).collect();
    Ok(json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out)?}],"isError":false}))
}

fn tool_get(slug: &str) -> anyhow::Result<Value> {
    let rows = query_rows(
        "SELECT slug, kind, order_key, title, body_md, word_count FROM ssot_brochure.chapters WHERE slug=$1",
        &[&slug],
    )?;
    if rows.is_empty() {
        return Ok(json!({"content":[{"type":"text","text":format!("Chapter '{}' not found",slug)}],"isError":true}));
    }
    let r = &rows[0];
    Ok(json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&json!({
        "slug":rs(r,"slug"),"kind":rs(r,"kind"),"order_key":ri(r,"order_key"),
        "title":rs(r,"title"),"word_count":ri(r,"word_count"),"body_md":rs(r,"body_md")
    }))?}],"isError":false}))
}

fn tool_list() -> anyhow::Result<Value> {
    let rows = query_rows(
        "SELECT slug, kind, order_key, title, word_count FROM ssot_brochure.chapters ORDER BY order_key",
        &[],
    )?;
    let out: Vec<Value> = rows.iter().map(|r| json!({
        "slug":rs(r,"slug"),"kind":rs(r,"kind"),"order_key":ri(r,"order_key"),
        "title":rs(r,"title"),"words":ri(r,"word_count")
    })).collect();
    Ok(json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out)?}],"isError":false}))
}

fn tool_audit() -> anyhow::Result<Value> {
    let forbidden = ["63 tok/s/W","tok/s/W","compete with nvidia","vasilev-pellis constants","claude","rollback"];
    let rows = query_rows("SELECT slug, body_md FROM ssot_brochure.chapters", &[])?;
    let mut hits = Vec::new();
    for r in &rows {
        let s = rs(r,"slug"); let b = rs(r,"body_md").to_lowercase();
        for t in &forbidden { if b.contains(t) { hits.push(json!({"slug":s,"term":t})); } }
    }
    let msg = if hits.is_empty() { "CLEAN".into() } else { format!("{} violations: {}", hits.len(), serde_json::to_string(&hits)?) };
    Ok(json!({"content":[{"type":"text","text":msg}],"isError":false}))
}

fn tool_cover() -> anyhow::Result<Value> {
    let rows = query_rows("SELECT count(*) AS n, sum(word_count) AS tw FROM ssot_brochure.chapters", &[])?;
    let n: i64 = rows[0].get::<_, i64>("n");
    let tw: i64 = rows[0].get::<_, Option<i64>>("tw").unwrap_or(0);
    let tex = format!(r"\begin{{titlepage}}\centering\vspace*{{2cm}}\n{{\Huge\bfseries GOLDEN BRIDGE}}\\[0.5em]\n{{\Large A Three-Strand Compendium on $\varphi$-Structured Physical Constants and Ternary Silicon}}\\[2em]\n{{\large Dmitrii Vasilev \\ Stergios Pellis \\ Kenneth Olsen}}\\[1em]\n{{\large April 2026 \\ v27}}\\[2em]\n\rule{{\textwidth}}{{0.4pt}}\\[1em]\n{{\\textit{{Railway PostgreSQL SSOT}}}}\\[0.5em]\n{{\\texttt{{{n} chapters, {tw} words}}}}\\[2em]\n\\rule{{\\textwidth}}{{0.4pt}}\\[2em]\n{{\\small $\\varphi^2 + \\varphi^{{-2}} = 3$ (Coq Qed) \\quad 0x47C0 silicon anchor (Theorem 36.1)}}\\[0.5em]\n{{\\small $\\sim$1 GOPS @ $\\sim$50 MHz @ $\\sim$1 W ternary (projected)}}\\[3em]\n\\end{{titlepage}}\n");
    Ok(json!({"content":[{"type":"text","text":tex}],"isError":false}))
}

fn tools_def() -> Value {
    json!([
        {"name":"search_chapters","description":"Full-text search across all 80 GOLDEN BRIDGE chapters in Railway SSOT",
         "inputSchema":{"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer","default":10}},"required":["query"]}},
        {"name":"get_chapter","description":"Fetch full chapter by slug",
         "inputSchema":{"type":"object","properties":{"slug":{"type":"string"}},"required":["slug"]}},
        {"name":"list_chapters","description":"List all chapter slugs with metadata",
         "inputSchema":{"type":"object","properties":{}}},
        {"name":"forbidden_audit","description":"Scan all chapters for policy violations",
         "inputSchema":{"type":"object","properties":{}}},
        {"name":"build_cover","description":"Generate LaTeX titlepage for GOLDEN BRIDGE v27",
         "inputSchema":{"type":"object","properties":{}}}
    ])
}

fn dispatch(method: &str, id: &serde_json::Value, params: &serde_json::Value) -> serde_json::Value {
    match method {
        "initialize" => json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"trios-mcp-rag","version":env!("CARGO_PKG_VERSION")}}),
        "notifications/initialized" => json!(null),
        "tools/list" => json!({"tools":tools_def()}),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let res = match name {
                "search_chapters" => tool_search(args.get("query").and_then(|v|v.as_str()).unwrap_or(""), args.get("limit").and_then(|v|v.as_u64()).unwrap_or(10) as usize),
                "get_chapter" => tool_get(args.get("slug").and_then(|v|v.as_str()).unwrap_or("")),
                "list_chapters" => tool_list(),
                "forbidden_audit" => tool_audit(),
                "build_cover" => tool_cover(),
                _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
            };
            res.unwrap_or_else(|e| json!({"content":[{"type":"text","text":e.to_string()}],"isError":true}))
        }
        _ => json!({"content":[{"type":"text","text":format!("Unknown method: {}", method)}],"isError":true}),
    }
}

fn main() -> anyhow::Result<()> {
    eprintln!("trios-mcp-rag: stdio JSON-RPC server (Railway SSOT)");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut locked = stdin.lock();
    loop {
        let mut line = String::new();
        let n = locked.read_line(&mut line)?;
        if n == 0 { break; }
        let line = line.trim();
        if line.is_empty() { continue; }
        let req: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => { eprintln!("parse error: {}", e); continue; }
        };
        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = req.get("id").cloned().unwrap_or(json!(null));
        if id.is_null() && method.starts_with("notifications/") { continue; }
        let params = req.get("params").cloned().unwrap_or(json!({}));
        let result = dispatch(method, &id, &params);
        let resp = json!({"jsonrpc":"2.0","id":id,"result":result});
        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
        stdout.flush()?;
    }
    Ok(())
}
