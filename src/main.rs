mod content;
mod events;
mod game;
mod model;
mod persistence;
mod ui;

fn main() {
    if let Err(err) = game::run() {
        eprintln!("Fatal error: {err}");
        std::process::exit(1);
    }
}
