use clap::Parser;
use colored::*;
use env_logger::Env;
use log::info;
use std::io::Write;
use std::{fs::File, path::PathBuf};

use spinne::ProjectTraverser;

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

    /// Ignore directories in glob patterns (comma separated)
    #[arg(
        short,
        long,
        value_delimiter = ',',
        default_value = "**/node_modules/**,**/dist/**,**/build/**"
    )]
    ignore: Vec<String>,

    /// Sets the level of logging
    #[arg(short, long, default_value = "info")]
    log_level: LogLevel,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum, Debug)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum, Debug)]
enum Format {
    File,
    Console,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let log_level = match args.log_level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
        LogLevel::Trace => "trace",
    };

    // Initialize the logger
    env_logger::Builder::from_env(Env::default().default_filter_or(log_level))
        .format(|buf, record| {
            let level = record.level();

            // Assign colors based on log level
            let level_str = match level {
                log::Level::Error => "ERROR".red().bold().to_string(),
                log::Level::Warn => "WARN".yellow().bold().to_string(),
                log::Level::Info => "INFO".green().bold().to_string(),
                log::Level::Debug => "DEBUG".blue().bold().to_string(),
                log::Level::Trace => "TRACE".purple().bold().to_string(),
            };

            writeln!(buf, "{}: {}", level_str, record.args())
        })
        .init();

    let absolute_entry = std::fs::canonicalize(&args.entry)?;

    let mut traverser = ProjectTraverser::new(&absolute_entry);
    let component_graph = traverser.traverse(&args.entry, &args.ignore)?;

    let file_name = args.file_name.unwrap();

    // output to json file in current working directory
    if args.format == Format::File {
        let current_dir = std::env::current_dir()?;
        let output_path_with_extension = current_dir.join(format!("{}.json", file_name));
        info!("Writing report to: {:?}", output_path_with_extension);
        let file = File::create(output_path_with_extension)?;
        serde_json::to_writer_pretty(file, &component_graph.to_serializable())?;
    }

    // output to console
    if args.format == Format::Console {
        info!("Printing report to console:");
        component_graph.print_graph();
    }

    Ok(())
}
