// Copyright 2017 Clément Bizeau
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The `devices` module contains all functionnality to access devices

use f3::hal::stm32f30x::I2C1;

/// I2C callbacks. It contains the I2C events needed to interact with other devices.
pub trait I2CCallbacks : Sync {
    /// Called when a stop event is received.
    ///
    /// # Arguments
    ///
    /// * `device` the i2c device
    fn stop(&self, device: &I2C);

    /// Called when a transfer completed event is received.
    ///
    /// # Arguments
    ///
    /// * `device` the i2c device
    fn transfer_completed(&self, device: &I2C);

    /// Called when a rx buffer full event is received.
    ///
    /// # Arguments
    ///
    /// * `device` the i2c device
    /// * `data` the data received from the read
    fn receive(&self, device: &I2C, data: u8);
}

/// The I2C address mode
pub enum I2CAddrMode {
    /// 10 bits adress mode
    Bits10,
    /// 7 bits address mode
    Bits7,
}

/// I2C device
///
/// # Arguments
///
/// * `device` The register block of the i2c device
/// * `callbacks` An instance of I2C callbacks to manage i2c events
pub struct I2C {
    pub device: I2C1,
    pub callbacks: &'static I2CCallbacks,
}

impl I2C {
    /// Create and initialize the i2c device
    ///
    /// # Arguments
    ///
    /// * `device` - The i2c rgister block
    /// * `callbacks` - An i2c callbacks instance
    pub fn new(device: I2C1, callbacks: &'static I2CCallbacks) -> I2C {
        device.cr1.modify(|_, w| w.pe().clear_bit());
        unsafe {
            device.cr1.modify(|_, w| {
                w.anfoff()
                    .clear_bit()
                    .dnf()
                    .bits(0)
                    .nostretch()
                    .clear_bit()
                    .errie()
                    .set_bit()
                    .tcie()
                    .set_bit()
                    .stopie()
                    .set_bit()
                    .nackie()
                    .set_bit()
                    .rxie()
                    .set_bit()
                    .txie()
                    .set_bit()
            });
            device.timingr.modify(|_, w| {
                w.presc()
                    .bits(1)
                    .scll()
                    .bits(0x13)
                    .sclh()
                    .bits(0xF)
                    .sdadel()
                    .bits(0x2)
                    .scldel()
                    .bits(0x4)
            });
        }

        let i2c = I2C {
            device: device,
            callbacks: callbacks,
        };
        i2c.enable();

        i2c
    }

    /// Disable i2c device
    pub fn disable(&self) {
        self.device.cr1.modify(|_, w| w.pe().clear_bit());
    }

    /// Enable i2c device
    pub fn enable(&self) {
        self.device.cr1.modify(|_, w| w.pe().set_bit());
    }

    /// Set the address type
    ///
    /// #Arguments
    ///
    /// * `addr_mode` - the address mode, either 7 bits or 10 bits
    pub fn set_addr_type(&self, addr_mode: I2CAddrMode) {
        match addr_mode {
            I2CAddrMode::Bits10 => self.device.cr2.modify(|_, w| w.add10().set_bit()),
            I2CAddrMode::Bits7 => self.device.cr2.modify(|_, w| w.add10().clear_bit()),
        }
    }

    /// Begins transmission with a slave
    ///
    /// #Arguments
    ///
    /// * `addr` the address of the salve
    pub fn begin(&self, addr: u16) {
        self.device
            .cr2
            .modify(|_, w| w.sadd1().bits(addr as u8).autoend().clear_bit());
    }

    /// Ends transmission
    pub fn end(&self) {
        self.device.cr2.modify(|_, w| w.stop().set_bit());
    }

    /// Write a byte to a slave
    pub fn write(&self, byte: &u8) {
        unsafe {
            self.device
                .cr2
                .modify(|_, w| w.rd_wrn().clear_bit().nbytes().bits(1));
            self.device.txdr.modify(|_, w| w.bits(*byte as u32));
            self.device.cr2.modify(|_, w| w.start().set_bit());
        }
    }

    /// Request read to a slave
    pub fn request_read(&self) {
        self.device
            .cr2
            .modify(|_, w| w.rd_wrn().set_bit().start().set_bit());
    }

    /// This function receive the i2c event interrupt and dispatch it to the corresponding callback.
    pub fn event_int(&self) {
        if self.rx_buffer_full() {
            self.callbacks.receive(self, self.rx_read());
        } else if self.stopped() {
            self.clear_stop();
            self.callbacks.stop(self);
        } else if self.transfer_completed() {
            self.callbacks.transfer_completed(self);
        }
    }

    /// Read the rx buffer
    pub fn rx_read(&self) -> u8 {
        self.device.rxdr.read().rxdata().bits()
    }

    /// Is a transfer completed
    pub fn transfer_completed(&self) -> bool {
        self.device.isr.read().tc().bit_is_set()
    }

    /// Is the RX buffer full
    pub fn rx_buffer_full(&self) -> bool {
        self.device.isr.read().rxne().bit_is_set()
    }

    /// Is the communication stopped
    pub fn stopped(&self) -> bool {
        self.device.isr.read().stopf().bit_is_set()
    }

    /// clear the sopped flag
    pub fn clear_stop(&self) {
        self.device.icr.write(|w| w.stopcf().set_bit());
    }

    /// Read the ISR register
    pub fn isr_read(&self) -> u32 {
        return self.device.isr.read().bits();
    }
}
