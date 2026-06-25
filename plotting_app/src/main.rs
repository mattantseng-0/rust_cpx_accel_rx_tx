mod app;
mod serial;


use std::sync::mpsc;
use std::thread;
use shared::messages::{AccMsg, Message, MessageId};
use serial::serial_receiver_thread;
use app::{DataPlottingApp};
use eframe::egui;


fn main() {

    let (tx, rx) = mpsc::channel::<AccMsg>();

    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();
            thread::spawn(move || serial_receiver_thread(tx, ctx));
            Ok(Box::new(DataPlottingApp::new(rx)))
        }),
    );
}
