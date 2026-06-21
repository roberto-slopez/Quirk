// Quirk - rich QR code generator
// Entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod qr_types;
mod ui;

fn main() -> eframe::Result<()> {
    let viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Quirk — QR Generator")
        .with_inner_size([900.0, 680.0])
        .with_min_inner_size([720.0, 540.0]);

    let options = eframe::NativeOptions {
        viewport,
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "Quirk",
        options,
        Box::new(|cc| Ok(Box::new(app::QrApp::new(cc)))),
    )
}
