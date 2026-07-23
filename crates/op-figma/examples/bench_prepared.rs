//! Stage benchmark for the deferred binary Figma importer.
//!
//! Reports preparation, one-page conversion, and all-pages conversion
//! separately, together with prepared page metadata and recursive converted
//! node counts. Run this example with `--release` for meaningful timings.
//!
//! `PreparedFig` conversion consumes the prepared value, so the all-pages run
//! performs and reports a second preparation after the one-page result has
//! been summarized and dropped.
//!
//! Usage:
//! `cargo run --release -p op-figma --example bench_prepared -- <file.fig> [--page INDEX] [--layout preserve|openpencil]`

use jian_ops_schema::node::PenNode;
use op_figma::{prepare_fig_binary, FigLayoutMode};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const HELP: &str = "\
Benchmark PreparedFig stages on a real binary .fig file.

Usage:
  cargo run --release -p op-figma --example bench_prepared -- \
    <file.fig> [--page INDEX] [--layout preserve|openpencil]

Options:
  --page INDEX       Page converted by the single-page run (default: 0)
  --layout MODE      preserve (default) or openpencil
  -h, --help         Show this help

The input is read before timing starts. Conversion includes image resolution.
Because conversion consumes PreparedFig, the all-pages run prepares the same
bytes a second time and reports that re-prepare duration separately.";

struct Options {
    path: PathBuf,
    page_index: usize,
    layout_mode: FigLayoutMode,
}

struct PageSummary {
    name: String,
    root_nodes: usize,
    all_nodes: usize,
}

fn main() {
    match parse_options(std::env::args().skip(1)) {
        Ok(Some(options)) => {
            if let Err(error) = run(options) {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        }
        Ok(None) => println!("{HELP}"),
        Err(error) => {
            eprintln!("error: {error}\n\n{HELP}");
            std::process::exit(2);
        }
    }
}

fn parse_options(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut args = args.into_iter();
    let mut path = None;
    let mut page_index = 0;
    let mut layout_mode = FigLayoutMode::Preserve;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--page" => {
                let value = args.next().ok_or("--page requires an index")?;
                page_index = value
                    .parse()
                    .map_err(|_| format!("invalid page index {value:?}"))?;
            }
            "--layout" => {
                let value = args.next().ok_or("--layout requires a mode")?;
                layout_mode = match value.as_str() {
                    "preserve" => FigLayoutMode::Preserve,
                    "openpencil" => FigLayoutMode::OpenPencil,
                    _ => return Err(format!("invalid layout mode {value:?}")),
                };
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option {value:?}"));
            }
            value => {
                if path.replace(PathBuf::from(value)).is_some() {
                    return Err("only one input .fig path may be supplied".to_string());
                }
            }
        }
    }

    let path = path.ok_or("missing input .fig path")?;
    Ok(Some(Options {
        path,
        page_index,
        layout_mode,
    }))
}

fn run(options: Options) -> Result<(), String> {
    let bytes = std::fs::read(&options.path)
        .map_err(|error| format!("cannot read {}: {error}", options.path.display()))?;
    let file_name = options
        .path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Figma Import");
    println!("input: {} ({} bytes)", options.path.display(), bytes.len());
    println!("layout: {}", layout_name(options.layout_mode));

    let started = Instant::now();
    let prepared = prepare_fig_binary(&bytes, file_name, options.layout_mode)
        .map_err(|error| format!("prepare failed: {error}"))?;
    let prepare_elapsed = started.elapsed();
    let pages = prepared.pages().to_vec();
    println!("prepare: {}", elapsed(prepare_elapsed));
    println!("prepared pages: {}", pages.len());
    for (index, page) in pages.iter().enumerate() {
        println!(
            "  [{index}] id={:?} name={:?} root_children={}",
            page.id, page.name, page.child_count
        );
    }

    let selected = pages.get(options.page_index).ok_or_else(|| {
        format!(
            "page index {} is out of range (page count: {})",
            options.page_index,
            pages.len()
        )
    })?;
    let started = Instant::now();
    let single = prepared
        .into_page(options.page_index)
        .map_err(|error| format!("single-page conversion failed: {error}"))?;
    let single_elapsed = started.elapsed();
    let single_warnings = single.warnings.len();
    let single_pages = summarize_pages(single.document.pages.as_deref().unwrap_or(&[]));
    println!(
        "single-page convert: {} index={} name={:?} warnings={}",
        elapsed(single_elapsed),
        options.page_index,
        selected.name,
        single_warnings
    );
    print_converted_pages(&single_pages);
    drop(single);

    let started = Instant::now();
    let prepared = prepare_fig_binary(&bytes, file_name, options.layout_mode)
        .map_err(|error| format!("re-prepare failed: {error}"))?;
    let reprepare_elapsed = started.elapsed();
    println!("re-prepare for all pages: {}", elapsed(reprepare_elapsed));

    let started = Instant::now();
    let all = prepared
        .into_all_pages()
        .map_err(|error| format!("all-pages conversion failed: {error}"))?;
    let all_elapsed = started.elapsed();
    let all_warnings = all.warnings.len();
    let all_pages = summarize_pages(all.document.pages.as_deref().unwrap_or(&[]));
    let total_nodes: usize = all_pages.iter().map(|page| page.all_nodes).sum();
    println!(
        "all-pages convert: {} pages={} nodes={} warnings={}",
        elapsed(all_elapsed),
        all_pages.len(),
        total_nodes,
        all_warnings
    );
    print_converted_pages(&all_pages);
    Ok(())
}

fn summarize_pages(pages: &[jian_ops_schema::page::PenPage]) -> Vec<PageSummary> {
    pages
        .iter()
        .map(|page| PageSummary {
            name: page.name.clone(),
            root_nodes: page.children.len(),
            all_nodes: count_nodes(&page.children),
        })
        .collect()
}

fn print_converted_pages(pages: &[PageSummary]) {
    for (index, page) in pages.iter().enumerate() {
        println!(
            "  [{index}] name={:?} root_nodes={} nodes={}",
            page.name, page.root_nodes, page.all_nodes
        );
    }
}

fn count_nodes(nodes: &[PenNode]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + count_nodes(children(node)))
        .sum()
}

fn children(node: &PenNode) -> &[PenNode] {
    match node {
        PenNode::Frame(node) => node.children.as_deref().unwrap_or(&[]),
        PenNode::Group(node) => node.children.as_deref().unwrap_or(&[]),
        PenNode::Rectangle(node) => node.children.as_deref().unwrap_or(&[]),
        PenNode::Tabs(node) => node.children.as_deref().unwrap_or(&[]),
        PenNode::Ref(node) => node.children.as_deref().unwrap_or(&[]),
        _ => &[],
    }
}

fn elapsed(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1_000.0)
}

fn layout_name(mode: FigLayoutMode) -> &'static str {
    match mode {
        FigLayoutMode::Preserve => "preserve",
        FigLayoutMode::OpenPencil => "openpencil",
    }
}
