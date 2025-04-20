use clap::Parser;
use spinne_logger::Logger;
use std::{fs::File, path::PathBuf};

use spinne_core::Workspace;
use spinne_html::HtmlGenerator;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Entry point directory
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
}

const FILE_NAME: &str = "spinne-report";

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    Logger::set_level(args.verbosity);

    let absolute_entry = std::fs::canonicalize(&args.entry)?;

    let mut workspace = Workspace::new(absolute_entry);
    workspace.discover_projects();
    workspace.traverse_projects(&args.exclude, &args.include);

    // Create a serializable structure containing all project graphs
    let workspace_data = workspace
        .get_projects()
        .iter()
        .map(|project| {
            serde_json::json!({
                "name": project.get_name(),
                "graph": project.get_component_graph().to_serializable()
            })
        })
        .collect::<Vec<_>>();

    // output to json file in current working directory
    if args.format == Format::File {
        let current_dir = std::env::current_dir()?;
        let output_path_with_extension = current_dir.join(format!("{}.json", FILE_NAME));

        Logger::info(&format!(
            "Writing report to: {:?}",
            output_path_with_extension
        ));

        let file = File::create(output_path_with_extension)?;
        serde_json::to_writer_pretty(file, &workspace_data)?;
    }

    // output to console
    if args.format == Format::Console {
        Logger::info("Printing report to console:");
        for project_data in workspace_data.iter() {
            println!(
                "Project '{}': {:#?}",
                project_data["name"], project_data["graph"]
            );
        }
    }

    // output to html file in current working directory
    if args.format == Format::Html {
        let current_dir = std::env::current_dir()?;
        let output_path_with_extension = current_dir.join(format!("{}.html", FILE_NAME));

        Logger::info(&format!(
            "Writing report to: {:?}",
            output_path_with_extension
        ));

        match HtmlGenerator::new(serde_json::to_value(workspace_data).unwrap())
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
