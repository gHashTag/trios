# Skill: Axum Auth Middleware

## When to use

When building a Rust HTTP server with Axum that needs JWT or API-key authentication on all routes except a few public ones.

## Why

- Public compiler/server endpoints are a security vulnerability
- Axum's `middleware::from_fn` allows blanket auth across all routes with selective whitelisting
- Environment-variable bypass (`T27C_NO_AUTH=1`) enables CI/local testing without tokens

## Steps

1. **Implement the middleware function:**
   ```rust
   async fn require_auth_middleware(
       State(state): State<Arc<AppState>>,
       req: Request,
       next: Next,
   ) -> Result<Response, StatusCode> {
       let path = req.uri().path();
       // 1. Bypass for local/CI testing
       if std::env::var("T27C_NO_AUTH").is_ok_and(|v| v == "1") {
           return Ok(next.run(req).await);
       }
       // 2. Whitelist public routes
       if path.starts_with("/instance") || path.starts_with("/provider/auth") {
           return Ok(next.run(req).await);
       }
       // 3. Validate Authorization header
       if let Some(auth) = req.headers().get("Authorization") {
           if let Ok(s) = auth.to_str() {
               if s.starts_with("Bearer ") {
                   let token = &s[7..];
                   if validate_jwt(token, &state.jwt_secret) { return Ok(next.run(req).await); }
               } else if s.starts_with("ApiKey ") {
                   let key = &s[7..];
                   if validate_api_key(key, &state.api_keys) { return Ok(next.run(req).await); }
               }
           }
       }
       // 4. Reject with structured JSON error
       let body = Json(json!({"error":"Unauthorized","message":"Valid JWT or API key required"}));
       Ok((StatusCode::UNAUTHORIZED, body).into_response())
   }
   ```

2. **Apply to the router:**
   ```rust
   let app = Router::new()
       .route("/compile", post(compile_handler))
       .route("/gen", post(gen_handler))
       // ... other routes ...
       .layer(middleware::from_fn(require_auth_middleware));
   ```

3. **Add JWT validation helper:**
   ```rust
   fn validate_jwt(token: &str, secret: &str) -> bool {
       jsonwebtoken::decode::<Claims>(
           token,
           &DecodingKey::from_secret(secret.as_bytes()),
           &Validation::new(Algorithm::HS256),
       ).is_ok()
   }
   ```

4. **Test bypass:**
   ```bash
   T27C_NO_AUTH=1 cargo test --workspace --all-features
   ```

## Common Pitfalls

- **Return type mismatch:** Middleware must return `Response`, not a tuple. Use `.into_response()`.
- **Token expiry:** Add `Validation::new(Algorithm::HS256).set_required_spec_claims(&["exp"])` to enforce expiry.
- **Timing attacks:** Use constant-time comparison for API keys (`subtle::ConstantTimeEq`).

## Related

- [[ssrf-path-validation]] — Input validation for file-system endpoints
- [[jwt-security]] — JWT secret management and rotation
