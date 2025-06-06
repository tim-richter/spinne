use clap::Parser;
use spinne_logger::Logger;
use std::{fs::File, io::{BufWriter, Write}, path::PathBuf};

use spinne_core::Workspace;
use spinne_html::HtmlGenerator;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Entry point directory
    ///
    /// This is only the starting point of the analysis and spinne will traverse all projects in this directory.
    #[arg(short, long, default_value = "./")]
    entry: PathBuf,

    /// Output format for the report
    ///
    /// - file: Outputs a JSON file (spinne-report.json) containing the component graph.
    ///   The JSON structure includes:
    ///   - An array of project objects, each containing:
    ///     - name: The project name
    ///     - graph: A component graph containing:
    ///       - components: Array of component objects with id, name, path, props, and project
    ///       - edges: Array of edge objects with from and to component IDs
    ///
    /// - console: Prints the report directly to the console in a human-readable format
    ///
    /// - html: Generates an interactive HTML report (spinne-report.html)
    ///
    /// - json: Outputs raw JSON to stdout (useful for piping to other commands)
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
    /// Outputs a JSON file containing the component graph
    File,
    /// Prints the report directly to the console
    Console,
    /// Generates an interactive HTML report
    Html,
    /// Outputs raw JSON to stdout (useful for piping to other commands)
    Json,
}

const FILE_NAME: &str = "spinne-report";

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    Logger::set_level(args.verbosity);

    let absolute_entry = std::fs::canonicalize(&args.entry)?;

    let mut workspace = Workspace::new(absolute_entry);
    workspace.discover_projects();
    workspace.traverse_projects(&args.exclude, &args.include);

    // Get the shared component registry
    let registry = workspace.get_component_registry();
    let serializable_data = registry.to_serializable();

    // output to json file in current working directory
    if args.format == Format::File {
        let current_dir = std::env::current_dir()?;
        let output_path_with_extension = current_dir.join(format!("{}.json", FILE_NAME));

        Logger::info(&format!(
            "Writing report to: {:?}",
            output_path_with_extension
        ));

        let file = File::create(output_path_with_extension)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &serializable_data)?;
        writer.flush()?;
    }

    // output to console
    if args.format == Format::Console {
        Logger::info("Printing report to console:");
        println!("{:#?}", serializable_data);
    }

    // output raw JSON to stdout
    if args.format == Format::Json {
        let stdout = std::io::stdout();
        let mut writer = BufWriter::new(stdout.lock());
        serde_json::to_writer(&mut writer, &serializable_data)?;
        writer.flush()?;
        // Ensure we exit cleanly after writing
        std::process::exit(0);
    }

    // output to html file in current working directory
    if args.format == Format::Html {
        let current_dir = std::env::current_dir()?;
        let output_path_with_extension = current_dir.join(format!("{}.html", FILE_NAME));

        Logger::info(&format!(
            "Writing report to: {:?}",
            output_path_with_extension
        ));

        match HtmlGenerator::new(serializable_data.clone())
            .save(&output_path_with_extension)
        {
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
