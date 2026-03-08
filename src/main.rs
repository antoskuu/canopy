mod app;
mod render;
mod scanner;
mod treemap;
mod utils;

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "canopy", about = "Disk space treemap visualizer")]
struct Cli {
    /// Directory to scan (opens folder picker if omitted)
    path: Option<PathBuf>,
}

fn main() -> eframe::Result<()> {
    let cli = Cli::parse();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("Canopy"),
        ..Default::default()
    };

    eframe::run_native(
        "Canopy",
        options,
        Box::new(move |_cc| Ok(Box::new(app::StorageApp::new(cli.path)))),
    )
}
