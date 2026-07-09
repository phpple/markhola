mod app;
mod document;
mod file_io;
mod markdown;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("markhola failed: {error}");
    }
}
