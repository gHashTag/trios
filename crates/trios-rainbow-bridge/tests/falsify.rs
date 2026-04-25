//! Falsification witnesses for INV-8 (ONE SHOT trios#267, lane L13).
//!
//! Each test exercises **one** [`BridgeError`] variant. The mapping is
//! one-to-one with the Coq `counter_*` lemmas in
//! `trinity-clara/proofs/igla/rainbow_bridge_consistency.v`. CI fails if any
//! falsify_* is missing (enforced in `.github/workflows/rainbow-bridge.yml`).

use trios_rainbow_bridge::{
    channel::{Channel, Payload},
    event::RainbowEvent,
    funnel_client::InMemoryFunnel,
    signer::HoneySigner,
    Bridge, BridgeError, HEARTBEAT_MAX_S, LATENCY_P95_MS,
};

fn ev(
    lamport: u64,
    agent: &str,
    channel: Channel,
    payload: Payload,
    ts: u64,
    sig: Vec<u8>,
) -> RainbowEvent {
    RainbowEvent {
        lamport,
        agent: agent.into(),
        channel,
        payload,
        ts_unix_s: ts,
        signature: sig,
    }
}

// -------------------------------------------------------------------- //
// (1) DuplicateClaim — two agents claim the same lane at same lamport. //
// Coq: counter_duplicate_claim.                                        //
// -------------------------------------------------------------------- //
#[test]
fn falsify_duplicate_claim() {
    let mut b = Bridge::new(InMemoryFunnel::new());
    let a = ev(
        10,
        "alpha",
        Channel::Red,
        Payload::Claim { lane: "L13".into() },
        0,
        vec![],
    );
    let c = ev(
        10,
        "beta",
        Channel::Red,
        Payload::Claim { lane: "L13".into() },
        0,
        vec![],
    );

    b.accept(a, 0).expect("first claim accepted");
    let err = b.accept(c, 0).expect_err("second claim must be rejected");
    match err {
        BridgeError::DuplicateClaim {
            lane,
            lamport,
            agent_a,
            agent_b,
        } => {
            assert_eq!(lane, "L13");
            assert_eq!(lamport, 10);
            assert_eq!(agent_a, "alpha");
            assert_eq!(agent_b, "beta");
        }
        other => panic!("expected DuplicateClaim, got {other:?}"),
    }
}

// -------------------------------------------------------------------- //
// (2) HeartbeatStale — a heartbeat arrives > 4h after the previous one //
// Coq: counter_heartbeat_stale.                                        //
// -------------------------------------------------------------------- //
#[test]
fn falsify_heartbeat_stale() {
    let mut b = Bridge::new(InMemoryFunnel::new());
    let first = ev(
        1,
        "alpha",
        Channel::Orange,
        Payload::Heartbeat { lane: "L13".into() },
        0,
        vec![],
    );
    b.accept(first, 0).expect("first heartbeat ok");

    let stale = ev(
        2,
        "alpha",
        Channel::Orange,
        Payload::Heartbeat { lane: "L13".into() },
        HEARTBEAT_MAX_S + 1,
        vec![],
    );
    let err = b
        .accept(stale, HEARTBEAT_MAX_S + 1)
        .expect_err("stale heartbeat must be rejected");
    match err {
        BridgeError::HeartbeatStale { lane, age_s, max_s } => {
            assert_eq!(lane, "L13");
            assert!(age_s > max_s);
            assert_eq!(max_s, HEARTBEAT_MAX_S);
        }
        other => panic!("expected HeartbeatStale, got {other:?}"),
    }
}

// -------------------------------------------------------------------- //
// (3) LamportRegression — same agent, lower lamport.                   //
// Coq: counter_lamport_regression.                                     //
// -------------------------------------------------------------------- //
#[test]
fn falsify_lamport_regression() {
    let mut b = Bridge::new(InMemoryFunnel::new());
    let up = ev(
        10,
        "alpha",
        Channel::Red,
        Payload::Claim { lane: "L1".into() },
        0,
        vec![],
    );
    let down = ev(
        5,
        "alpha",
        Channel::Red,
        Payload::Claim { lane: "L2".into() },
        0,
        vec![],
    );
    b.accept(up, 0).expect("advance ok");
    let err = b.accept(down, 0).expect_err("regression rejected");
    match err {
        BridgeError::LamportRegression { agent, new, prev } => {
            assert_eq!(agent, "alpha");
            assert_eq!(new, 5);
            assert_eq!(prev, 10);
        }
        other => panic!("expected LamportRegression, got {other:?}"),
    }
}

// -------------------------------------------------------------------- //
// (4) UnsignedHoney — honey event without signature is rejected.       //
// Coq: counter_unsigned_honey.                                         //
// -------------------------------------------------------------------- //
#[test]
fn falsify_unsigned_honey() {
    let mut b = Bridge::new(InMemoryFunnel::new());
    let e = ev(
        1,
        "alpha",
        Channel::Green,
        Payload::Honey {
            line: "{}".into(),
        },
        0,
        vec![], // empty signature
    );
    let err = b.accept(e, 0).expect_err("unsigned honey rejected");
    match err {
        BridgeError::UnsignedHoney { agent } => assert_eq!(agent, "alpha"),
        other => panic!("expected UnsignedHoney, got {other:?}"),
    }
}

// -------------------------------------------------------------------- //
// (4b) Valid signed honey passes — sanity check for the happy path.    //
// -------------------------------------------------------------------- //
#[test]
fn signed_honey_accepted() {
    let mut b = Bridge::new(InMemoryFunnel::new());
    let signer = HoneySigner::from_seed(&[42u8; 32]);
    b.register_honey_key("alpha".into(), signer.verifying_key());
    let line = "{\"lane\":\"L13\",\"lesson\":\"f8 = 21\"}";
    let sig = signer.sign(line.as_bytes());
    let e = ev(
        1,
        "alpha",
        Channel::Green,
        Payload::Honey { line: line.into() },
        0,
        sig,
    );
    b.accept(e, 0).expect("signed honey ok");
}

// -------------------------------------------------------------------- //
// (5) SplitBrainDetected — two agents commit State at same lamport.    //
// Coq: counter_split_brain.                                            //
// -------------------------------------------------------------------- //
#[test]
fn falsify_split_brain() {
    let mut b = Bridge::new(InMemoryFunnel::new());
    let a = ev(
        17,
        "alpha",
        Channel::Blue,
        Payload::State {
            delta_b64: "aaa".into(),
        },
        0,
        vec![],
    );
    let c = ev(
        17,
        "beta",
        Channel::Blue,
        Payload::State {
            delta_b64: "bbb".into(),
        },
        0,
        vec![],
    );
    b.accept(a, 0).expect("first state ok");
    let err = b.accept(c, 0).expect_err("split brain rejected");
    match err {
        BridgeError::SplitBrainDetected {
            lamport,
            agent_a,
            agent_b,
        } => {
            assert_eq!(lamport, 17);
            assert_eq!(agent_a, "alpha");
            assert_eq!(agent_b, "beta");
        }
        other => panic!("expected SplitBrainDetected, got {other:?}"),
    }
}

// -------------------------------------------------------------------- //
// (6) FunnelUnreachable — publish latency > LATENCY_P95_MS.            //
// Coq: counter_funnel_unreachable.                                     //
// -------------------------------------------------------------------- //
#[test]
fn falsify_funnel_unreachable() {
    let funnel = InMemoryFunnel::new();
    funnel.set_simulated_latency_ms(LATENCY_P95_MS + 1);
    let mut b = Bridge::new(funnel);
    let e = ev(
        1,
        "alpha",
        Channel::Yellow,
        Payload::Done {
            lane: "L13".into(),
            sha: "abc".into(),
        },
        0,
        vec![],
    );
    let err = b.accept(e, 0).expect_err("funnel unreachable rejected");
    match err {
        BridgeError::FunnelUnreachable {
            latency_ms,
            budget_ms,
        } => {
            assert!(latency_ms > budget_ms);
            assert_eq!(budget_ms, LATENCY_P95_MS);
        }
        other => panic!("expected FunnelUnreachable, got {other:?}"),
    }
}

// -------------------------------------------------------------------- //
// (7) ChannelMismatch — declared channel != channel_of_payload.        //
// Coq: counter_channel_mismatch.                                       //
// -------------------------------------------------------------------- //
#[test]
fn falsify_channel_mismatch() {
    let mut b = Bridge::new(InMemoryFunnel::new());
    // Victory payload tagged as the Claim channel (Red).
    let e = ev(
        1,
        "alpha",
        Channel::Red,
        Payload::Victory {
            bpb: 1.49,
            seeds_distinct: 3,
        },
        0,
        vec![],
    );
    let err = b.accept(e, 0).expect_err("mismatch rejected");
    match err {
        BridgeError::ChannelMismatch { declared, expected } => {
            assert_eq!(declared, Channel::Red);
            assert_eq!(expected, Channel::Violet);
        }
        other => panic!("expected ChannelMismatch, got {other:?}"),
    }
}
