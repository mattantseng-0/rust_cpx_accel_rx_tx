use crc::{Crc, CRC_32_CKSUM};
use shared::messages::{AccMsg, Message};
use std::time::Duration;
use num_traits::WrappingAdd;
use std::thread;

use std::f32::consts::PI;


const CRC_ALGO: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

const DT: f32 = 0.01;

fn main() {
    let port_name = "/dev/ttys035";

    let mut port = serialport::new(port_name, 0)
    .timeout(Duration::from_millis(1000))
    .open()
    .expect("Failed to open tx port");

    let mut tx_msg = AccMsg::new();

    let mut counter: u16 = 0;

    // the crc is 4 bytes
    let mut output_buffer = [0u8; std::mem::size_of::<AccMsg>() + 4];

    // To check syncing of the rx, inject some garbage bytes into the buffer
    let random_bytes: [u8; 3] = [1u8, 2u8, 3u8];


    loop {

        // every 10th message inject some garbage data. This is to test that the rx 
        // can resync after a misalignment
        if counter % 10 == 0
        {
            port.write_all(&random_bytes)
            .expect("Write failed");
        }
        tx_msg.counter = counter;

        tx_msg.acc_x = tx_msg.acc_x.wrapping_add(1) % 10;
        tx_msg.acc_y = tx_msg.acc_y.wrapping_add(2) % 20;
        tx_msg.acc_z = tx_msg.acc_z.wrapping_add(3) % 30;

        tx_msg.print_fields();

        let serialized_slice = postcard::to_slice_crc32(
            &tx_msg, 
            &mut output_buffer, 
            CRC_ALGO.digest()
        ).expect("Serialization failed");

        // Every 100 messages flip a byte after the crc calculation. 
        // This is to check the rx behavior on corrupted data
        if counter % 100 == 0 {
            serialized_slice[3] = !serialized_slice[3];
        }


        port.write_all(&serialized_slice)
        .expect("Write failed");

        counter = counter.wrapping_add(1 as u16);
        thread::sleep(Duration::from_millis(10));


    }


}