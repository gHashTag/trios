# UART Endpoint

Remote access to the host's USB-serial adapters through `trios-server`,
intended for pairing with [tri-tunnel](https://github.com/gHashTag/tri-tunnel)
so that a cloud agent can drive a physical UART on the developer Mac.

Anchor: `phi^2 + phi^-2 = 3`.

## Threat model

- Assumes `trios-server` is either bound to `127.0.0.1` or fronted by
  `tri-tunnel` / Tailscale Funnel. In the latter case, the tailnet ACL provides
  transport auth. `TRIOS_UART_TOKEN` is a defence-in-depth bearer on top.
- The UART endpoint uses a **separate** token from `TRIOS_API_KEY`. This is
  deliberate: git access should not automatically grant serial-port access.
- If `TRIOS_UART_TOKEN` is unset, the router is not mounted at all
  (fail-closed). The server logs `UART endpoint DISABLED` on startup.

## Configuration

```bash
# One-time: generate a strong token (macOS / Linux)
export TRIOS_UART_TOKEN="$(openssl rand -hex 32)"

# Then start the server
cargo run -p trios-server
```

Look for one of these lines in the startup log:

```
INFO UART endpoint ENABLED at /api/uart (auth: TRIOS_UART_TOKEN)
WARN UART endpoint DISABLED — set TRIOS_UART_TOKEN to enable /api/uart/*
```

## Endpoints

All endpoints require `Authorization: Bearer $TRIOS_UART_TOKEN`. The global
`TRIOS_API_KEY` check is bypassed for `/api/uart/*` — clients only need one
credential.

### `GET /api/uart/ports`

Enumerate available serial ports without opening them.

```bash
curl -s http://127.0.0.1:9005/api/uart/ports \
  -H "Authorization: Bearer $TRIOS_UART_TOKEN" | jq
```

Response:

```json
{
  "ports": [
    {
      "device": "/dev/cu.usbmodem14201",
      "port_type": "usb",
      "vid": 1027,
      "pid": 24597,
      "serial_number": "FTBXYZ",
      "manufacturer": "FTDI",
      "product": "FT232R USB UART"
    }
  ]
}
```

### `GET /api/uart/stream?port=<device>&baud=<baud>`

Server-Sent Events stream of bytes read from the port. `baud` defaults to
115200.

Event types:

| Event    | Payload                                             |
|----------|-----------------------------------------------------|
| `data`   | base64-encoded chunk of bytes read from the port    |
| `lag`    | integer — number of chunks dropped due to backpressure |
| `idle`   | literal `"30s no data"` — sent after 30s of silence |
| `closed` | literal `"port closed"` — port reader stopped       |

Example client (curl in the raw):

```bash
curl -N -s "http://127.0.0.1:9005/api/uart/stream?port=/dev/cu.usbmodem14201&baud=115200" \
  -H "Authorization: Bearer $TRIOS_UART_TOKEN"
```

Decoding one event in shell:

```bash
# Pipe SSE through a small filter to decode `data:` payloads.
awk '/^data: / { print substr($0, 7) }' \
  | while read line; do echo -n "$line" | base64 -d; done
```

### `POST /api/uart/write`

One-shot write. Body:

```json
{
  "port": "/dev/cu.usbmodem14201",
  "baud": 115200,
  "data": "cm9vdApwYXNzd29yZAo="
}
```

`data` must be base64. This is intentional: it lets control characters
(`Ctrl-C = 0x03`, `Ctrl-A = 0x01`, escape sequences) survive JSON transport
without shell-escaping games.

```bash
# Send "root\n" then wait, then send "analog\n"
printf 'root\n' | base64 | xargs -I {} curl -sX POST \
  http://127.0.0.1:9005/api/uart/write \
  -H "Authorization: Bearer $TRIOS_UART_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"port":"/dev/cu.usbmodem14201","data":"'{}'"}'
```

Response:

```json
{ "written": 5 }
```

## Failure modes

| Symptom                                    | Diagnosis                                          |
|--------------------------------------------|----------------------------------------------------|
| `503 Service Unavailable` on `/api/uart/*` | `TRIOS_UART_TOKEN` unset — endpoint not mounted    |
| `401 Unauthorized`                         | Wrong or missing `Authorization: Bearer <token>`   |
| `500 Internal Server Error` with `open ...`| macOS blocked port access, or wrong device path    |
| `data` event never arrives                 | Port opens but the device is not transmitting yet — try cold-power-cycling the target board |
| `idle` event repeats                       | Port healthy, target silent for 30s — expected between boot messages |
| `lag` event with large N                   | Client is reading SSE slower than port produces — reduce baud or fix the client |

## Wiring with `tri-tunnel`

Once the endpoint is up locally, expose it through Tailscale Funnel:

```bash
# Assuming tri-tunnel already targets port 9005 (or 9105 in newer builds).
tri-tunnel start
```

Any tailnet-member client (including a cloud agent that installed `tailscaled`
via an auth-key) can now reach:

```
https://<device>.tailXXXX.ts.net/api/uart/ports
```

## Not implemented on purpose

- No **write-then-read** convenience endpoint. Compose `POST /write` +
  `GET /stream` on the client. This keeps the server's state minimal and
  concurrency easy to reason about.
- No **port sharing** between multiple SSE clients on the same device.
  serialport-rs opens exclusively; if you need multi-viewer, run one broadcaster
  process locally and put the fan-out in front of it.
- No **flow control settings** exposed (RTS/CTS/DTR). Default settings match
  P201Mini U-Boot / Linux console. Extend the query params if a future target
  needs otherwise.

## References

- [`serialport` crate v4](https://docs.rs/serialport/4/serialport/)
- [Axum SSE guide](https://docs.rs/axum/latest/axum/response/sse/index.html)
- [`docs/LOCAL_FLASH.md`](./LOCAL_FLASH.md) — the identical-image trap that
  motivated remote UART access in the first place
