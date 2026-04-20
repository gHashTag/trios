use crate::ws_handler::AppState;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub async fn read(_state: &AppState, params: Option<Value>) -> Value {
    let params = params.unwrap_or(json!({}));
    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let experience_dir = Path::new(".trinity/experience");
    if !experience_dir.exists() {
        return json!([]);
    }

    let mut entries: Vec<String> = Vec::new();

    let read_dir = match fs::read_dir(experience_dir) {
        Ok(d) => d,
        Err(_) => return json!([]),
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Ok(content) = fs::read_to_string(&path) {
                entries.push(content);
            }
        }
    }

    entries.truncate(limit);
    json!(entries)
}
