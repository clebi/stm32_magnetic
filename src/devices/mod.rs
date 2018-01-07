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

use f3::stm32f30x::i2c1::RegisterBlock;

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
pub struct I2C {
    pub device: &'static RegisterBlock,
}

impl I2C {
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
        unsafe {
            self.device
                .cr2
                .modify(|_, w| w.sadd1().bits(addr as u8).autoend().clear_bit());
        }
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
}
