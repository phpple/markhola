use std::path::PathBuf;

mod app;
mod document;
mod file_io;
mod markdown;
mod pdf_export;
mod render_assets;
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

    if let Err(error) = app::run() {
        eprintln!("markhola failed: {error}");
    }
}
