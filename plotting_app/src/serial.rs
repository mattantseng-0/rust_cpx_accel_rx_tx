use std::sync::mpsc::Sender;
use shared::messages::{AccMsg, Message, MessageId};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::collections::VecDeque;


use eframe::egui;


use postcard::take_from_bytes_crc32;
use crc::{Crc, CRC_32_CKSUM};
const CRC_ALGO: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

// how many samples do you average the freq over
const TIME_AVG_SAMPLES: usize = 10000;


pub fn serial_receiver_thread(tx: Sender<AccMsg>,  ctx: egui::Context, serial_device: String) {

    let mut my_counter: u16 = 0;
    let mut read_fifo: Vec<u8> = Vec::new();
    // let port_name = "/dev/tty.usbmodem11401";
    let port_name = serial_device;
    let time_since_last_msg = Instant::now();

    let mut msg_times = VecDeque::new();

    let mut msg_counter: u16 = 0;
    let mut num_rx_msgs: u16 = 0;

    println!("Reading from: {}", port_name);

    let mut port = serialport::new(port_name, 0)
    .timeout(Duration::from_millis(10))
    .open()
    .expect("Failed to open rx port");

    
    loop {
        let mut read_buffer = [0u8; 1024];
        let bytes_read = 0;
        match port.read(&mut read_buffer) {
            Ok(bytes_read) => {
                read_fifo.extend_from_slice(&read_buffer[..bytes_read]);
                // println!("Bytes read: {:?} bytes in read_fifo: {:?}", bytes_read, read_fifo.len());


            }
            Err(e) => {
                println!("rx.rs 52: {:?}", e);

            }
        }

        // This will repeat while there's enough bytes in the read fifo for there to be a valid message
        // The +4 is to account for the CRC added by postcard
        while read_fifo.len() > std::mem::size_of::<AccMsg>() + 4  {
            // Attempt to create an AccMsg from the bytes in the read_fifo
            match take_from_bytes_crc32::<AccMsg>(&read_fifo, CRC_ALGO.digest()) {
                // If we were able to parse a valid message:
                Ok((parsed_msg, remaining)) => {
                    msg_counter = parsed_msg.counter;
                    num_rx_msgs = num_rx_msgs.wrapping_add(1);
                    
                    // println!("remaining.len(): {:?}", remaining.len());
                    // Put our parsed message in our message queue
                    tx.send(parsed_msg);

                    // Update the read fifo to contain the unprocessed bytes
                    read_fifo = remaining.to_vec();

                    ctx.request_repaint();

                    msg_times.push_back(Instant::now());


                    if msg_times.len() >= 2 {
                        let oldest = msg_times.front().unwrap();
                        let newest = msg_times.back().unwrap();

                        let dt = newest.duration_since(*oldest).as_secs_f64();

                        if dt > 0.0 {

                            let freq = (msg_times.len() - 1) as f64 / dt;
                            println!("RX Frequency: {:.2} Hz", freq);
                        }

                    }

                    if msg_times.len() > TIME_AVG_SAMPLES {
                        msg_times.drain(0..TIME_AVG_SAMPLES);
                    }

                }
                // If we are unable to parse a valid message:
                Err(e) => {
                    // This will likely fail if the CRC is invalid which can be because of a misalignment, or dropped bytes
                    println!("Error: {:?}", e);
                    // If there's a decoding issue, then assume misalignment and drop the 0th entry to attempt to realign
                    read_fifo.remove(0);
                    break;
                }
            }

        }

        println!("RX Success Ratio: {}", (msg_counter as f64/num_rx_msgs as f64 + 0.00001) as f64);

    }

}