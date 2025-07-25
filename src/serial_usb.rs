
use defmt::{info, panic, unwrap};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::USB,
    usb::{
        Driver,
        Instance,
        InterruptHandler
    }
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    signal::Signal
};
use embassy_usb::{
    class::cdc_acm::{
        CdcAcmClass,
        State
    },
    driver::EndpointError,
    UsbDevice
};
use static_cell::StaticCell;

pub struct Packet {
    data: [u8; 64],
    len: usize
}

impl Packet {
    pub fn get_data<'a>(&'a self) -> &'a [u8] {
        &self.data[..self.len]
    }
}

impl From<&[u8]> for Packet {
    fn from(value: &[u8]) -> Self {
        let mut packet = Packet {
            data: [0; 64],
            len: value.len(),
        };
        packet.data[..value.len()].copy_from_slice(value);
        packet
    }
}

pub static PACKET: Signal<CriticalSectionRawMutex, Packet> = Signal::new();

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

pub fn signal_bytes(b: &[u8]) {
    PACKET.signal(Packet::from(b));
}

#[embassy_executor::task]
pub async fn new_serial(spawner: Spawner, usb_peripheral: embassy_rp::Peri<'static, USB>) {

    // Create the driver, from the HAL.
    let driver = Driver::new(usb_peripheral, Irqs);

    // Create embassy-usb Config
    let config = {
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("thatpix3l");
        config.product = Some("coolhead");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        config
    };

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let builder = embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        );
        builder
    };

    // Create classes on the builder.
    let mut class = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };

    // Build the builder.
    let usb_device = builder.build();

    // Run the USB device.
    unwrap!(spawner.spawn(usb_task(usb_device)));

    // Do stuff with the class!
    loop {
        class.wait_connection().await;
        info!("Connected");
        let _ = start_packet_writer(&mut class).await;
        info!("Disconnected");
    }
}

type MyUsbDriver = Driver<'static, USB>;
type MyUsbDevice = UsbDevice<'static, MyUsbDriver>;

#[embassy_executor::task]
async fn usb_task(mut usb: MyUsbDevice) -> ! {
    usb.run().await
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn start_packet_writer<'d, T: Instance + 'd>(class: &mut CdcAcmClass<'d, Driver<'d, T>>) -> Result<(), Disconnected> {
    loop {
        let packet = PACKET.wait().await;
        class.write_packet(packet.get_data()).await?
    }
}