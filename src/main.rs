use std::path::PathBuf;

#[path = "interface/app.rs"]
mod app;
#[path = "interface/document.rs"]
mod document;
#[path = "interface/file_io.rs"]
mod file_io;
#[path = "interface/html_export.rs"]
mod html_export;
#[path = "interface/markdown.rs"]
mod markdown;
#[path = "interface/pdf_export.rs"]
mod pdf_export;
#[path = "interface/printing.rs"]
mod printing;
#[path = "interface/render_assets.rs"]
mod render_assets;
#[path = "interface/workspace.rs"]
mod workspace;

fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.len() == 4 && args[1] == "--smoke-export" {
        let input = PathBuf::from(&args[2]);
        let output = PathBuf::from(&args[3]);
        if let Err(error) = pdf_export::export_markdown_file_to_path(&input, &output) {
            eprintln!("markhola smoke export failed: {error}");
            std::process::exit(1);
        }
        println!("markhola smoke export succeeded: {}", output.display());
        return;
    }
    if args.len() == 3 && args[1] == "--smoke-print-prepare" {
        let input = PathBuf::from(&args[2]);
        if let Err(error) = printing::smoke_prepare_markdown_file_for_print(&input) {
            eprintln!("markhola smoke print prepare failed: {error}");
            std::process::exit(1);
        }
        println!(
            "markhola smoke print prepare succeeded: {}",
            input.display()
        );
        return;
    }
    if args.len() == 4 && args[1] == "--smoke-export-html" {
        let input = PathBuf::from(&args[2]);
        let output = PathBuf::from(&args[3]);
        if let Err(error) = html_export::export_markdown_file_to_path(&input, &output) {
            eprintln!("markhola smoke html export failed: {error}");
            std::process::exit(1);
        }
        println!("markhola smoke html export succeeded: {}", output.display());
        return;
    }

    if let Err(error) = app::run() {
        eprintln!("markhola failed: {error}");
    }
}
