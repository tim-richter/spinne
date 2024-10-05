use clap::Parser;
use std::{fs::File, io::Write, path::PathBuf};

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
    #[arg(short, long, default_value =  "json")]
    format: Format,

    /// Ignore directories in glob patterns (comma separated)
    #[arg(short, long, value_delimiter = ',', default_value = "**/node_modules/**,**/dist/**,**/build/**")]
    ignore: Vec<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum, Debug)]
enum Format {
    JSON,
    Console,
    HTML,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let mut traverser = ProjectTraverser::new();
    let component_graph = traverser.traverse(&args.entry, &args.ignore)?;
    
    let output = args.output.unwrap();

    // output to json file in current working directory
    if args.format == Format::JSON {
        let output_path_with_extension = format!("{}.json", output);
        println!("Writing report to: {:?}", output_path_with_extension);
        let file = File::create(output_path_with_extension)?;
        serde_json::to_writer_pretty(file, &component_graph.to_serializable())?;
    }

    if args.format == Format::HTML {
        let output_path_with_extension = format!("{}.html", output);
        println!("Writing report to: {:?}", output_path_with_extension);

        let mut file = File::create(output_path_with_extension)?;
        let html_content = generate_html_report(&component_graph);
        file.write_all(html_content.as_bytes())?;
    }

    // output to console
    if args.format == Format::Console {
        println!("Printing report to console:");
        println!("{:?}", component_graph.print_graph());
    }

    Ok(())
}

fn generate_html_report(component_graph: &spinne::ComponentGraph) -> String {
    let serializable = component_graph.to_serializable();
    let nodes_json = serde_json::to_string(&serializable.nodes).unwrap();
    let edges_json = serde_json::to_string(&serializable.edges).unwrap();

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Component Graph</title>
    <script type="text/javascript" src="https://unpkg.com/vis-network/standalone/umd/vis-network.min.js"></script>
    <style type="text/css">
        #mynetwork {{
            width: 100%;
            height: 600px;
            border: 1px solid lightgray;
        }}
    </style>
</head>
<body>
<div id="mynetwork"></div>
<script type="text/javascript">
    var nodes = new vis.DataSet({nodes_json});
    var edges = new vis.DataSet({edges_json});

    var container = document.getElementById('mynetwork');
    var data = {{
        nodes: nodes,
        edges: edges
    }};
    var options = {{
        nodes: {{
            shape: 'box',
            font: {{
                size: 12,
                face: 'Tahoma'
            }}
        }},
        edges: {{
            width: 0.15,
            color: {{inherit: 'from'}},
            smooth: {{
                type: 'continuous'
            }}
        }},
        physics: {{
            stabilization: false,
            barnesHut: {{
                gravitationalConstant: -80000,
                springConstant: 0.001,
                springLength: 200
            }}
        }},
        layout: {{
            improvedLayout: true,
            hierarchical: {{
                enabled: true,
                levelSeparation: 150,
                nodeSpacing: 100,
                treeSpacing: 200,
                blockShifting: true,
                edgeMinimization: true,
                parentCentralization: true,
                direction: 'UD',
                sortMethod: 'hubsize'
            }}
        }}
    }};
    var network = new vis.Network(container, data, options);
</script>
</body>
</html>"#
    )
}