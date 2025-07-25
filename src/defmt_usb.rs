use core::{cell::{UnsafeCell}, sync::atomic::{AtomicBool, Ordering}};

use crate::serial_usb::signal_bytes;

#[defmt::global_logger]
struct Logger;

static USB_ENCODER: UsbEncoder = UsbEncoder::new();

struct UsbEncoder {
    /// A boolean lock
    ///
    /// Is `true` when `acquire` has been called and we have exclusive access to
    /// the rest of this structure.
    taken: AtomicBool,
     /// We need to remember this to exit a critical section
    cs_restore: UnsafeCell<critical_section::RestoreState>,       
    /// A `defmt::Encoder` for encoding frames
    encoder: UnsafeCell<defmt::Encoder>
}

unsafe impl Sync for UsbEncoder {}

impl UsbEncoder {
    const fn new() -> Self {
        UsbEncoder {
            taken: AtomicBool::new(false),
            cs_restore: UnsafeCell::new(critical_section::RestoreState::invalid()),
            encoder: UnsafeCell::new(defmt::Encoder::new()),
        }
    }
    
    /// Acquire the defmt encoder.
    fn acquire(&self) {
        // safety: Must be paired with corresponding call to release(), see below
        let restore = unsafe { critical_section::acquire() };

        // NB: You can re-enter critical sections but we need to make sure
        // no-one does that.
        if self.taken.load(Ordering::Relaxed) {
            panic!("defmt logger taken reentrantly")
        }

        // no need for CAS because we are in a critical section
        self.taken.store(true, Ordering::Relaxed);

        // safety: accessing the cell is OK because we have acquired a critical
        // section.
        unsafe {
            self.cs_restore.get().write(restore);
            let encoder= &mut *self.encoder.get();
            encoder.start_frame(signal_bytes);
        }
    }

    /// Write bytes to the defmt encoder.
    ///
    /// # Safety
    ///
    /// Do not call unless you have called `acquire`.
    unsafe fn write(&self, bytes: &[u8]) {
        // safety: accessing the cell is OK because we have acquired a critical
        // section.
        unsafe {
            let encoder: &mut defmt::Encoder = &mut *self.encoder.get();
            encoder.write(bytes, signal_bytes);
        }
    }

    /// Flush the encoder
    ///
    /// # Safety
    ///
    /// Do not call unless you have called `acquire`.
    unsafe fn flush(&self) {
        // safety: accessing the `&'static _` is OK because we have acquired a
        // critical section.
        // _SEGGER_RTT.up_channel.flush();
    }

    /// Release the defmt encoder.
    ///
    /// # Safety
    ///
    /// Do not call unless you have called `acquire`. This will release
    /// your lock - do not call `flush` and `write` until you have done another
    /// `acquire`.
    unsafe fn release(&self) {
        if !self.taken.load(Ordering::Relaxed) {
            panic!("defmt release out of context")
        }

        // safety: accessing the cell is OK because we have acquired a critical
        // section.
        unsafe {
            let encoder: &mut defmt::Encoder = &mut *self.encoder.get();
            encoder.end_frame(signal_bytes);
            let restore = self.cs_restore.get().read();
            self.taken.store(false, Ordering::Relaxed);
            // paired with exactly one acquire call
            critical_section::release(restore);
        }
    }

}

unsafe impl defmt::Logger for Logger {
    fn acquire() {
        USB_ENCODER.acquire();
    }

    unsafe fn flush() {
        unsafe { USB_ENCODER.flush() };
    }

    unsafe fn release() {
        unsafe { USB_ENCODER.release() };
    }

    unsafe fn write(bytes: &[u8]) {
        unsafe { USB_ENCODER.write(bytes) };
    }
}