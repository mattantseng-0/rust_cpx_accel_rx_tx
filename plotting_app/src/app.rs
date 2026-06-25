use std::sync::mpsc::Receiver;
use shared::messages::{AccMsg, Message, MessageId};
use std::thread;
use std::time::Duration;
use eframe::egui;
use egui_plot::{Plot, Line, PlotPoints, Legend};

use std::collections::VecDeque;

pub struct DataPlottingApp {
    label: String,
    num_pts: i32,
    rx: Receiver<AccMsg>,
    msgs: VecDeque<AccMsg>,
    counter: Vec<f64>,
    acc_x: Vec<f64>,
    acc_y: Vec<f64>,
    acc_z: Vec<f64>,
}

impl DataPlottingApp {
    pub fn new(rx: Receiver<AccMsg>) -> Self {
        Self {
            label: String::new(),
            num_pts: 0,
            rx: rx,
            msgs: VecDeque::<AccMsg>::new(),

            counter: Vec::new(),
            acc_x: Vec::new(),
            acc_y: Vec::new(),
            acc_z: Vec::new(),
        }
    }
}

impl eframe::App for DataPlottingApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let mut regenerate = false;

            while let Ok(msg) = self.rx.try_recv() {
                regenerate = true;
                self.msgs.push_front(msg);

            }

            if self.msgs.len() > self.num_pts as usize {

                self.msgs.truncate(self.num_pts as usize);
            }

            regenerate |= ui.add(egui::Slider::new(&mut self.num_pts, 0..=2000).text("Num Points")).changed();


            if regenerate == true {
                self.acc_x = self.msgs.iter().map(|rx_data| ((rx_data.acc_x as f64 )/ (16384 as f64))).collect();
                self.acc_y = self.msgs.iter().map(|rx_data| ((rx_data.acc_y as f64 )/ (16384 as f64))).collect();
                self.acc_z = self.msgs.iter().map(|rx_data| ((rx_data.acc_z as f64 )/ (16384 as f64))).collect();

                self.counter = self.msgs.iter().map(|rx_data| rx_data.counter as f64).collect();

            }


             let x_points: PlotPoints = self.counter
                                         .iter()
                                         .zip(self.acc_x.iter())
                                         .map(|(x, y)| [*x, *y]).collect();

            let y_points: PlotPoints = self.counter
                                         .iter()
                                         .zip(self.acc_y.iter())
                                         .map(|(x, y)| [*x, *y]).collect();

            let z_points: PlotPoints = self.counter
                                         .iter()
                                         .zip(self.acc_z.iter())
                                         .map(|(x, y)| [*x, *y]).collect();

            
            let x_line = Line::new("X-Acceleration", x_points);
            let y_line = Line::new("Y-Acceleration", y_points);
            let z_line = Line::new("Z-Acceleration", z_points);

            
            Plot::new("x_plot")
                .legend(Legend::default())
                .view_aspect(2.0)
                .show(ui, |plot_ui| { 
                    plot_ui.line(x_line); 
                    plot_ui.line(y_line); 
                    plot_ui.line(z_line); 
                });

            

        });
    }
}