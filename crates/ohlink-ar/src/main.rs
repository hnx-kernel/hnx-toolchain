use anyhow::Result;
use ohlink_format::OhlibBuilder;
use std::fs;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Create Ohlib static library")]
struct Args {
    /// Output .ohlib
    #[arg(short, long)]
    output: PathBuf,
    /// Input .o files
    inputs: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();   // ← 就是这行缺失
    let mut b = OhlibBuilder::new();
    for p in &args.inputs {
        let bytes = fs::read(p)?;
        let name = p.file_name().unwrap().to_str().unwrap();
        b.add_member(name, &bytes);
    }
    fs::write(&args.output, &b.build())?;
    println!("ar: {} members -> {}", args.inputs.len(), args.output.display());
    Ok(())
}