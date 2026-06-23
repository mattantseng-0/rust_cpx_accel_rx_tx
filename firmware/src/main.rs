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

    #[local]
    struct Local {
        // usb_allocator: UsbBusAllocator<UsbBus>,
    }

    #[shared]
    struct Shared {
        usb_bus: UsbDevice<'static, UsbBus>,
        usb_serial: SerialPort<'static, UsbBus>,
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

        Mono::start(peripherals.rtc);

        core.SCB.set_sleepdeep();

        usb_tx_loop::spawn().unwrap();


        (
            Shared {
                usb_bus,
                usb_serial,
            },
            Local {},
        )
    }


     #[task(shared = [usb_serial])]
    async fn usb_tx_loop(mut cx: usb_tx_loop::Context)
    {
        let counter: u16 = 0;
        let mut tx_msg = AccMsg::new();
        let mut output_buffer = [0u8; core::mem::size_of::<AccMsg>() + 4];
        loop {

            tx_msg.acc_x = (tx_msg.counter as i16);
            tx_msg.acc_y = (tx_msg.counter as i16) + 1i16;
            tx_msg.acc_z = (tx_msg.counter as i16) - 1i16; 

            let serialized_slice = postcard::to_slice_crc32(
                &tx_msg, 
                &mut output_buffer, 
                CRC_ALGO.digest()
            ).expect("Serialization failed");

            cx.shared.usb_serial.lock(|serial| {
                let _ = serial.write(serialized_slice);
            });
 
            Mono::delay(100u64.micros()).await;
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
