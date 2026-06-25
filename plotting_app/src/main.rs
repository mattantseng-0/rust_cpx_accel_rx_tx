mod app;
mod serial;


use std::sync::mpsc;
use std::thread;
use shared::messages::{AccMsg, Message, MessageId};
use serial::serial_receiver_thread;
use app::{DataPlottingApp};
use eframe::egui;
use std::env;


fn main() {

    let mut args = env::args().skip(1);

    let mut serial_interface: Option<String> = None;

    if args.len() == 0 {
        eprintln!("No args received. Use 'cargo run -- -h' to see help menu");
        std::process::exit(1);
    }

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-d" | "--device" => {
                if let Some(val) = args.next() {
                    serial_interface = Some(val);
                } else {
                    eprintln!("No device given with device flag");
                    std::process::exit(1);
                    
                }
            }
            _ | "-h" | "--help" => {
                println!("-h help: print help menu\n-d device: /dev/ttySomeDevice");
                std::process::exit(1);

            }
        }
    }

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
            thread::spawn(move || serial_receiver_thread(tx, ctx, serial_interface.unwrap()));
            Ok(Box::new(DataPlottingApp::new(rx)))
        }),
    );
}
