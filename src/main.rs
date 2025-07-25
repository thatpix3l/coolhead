#![no_std]
#![no_main]

mod constants;
mod serial_usb;
mod defmt_usb;

use panic_probe as _;

use embassy_time::{Duration, Timer};

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::{gpio::{Input, Level::{High, Low}, Pull}};

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    info!("Starting...");
    let p = embassy_rp::init(Default::default());
    
    let mut ir_pin = Input::new(p.PIN_16, Pull::None);
    
    _spawner.spawn(serial_usb::new_serial(_spawner.clone(), p.USB)).unwrap();
    _spawner.spawn(repeatedly_print_message_usb()).unwrap();
    
    loop {
        ir_pin.wait_for_any_edge().await;
        let l = ir_pin.get_level();
        match l {
            Low => info!("pulsing..."),
            High => info!("pausing..."),
        }
    }

}

#[embassy_executor::task]
async fn repeatedly_print_message_usb() {
    loop {
        Timer::after(Duration::from_secs(1)).await;
        // signal_bytes("bruh\n".as_bytes());
        defmt::println!("lmao")
    }
}