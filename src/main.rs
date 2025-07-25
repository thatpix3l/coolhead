#![no_std]
#![no_main]

mod constants;
mod serial_usb;
mod defmt_usb;

use panic_probe as _;

use embassy_time::{Duration, Instant, Timer};

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::{gpio::{Input, Level::{High, Low}, Pull}};

const IR_COMMAND_PAUSE: Duration = Duration::from_secs(500);

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    info!("Starting...");
    let p = embassy_rp::init(Default::default());
    
    let mut ir_pin = Input::new(p.PIN_16, Pull::None);
    
    _spawner.spawn(serial_usb::new_serial(_spawner.clone(), p.USB)).unwrap();
    _spawner.spawn(repeatedly_print_message_usb()).unwrap();
    
    let mut pause_begin = Instant::now();
    let mut pulse_begin = pause_begin.clone();
    
    loop {
        ir_pin.wait_for_any_edge().await;
        let l = ir_pin.get_level();
        match l {
            Low =>  {
                pulse_begin = Instant::now();
                serial_usb::signal_bytes("pulsing...\n".as_bytes());
            },
            High => {
                pause_begin = Instant::now();
                serial_usb::signal_bytes("pausing...\n".as_bytes());
            },
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