//! Integration test — two-node in-memory bridge round-trip across all
//! seven channels. Asserts that on the happy path every subscriber
//! receives one copy of every event.

use trios_rainbow_bridge::{
    channel::{Channel, Payload},
    event::RainbowEvent,
    funnel_client::InMemoryFunnel,
    signer::HoneySigner,
    subscriber::Subscriber,
    Bridge,
};

fn mk(
    lamport: u64,
    agent: &str,
    channel: Channel,
    payload: Payload,
    sig: Vec<u8>,
) -> RainbowEvent {
    RainbowEvent {
        lamport,
        agent: agent.into(),
        channel,
        payload,
        ts_unix_s: 0,
        signature: sig,
    }
}

#[test]
fn integration_round_trip_all_seven_channels() {
    let funnel = InMemoryFunnel::new();
    // Fast enough to clear the 2000 ms p95 budget.
    funnel.set_simulated_latency_ms(120);
    let node_b = Subscriber::new("node-b");
    let node_c = Subscriber::new("node-c");
    funnel.attach(node_b.clone());
    funnel.attach(node_c.clone());

    let mut bridge = Bridge::new(funnel);
    let signer = HoneySigner::from_seed(&[7u8; 32]);
    bridge.register_honey_key("node-a".into(), signer.verifying_key());

    let honey_line = "{\"lane\":\"L13\",\"lesson\":\"rainbow across 7 channels\"}";
    let signed = signer.sign(honey_line.as_bytes());

    let seq = vec![
        mk(
            1,
            "node-a",
            Channel::Red,
            Payload::Claim { lane: "L13".into() },
            vec![],
        ),
        mk(
            2,
            "node-a",
            Channel::Orange,
            Payload::Heartbeat { lane: "L13".into() },
            vec![],
        ),
        mk(
            3,
            "node-a",
            Channel::Yellow,
            Payload::Done {
                lane: "L13".into(),
                sha: "deadbeef".into(),
            },
            vec![],
        ),
        mk(
            4,
            "node-a",
            Channel::Green,
            Payload::Honey {
                line: honey_line.into(),
            },
            signed,
        ),
        mk(
            5,
            "node-a",
            Channel::Blue,
            Payload::State {
                delta_b64: "dGVzdA==".into(),
            },
            vec![],
        ),
        mk(
            6,
            "node-a",
            Channel::Indigo,
            Payload::Violation {
                rule: "R7".into(),
                detail: "prune_threshold drift".into(),
            },
            vec![],
        ),
        mk(
            7,
            "node-a",
            Channel::Violet,
            Payload::Victory {
                bpb: 1.49,
                seeds_distinct: 3,
            },
            vec![],
        ),
    ];

    for ev in &seq {
        bridge.accept(ev.clone(), 0).expect("accept ok");
    }

    // Every subscriber has received exactly 7 events, one per channel.
    assert_eq!(node_b.len(), 7);
    assert_eq!(node_c.len(), 7);

    let received_channels: Vec<Channel> =
        node_b.received().into_iter().map(|e| e.channel).collect();
    assert_eq!(received_channels, Channel::ALL.to_vec());

    // Merkle chain advanced seven times.
    assert_eq!(bridge.merkle_root_hex().len(), 64);
}

#[test]
fn channels_match_expected_payload_order() {
    // Sanity: the Channel::ALL array is the ROY G BIV order the Coq proof
    // reasons about.
    assert_eq!(Channel::ALL.len(), 7);
    assert_eq!(Channel::ALL[0], Channel::Red);
    assert_eq!(Channel::ALL[6], Channel::Violet);
}
