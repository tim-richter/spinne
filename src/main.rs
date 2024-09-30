use clap::Parser;
use std::path::PathBuf;
use spinne::ProjectTraverser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Entry point file
    #[arg(short, long)]
    entry: PathBuf,

    /// Output file for the report
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let traverser = ProjectTraverser::new();
    let component_graph = traverser.traverse(&args.entry)?;

    println!("Component Graph:");
    println!("{}", component_graph.generate_dot());

    println!("\nComponent Report:");
    let report = component_graph.generate_report();
    println!("{}", report);

    if let Some(output_path) = args.output {
        std::fs::write(output_path, report)?;
    }

    Ok(())
}
