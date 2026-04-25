# tri-cli — One Command for trios-server Cloud

Start trios-server with Tailscale Funnel in ONE command.

## Usage

```bash
# Start everything (recommended)
cargo run -p tri-cli -- start

# Check status
cargo run -p tri-cli -- status

# Stop everything
cargo run -p tri-cli -- stop
```

## What it does

1. **Starts trios-server** on port 9005 (or custom port)
2. **Starts Tailscale Funnel** to expose server to the internet
3. **Shows your cloud URL** like `https://your-device.ts.net/`

## Example Output

```
🚀 Starting trios-server on port 9005...
🌐 Starting Tailscale Funnel...

✅ tri cloud is running!
╔════════════════════════════════════════╗
║      tri-tunnel Status               ║
╠════════════════════════════════════════╣
║ Device: playra's MacBook Pro         ║
║ Funnel: ACTIVE ✅                     ║
║ URL: https://playras-macbook-pro-1.tail01804b.ts.net/ ║
║ Port: 9005                            ║
╚════════════════════════════════════════╝

📡 Your trios-server is now accessible from anywhere!
📝 Press Ctrl+C to stop

🟢 Server running locally at http://localhost:9005
```

## Requirements

- Tailscale installed from App Store (not brew)
- Tailscale logged in
- Port 9005 available
