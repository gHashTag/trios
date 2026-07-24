# Skill: SSRF Path Validation in Rust

## When to use

When a Rust web server (axum/tower) accepts filesystem paths from HTTP requests and uses them with `WalkDir`, `fs::read`, or similar operations.

## Why

- Unvalidated paths enable directory traversal (information disclosure)
- An attacker can read arbitrary files (`repo_root: "../../etc"`)
- SSRF via path parameters can reach internal endpoints

## Steps

1. **Canonicalize the path:**
   ```rust
   let canonical = path.canonicalize()
       .map_err(|e| format!("Invalid path: {}", e))?;
   ```

2. **Bind to a safe base directory:**
   ```rust
   let cwd = std::env::current_dir()?;
   let cwd_canonical = cwd.canonicalize()?;
   if !canonical.starts_with(&cwd_canonical) {
       return Err("Path escapes working directory".into());
   }
   ```

3. **Reject injection characters:**
   ```rust
   if path.contains('\0') {
       return Err("Path contains null bytes".into());
   }
   ```

4. **For proxy paths, reject traversal sequences:**
   ```rust
   if clean_path.contains("..") || clean_path.contains('\0') {
       return (StatusCode::BAD_REQUEST, "Invalid path characters").into_response();
   }
   ```

5. **Return BAD_REQUEST on validation failure** (not 500 — the client sent bad input)

## Related

- [[server-feature-compilation-fix]] — Fixing `--all-features` compilation errors
- [[zero-all-features-clippy]] — Maintaining zero warnings under `--all-features`
