#![no_std]
#![no_main]

use bsp::hal;
use circuit_playground_express as bsp;
use core::mem::MaybeUninit;
use hal::rtc::rtic::rtc_clock;

#[cfg(not(feature = "use_semihosting"))]
use panic_halt as _;
#[cfg(feature = "use_semihosting")]
use panic_semihosting as _;

hal::rtc_monotonic!(Mono, rtc_clock::ClockCustom<8_192>);

use shared::messages::{AccMsg, Message};
use postcard::take_from_bytes_crc32;
use crc::{Crc, CRC_32_CKSUM};
const CRC_ALGO: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

use lis3dh::{Lis3dh, SlaveAddr, accelerometer::RawAccelerometer};
use hal::time::Hertz;
use hal::sercom::i2c;
use heapless::spsc::Queue;



#[rtic::app(device = bsp::pac, dispatchers = [EVSYS])]
mod app {
    use super::*;
    use usb_device::bus::UsbBusAllocator;
    use usb_device::prelude::*;
    use usbd_serial::{SerialPort, USB_CLASS_CDC};
    
    use bsp::pin_alias;
    use hal::clock::{ClockGenId, ClockSource, GenericClockController};
    use hal::pac::Peripherals;
    use hal::prelude::*;
    use hal::usb::UsbBus;
    use circuit_playground_express::I2c;

    type AccelI2cConfig = hal::sercom::i2c::Config<
        hal::sercom::i2c::Pads<
            hal::pac::Sercom1, 
            bsp::AccelSda, 
            bsp::AccelScl
        >
    >;

    #[local]
    struct Local {
        lis3dh: Lis3dh<lis3dh::Lis3dhI2C<hal::sercom::i2c::I2c<AccelI2cConfig>>>,
    }

    #[shared]
    struct Shared {
        usb_bus: UsbDevice<'static, UsbBus>,
        usb_serial: SerialPort<'static, UsbBus>,
        data_queue: Queue<(i16, i16, i16), 8>, 

    }

    #[init(local=[usb_allocator: MaybeUninit<UsbBusAllocator<UsbBus>> = MaybeUninit::uninit()])]
    fn init(cx: init::Context) -> (Shared, Local) {
        let mut peripherals: Peripherals = cx.device;
        let mut core: rtic::export::Peripherals = cx.core;
        let mut clocks = GenericClockController::with_internal_32kosc(
            peripherals.gclk,
            &mut peripherals.pm,
            &mut peripherals.sysctrl,
            &mut peripherals.nvmctrl,
        );
        let pins = bsp::Pins::new(peripherals.port);

        *cx.local.usb_allocator = MaybeUninit::new(bsp::usb_allocator(
            peripherals.usb,
            &mut clocks,
            &mut peripherals.pm,
            pins.usb_dm,
            pins.usb_dp,
        ));
        // The usb allocator is initialized just above, which makes sure that
        // usb_allocator is allocated by this point. The reason it is done this
        // way is to avoid runtime checks for wheather the allocator is update
        // that would be required if we use an Option<UsbAllocator instead of
        // a MaybeUninit.
        let usb_allocator = unsafe { cx.local.usb_allocator.assume_init_ref() };
        let usb_serial = SerialPort::new(usb_allocator);
        let usb_bus = UsbDeviceBuilder::new(usb_allocator, UsbVidPid(0x16c0, 0x27dd))
            .strings(&[StringDescriptors::new(LangID::EN)
                .manufacturer("Fake company")
                .product("Serial port")
                .serial_number("TEST")])
            .expect("Failed to set strings")
            .device_class(USB_CLASS_CDC)
            .build();

        // Set the RTC clock to use a 8.192 kHz clock derived from the internal 32 kHz
        // oscillator.
        let rtc_clock_src = clocks
            .configure_gclk_divider_and_source(ClockGenId::Gclk2, 4, ClockSource::Osc32k, true)
            .unwrap();
        clocks.configure_standby(ClockGenId::Gclk2, true);
        let _ = clocks.rtc(&rtc_clock_src).unwrap();

        let gclk0 = clocks.gclk0();

        let sercom1_clock = clocks
            .sercom1_core(&gclk0)
            .expect("Could not configure core clock for SERCOM1");

        let freq = sercom1_clock.freq();

        let sda_pin: bsp::AccelSda = pins.accel_sda.into();
        let scl_pin: bsp::AccelScl = pins.accel_scl.into();

        let i2c_pads = i2c::Pads::new(sda_pin, scl_pin);

        let i2c = i2c::Config::new(
            &mut peripherals.pm,
            peripherals.sercom1,
            i2c_pads, 
            freq,
        ).baud(Hertz::Hz(400000)) // Configure for 400kHz fast mode
        .enable();

        let mut lis3dh = Lis3dh::new_i2c(i2c, SlaveAddr::Alternate).unwrap();

        lis3dh.set_range(lis3dh::Range::G2).unwrap();
        lis3dh.set_datarate(lis3dh::DataRate::Hz_400).unwrap();

        Mono::start(peripherals.rtc);

        core.SCB.set_sleepdeep();

        usb_tx_loop::spawn().unwrap();
        poll_accel::spawn().unwrap();

        (
            Shared {
                usb_bus,
                usb_serial,
                data_queue: heapless::spsc::Queue::new(), 

            },
            Local {
                lis3dh,

            },
        )
    }

    #[task(local = [lis3dh], shared = [data_queue])]
    async fn poll_accel(mut cx: poll_accel::Context)
    {
        loop {
            if let Ok(sample) = cx.local.lis3dh.accel_raw() {
                cx.shared.data_queue.lock(|queue| {
                    let _ = queue.enqueue((sample.x, sample.y, sample.z));
                });
            }
        
            Mono::delay(2u64.millis()).await;
        }
    }

    #[task(shared = [usb_serial, data_queue])]
    async fn usb_tx_loop(mut cx: usb_tx_loop::Context)
    {
        let mut counter: u16 = 0;
        let mut tx_msg = AccMsg::new();
        // let mut output_buffer = [0u8; core::mem::size_of::<AccMsg>() + 4];
        let mut output_buffer = [0u8; 64];
        let mut offset: usize = 0;

        loop {


            cx.shared.data_queue.lock(|queue| {
                while let Some((raw_x, raw_y, raw_z)) = queue.dequeue() {
                    // reset our offset counter
                    offset = 0;

                    // while let Some((raw_x, raw_y, raw_z)) = queue.dequeue()) {
                    tx_msg.acc_x = raw_x;
                    tx_msg.acc_y = raw_y;
                    tx_msg.acc_z = raw_z; 

                    let serialized_slice = postcard::to_slice_crc32(
                        &tx_msg, 
                        &mut output_buffer, 
                        CRC_ALGO.digest()
                    ).expect("Serialization failed");

                    cx.shared.usb_serial.lock(|serial| {
                        while offset < serialized_slice.len() {
                            match serial.write(&serialized_slice[offset..]) {
                                Ok(count) => offset += count,
                                Err(_) => break,
                            }
                        }                    
                    });
                    tx_msg.counter = tx_msg.counter.wrapping_add(1);
                }   
            });

            
            
            Mono::delay(1u64.millis()).await;
        }
    }

    #[task(binds = USB, shared = [usb_bus, usb_serial])]
    fn poll_usb(cx: poll_usb::Context) {
        let mut serial = cx.shared.usb_serial;
        let mut usb_bus = cx.shared.usb_bus;

        (&mut serial, &mut usb_bus).lock(|s, b| {
            if !b.poll(&mut [s]) {
                return;
            }
        })
    }
}
