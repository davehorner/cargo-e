// This file is the entry point of the application. It initializes the eframe window and sets up the main event loop for the egui interface.
mod app;
mod control;
mod parser;
mod position_grid;

fn main() -> eframe::Result<()> {
    e_window::run_window(std::env::args().skip(1))
}
