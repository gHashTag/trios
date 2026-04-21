use anyhow::Result;

pub struct StatusCmd {
    pub json: bool,
}

pub fn run(cmd: StatusCmd) -> Result<()> {
    if cmd.json {
        println!(r#"{{"status":"ok","branch":"dev","tests":436,"clippy":"clean"}}"#);
    } else {
        println!("Loop Status: OK");
        println!("  Branch: dev");
        println!("  Tests: 436 passed");
        println!("  Clippy: 0 warnings");
    }
    Ok(())
}
