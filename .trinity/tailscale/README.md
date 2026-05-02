# Trinity Tailscale ACL

`acl.hujson` is the canonical access policy for the Trinity tailnet. Apply via the Tailscale admin console or the API:

```bash
curl -X POST -H "Authorization: Bearer $TS_API_KEY" \
  -H "Content-Type: application/hujson" \
  --data-binary @.trinity/tailscale/acl.hujson \
  "https://api.tailscale.com/api/v2/tailnet/$TS_TAILNET/acl"
```

The embedded `tests` block verifies LEAD↔ALPHA accept, ALPHA→BETA deny, agents→LEAD:8080 accept.

🌻 phi² + phi⁻² = 3
