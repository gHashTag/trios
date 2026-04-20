use anyhow::Result;
use oracle::{Action, OracleController};

fn main() -> Result<()> {
    let mut controller = OracleController::new(3, 0.5);
    let seeds = [42u64, 84, 126, 168, 210, 252];

    let bpb_values = [10.0, 5.0, 2.0, 0.8, 0.3];

    for (round, &bpb) in bpb_values.iter().enumerate() {
        let start = (round * 2).min(seeds.len());
        let pending: Vec<u64> = seeds[start..].to_vec();
        let decisions = controller.decide(bpb, &pending);

        for d in &decisions {
            let icon = match d.action {
                Action::Spawn => "SPAWN",
                Action::Kill => "KILL",
                Action::Wait => "WAIT",
            };
            println!(
                "[round {}] {} seed={} | {}",
                round + 1,
                icon,
                d.seed,
                d.reason
            );
        }
    }

    let report = oracle::OracleDecision {
        action: Action::Wait,
        seed: 0,
        reason: "dry-run complete".into(),
    };
    println!("\n--- JSON ---");
    println!("{}", serde_json::to_string_pretty(&report)?);

    Ok(())
}
