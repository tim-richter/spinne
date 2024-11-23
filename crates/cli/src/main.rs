use clap::Parser;
use spinne_logger::Logger;
use std::{fs::File, path::PathBuf};

use spinne_html::HtmlGenerator;
use spinne_core::ProjectTraverser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Entry point directory
    #[arg(short, long, default_value = "./")]
    entry: PathBuf,

    /// Output file name
    #[arg(long, default_value = "spinne-report")]
    file_name: Option<String>,

    /// output format
    #[arg(short, long, default_value = "file")]
    format: Format,

    /// Exclude directories/files with glob patterns (comma separated)
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "**/node_modules/**,**/dist/**,**/build/**,**/*.stories.tsx,**/*.test.tsx"
    )]
    exclude: Vec<String>,

    /// Include directories/files with glob patterns (comma separated)
    #[arg(long, value_delimiter = ',', default_value = "**/*.tsx")]
    include: Vec<String>,

    /// Verbosity level (-l = level 1, -ll = level 2, etc.)
    #[arg(short = 'l', action = clap::ArgAction::Count)]
    verbosity: u8,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum, Debug)]
enum Format {
    File,
    Console,
    Html,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    Logger::set_level(args.verbosity);

    let absolute_entry = std::fs::canonicalize(&args.entry)?;

    let mut traverser = ProjectTraverser::new(&absolute_entry);
    let component_graph = traverser.traverse(&args.entry, &args.exclude, &args.include)?;

    let file_name = args.file_name.unwrap();

    // output to json file in current working directory
    if args.format == Format::File {
        let current_dir = std::env::current_dir()?;
        let output_path_with_extension = current_dir.join(format!("{}.json", file_name));

        Logger::info(&format!(
            "Writing report to: {:?}",
            output_path_with_extension
        ));

        let file = File::create(output_path_with_extension)?;
        serde_json::to_writer_pretty(file, &component_graph.to_serializable())?;
    }

    // output to console
    if args.format == Format::Console {
        Logger::info("Printing report to console:");
        component_graph.print_graph();
    }

    // output to html file in current working directory
    if args.format == Format::Html {
        let current_dir = std::env::current_dir()?;
        let output_path_with_extension = current_dir.join(format!("{}.html", file_name));

        Logger::info(&format!(
            "Writing report to: {:?}",
            output_path_with_extension
        ));

        let graph_data = serde_json::json!(component_graph.to_serializable());

        match HtmlGenerator::new(graph_data).save(&output_path_with_extension) {
            Ok(_) => Logger::info(&format!(
                "Report written to: {:?}",
                output_path_with_extension
            )),
            Err(e) => Logger::error(&format!("Failed to write report: {}", e)),
        }

        #[cfg(not(test))]
        match open::that_detached(output_path_with_extension) {
            Ok(_) => Logger::info("Opened report in browser"),
            Err(e) => Logger::error(&format!("Failed to open report in browser: {}", e)),
        }
    }

    Ok(())
}
