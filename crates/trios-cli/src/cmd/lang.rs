use anyhow::Result;

pub struct LangCmd {
    pub lang: String,
}

pub fn run(cmd: LangCmd) -> Result<()> {
    println!("Language set to: {}", cmd.lang);
    Ok(())
}
