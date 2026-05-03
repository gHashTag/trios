"""End-to-end Queen↔Doctor autonomous-loop smoke test.

Verifies the full bus contract on a *running* trios-server:

  Queen  --queen/order-->  trios-server  --BusEvent::QueenOrder-->  Doctor
  Doctor --doctor/report--> trios-server --BusEvent::DoctorReport--> Queen

Usage:
    # 1. Start the server (default port 9005):
    cargo run -p trios-server --bin trios-server
    # 2. In another shell:
    python3 crates/trios-queen-loop/tests-e2e/smoke_queen_doctor_loop.py

Exits 0 ("GREEN") when both halves of the loop close end-to-end.

Constitutional anchors: φ² + φ⁻² = 3 · L21 (append-only) · L24 (canonical bus).
Agent: SCARABS  Soul: Scarab Smith
"""
import asyncio, json, sys
import websockets

WS_URL = "ws://127.0.0.1:9005/ws"


async def doctor_subscriber(received_orders):
    async with websockets.connect(WS_URL) as ws:
        try:
            for _ in range(20):
                msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
                data = json.loads(msg)
                evt = data.get("event") if isinstance(data, dict) else None
                if isinstance(evt, dict):
                    print(f"[doctor] full event: {json.dumps(evt)[:200]}")
                    if evt.get("type") == "QueenOrder":
                        payload = evt.get("data", {})
                        received_orders.append(payload)
                        rpc = {
                            "jsonrpc": "2.0",
                            "id": "doctor-report-1",
                            "method": "doctor/report",
                            "params": {
                                "order_id": payload["order_id"],
                                "agent_id": "doctor",
                                "status": "green",
                                "summary": "smoke-test ack",
                                "diagnosis": {"checks": [], "smoke": True},
                            },
                        }
                        await ws.send(json.dumps(rpc))
                        print(f"[doctor] queen_order={payload['order_id']} → sent doctor/report")
                        # keep socket open for a bit so report can broadcast
                        await asyncio.sleep(0.5)
                        return
        except asyncio.TimeoutError:
            print("[doctor] TIMEOUT")


async def queen_publisher_and_listener(received_reports):
    async with websockets.connect(WS_URL) as ws:
        await asyncio.sleep(0.3)  # let doctor connect
        rpc = {
            "jsonrpc": "2.0", "id": "queen-order-1", "method": "queen/order",
            "params": {"action": "doctor scan", "target_agent": "doctor",
                       "params": {"reason": "smoke-test"}},
        }
        await ws.send(json.dumps(rpc))
        print("[queen] sent queen/order")
        try:
            for _ in range(20):
                msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
                data = json.loads(msg)
                evt = data.get("event") if isinstance(data, dict) else None
                if isinstance(evt, dict):
                    print(f"[queen] full event: {json.dumps(evt)[:200]}")
                    if evt.get("type") == "DoctorReport":
                        payload = evt.get("data", {})
                        received_reports.append(payload)
                        print(f"[queen] DoctorReport: {payload}")
                        return
        except asyncio.TimeoutError:
            print("[queen] TIMEOUT")


async def main():
    orders, reports = [], []
    await asyncio.gather(
        doctor_subscriber(orders),
        queen_publisher_and_listener(reports),
    )
    print("---")
    print(f"orders received by doctor: {len(orders)}")
    print(f"reports received by queen: {len(reports)}")
    if orders and reports:
        print("RESULT: GREEN — Queen↔Doctor loop closed end-to-end through real WS bus")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
