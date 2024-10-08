use clap::Parser;
use std::{fs::File, path::PathBuf};

use spinne::ProjectTraverser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Entry point directory
    #[arg(short, long, default_value = ".")]
    entry: PathBuf,

    /// Output file name
    #[arg(short, long, default_value = "spinne-report")]
    output: Option<String>,

    /// output format
    #[arg(short, long, default_value =  "file")]
    format: Format,

    /// Ignore directories in glob patterns (comma separated)
    #[arg(short, long, value_delimiter = ',', default_value = "**/node_modules/**,**/dist/**,**/build/**")]
    ignore: Vec<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum, Debug)]
enum Format {
    File,
    Console,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let mut traverser = ProjectTraverser::new();
    let component_graph = traverser.traverse(&args.entry, &args.ignore)?;
    
    let output = args.output.unwrap();

    // output to json file in current working directory
    if args.format == Format::File {
        let output_path_with_extension = format!("{}.json", output);
        println!("Writing report to: {:?}", output_path_with_extension);
        let file = File::create(output_path_with_extension)?;
        serde_json::to_writer_pretty(file, &component_graph.to_serializable())?;
    }

    // output to console
    if args.format == Format::Console {
        println!("Printing report to console:");
        println!("{:?}", component_graph.print_graph());
    }

    Ok(())
}