use std::io::Read;
use std::time::Duration;
use std::vec::{Vec};
use std::collections::VecDeque;

use eframe::egui;
use egui_plot::{Plot, Line, PlotPoints, Legend};
use env_logger;


use postcard::take_from_bytes_crc32;
use crc::{Crc, CRC_32_CKSUM};

use std::thread;

use std::time::Instant;



use shared::messages::{AccMsg, Message, MessageId};

const CRC_ALGO: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);


fn update_serial_data(rx_data: &mut VecDeque<AccMsg>, read_fifo: &mut Vec<u8>, ctx: &egui::Context, num_points: u32, num_msgs: &mut u32) -> bool {

    ctx.request_repaint(); 

    let mut got_new_data = false;

    println!("Update Serial Data");
    let port_name = "/dev/tty.usbmodem1401";
    // let port_name = "/dev/ttys035";

    let mut port = serialport::new(port_name, 0)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Failed to open rx port");

    // port.clear(serialport::ClearBuffer::All).expect("Failed to clear buffers");
    
    loop {

        let mut read_buffer = [0u8; 1024];
        let bytes_read = 0;
        match port.read(&mut read_buffer) {
            Ok(bytes_read) => {
                read_fifo.extend_from_slice(&read_buffer[..bytes_read]);
                println!("Bytes read: {:?} bytes in read_fifo: {:?}", bytes_read, read_fifo.len());

                print!("read_buffer: ");
                for byte in read_buffer[..bytes_read].iter()
                {
                    print!("{:02X}", byte);
                }
                println!();

            }
            Err(e) => {
                println!("rx.rs 52: {:?}", e);

            }
        }

        while read_fifo.len() > std::mem::size_of::<AccMsg>() + 4  {
            match take_from_bytes_crc32::<AccMsg>(&read_fifo, CRC_ALGO.digest()) {
                Ok((parsed_msg, remaining)) => {
                    println!("remaining.len(): {:?}", remaining.len());
                    rx_data.push_front(parsed_msg);
                    *read_fifo = remaining.to_vec();
                    got_new_data = true;
                    *num_msgs = num_msgs.wrapping_add(1);

                    println!("rx_data.len(): {:?}", rx_data.len());
                    if rx_data.len() > num_points as usize {

                        rx_data.truncate(num_points as usize);
                    }
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                    // If there's a decoding issue, then assume misalignment and drop the 0th entry to attempt to realign
                    read_fifo.remove(0);
                    break;
                }
            }

        }

        // if the msg buffer is empty, and we did not get new data, then break but don't repaint
        if read_fifo.is_empty() && !got_new_data {
            println!("Msg fifo is empty");
            break;


        }

        // if we got new data, then break and request a repaint
        if got_new_data {
            println!("Got new data");
            // ctx.request_repaint(); 
            
            break;
        }



    } 
    return got_new_data;
}

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            Ok(Box::<MyApp>::default())
        }),
    )
}


#[derive(Default)]
struct MyApp {
    text: String,
    num_pts: i32,
    read_fifo: Vec<u8>,
    rx_data: VecDeque<AccMsg>,
    counter: Vec<f64>,
    acc_x: Vec<f64>,
    acc_y: Vec<f64>,
    acc_z: Vec<f64>,
    num_points: u32,
    prev_num_msgs: u32,
    num_msgs: u32,
    msg_freq: f32,
    tx_rx_ratio: f32,
}


impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {

        let start_time = Instant::now();
        // thread::sleep(Duration::from_millis(10));

        egui::CentralPanel::default().show_inside(ui, |ui| {
            println!("This is line 140");
            ui.label("This is a test");
            ui.label(format!("Counter: {:?}", self.counter.get(0)));

            match self.counter.get(0) {
                Some(val) => self.tx_rx_ratio = (*val as f32) /(self.num_msgs as f32),
                None => self.tx_rx_ratio = 0f32,
            }


            ui.label(format!("Tx/Rx ratio: {:?}", self.tx_rx_ratio));


            let mut regenerate = update_serial_data(&mut self.rx_data, &mut self.read_fifo, ui.ctx(), self.num_points, &mut self.num_msgs);

            self.msg_freq = ((self.num_msgs - self.prev_num_msgs) as f32) / start_time.elapsed().as_secs_f32();
            
            // Update the previous number of messages
            self.prev_num_msgs = self.num_msgs;

            ui.label(format!("Msg Frequency: {}Hz", self.msg_freq));



            if regenerate {
                println!("Updating data vecs");
                self.acc_x = self.rx_data.iter().map(|rx_data| rx_data.acc_x as f64).collect();
                self.acc_y = self.rx_data.iter().map(|rx_data| rx_data.acc_y as f64).collect();
                self.acc_z = self.rx_data.iter().map(|rx_data| rx_data.acc_z as f64).collect();

                self.counter = self.rx_data.iter().map(|rx_data| rx_data.counter as f64).collect();

            }

            match self.acc_x.last() {
                Some(last_item) => println!("Last: {}", last_item),
                None => println!("Vector is empty"),
            }
            regenerate |= ui.add(egui::Slider::new(&mut self.num_points, 0..=2000).text("Num Points")).changed();



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
            Plot::new("x_plot").legend(Legend::default()).view_aspect(2.0).show(ui, |plot_ui| { plot_ui.line(x_line); plot_ui.line(y_line); plot_ui.line(z_line); });

        });
    }
}
