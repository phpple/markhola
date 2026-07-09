mod app;
mod markdown;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("markhola failed: {error}");
    }
}
