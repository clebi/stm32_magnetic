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

#![feature(proc_macro)]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rtfm as rtfm;
extern crate cortex_m_semihosting as semihosting;
extern crate f3;

mod devices;

use core::fmt::Write;
use rtfm::{app, Threshold};
use semihosting::hio;
use devices::{I2C, I2CAddrMode, I2CCallbacks};
use f3::stm32f30x::I2C1;

const GPIO_FREQ_HIGH: u8 = 0x3;
const GPIO_PULLUP: u8 = 0x1;
const GPIO_AF_4: u8 = 0x4;

const ADDR_MAGNETIC_SENSOR: u8 = 0x1E;
const MAGNETIC_REG_CRA_REG_M: u8 = 0x0;
//const MAGNETIC_REG_OUT_X_H_M: u32 = 0x03;
//const MAGNETIC_REG_IRA_M: u32 = 0x03;
//const TEMP_REG_OUT_H_M: u32 = 0x31;

struct I2CMagRead {}

/// Implementation of I2CCallbacks to read a value on the magnetic sensor
impl I2CCallbacks for I2CMagRead {
    fn stop(&self, _dev: &I2C) {
        writeln!(hio::hstdout().unwrap(), "i2c_event->stop").unwrap();
    }

    fn transfer_completed(&self, dev: &I2C) {
        writeln!(hio::hstdout().unwrap(), "i2c_event->read_on_slave").unwrap();
        dev.request_read();
    }

    fn receive(&self, dev: &I2C, data: u8) {
        // read rx buffer and stop
        writeln!(hio::hstdout().unwrap(), "i2c_event->recv").unwrap();
        writeln!(
            hio::hstdout().unwrap(),
            "i2c_event->i2c1::rxdr = {:x}",
            data
        ).unwrap();
        dev.end();
    }
}

app! {
    device: f3::stm32f30x,

    resources: {
        static I2C_1: I2C;
    },

    tasks: {
        I2C1_EV_EXTI23: {
            path: i2c_event,
            resources: [I2C_1, I2C1],
        },
        I2C1_ER: {
            path: i2c_error,
            resources: [I2C1],
        }
    },
}

fn init(p: init::Peripherals) -> init::LateResourceValues {
    // Configure gpiob : PB6 & PB7
    p.RCC.ahbenr.modify(|_, w| w.iopben().enabled());
    p.GPIOB
        .moder
        .modify(|_, w| w.moder6().alternate().moder7().alternate());
    p.GPIOB
        .otyper
        .modify(|_, w| w.ot6().set_bit().ot7().set_bit());
    unsafe {
        p.GPIOB.ospeedr.modify(|_, w| {
            w.ospeedr6()
                .bits(GPIO_FREQ_HIGH)
                .ospeedr7()
                .bits(GPIO_FREQ_HIGH)
        });
        p.GPIOB
            .pupdr
            .modify(|_, w| w.pupdr6().bits(GPIO_PULLUP).pupdr7().bits(GPIO_PULLUP));
        p.GPIOB
            .afrl
            .modify(|_, w| w.afrl6().bits(GPIO_AF_4).afrl7().bits(GPIO_AF_4));
    }

    // Reset i2c1
    p.RCC.apb1rstr.modify(|_, w| w.i2c1rst().set_bit());
    p.RCC.apb1rstr.modify(|_, w| w.i2c1rst().clear_bit());
    // Enable i2c1 clock
    p.RCC.cfgr3.modify(|_, w| w.i2c1sw().clear_bit());
    p.RCC.apb1enr.modify(|_, w| w.i2c1en().enabled());

    let late_resources: init::LateResourceValues;
    unsafe {
        late_resources = init::LateResourceValues {
            I2C_1: I2C::new(&*I2C1.get(), &I2CMagRead {}),
        };
    }

    writeln!(hio::hstdout().unwrap(), "i2c_event->init done").unwrap();

    late_resources.I2C_1.set_addr_type(I2CAddrMode::Bits7);
    // write to slave
    writeln!(hio::hstdout().unwrap(), "i2c_event->begin").unwrap();
    late_resources.I2C_1.begin(ADDR_MAGNETIC_SENSOR as u16);
    late_resources.I2C_1.write(&MAGNETIC_REG_CRA_REG_M);

    late_resources
}

fn idle() -> ! {
    loop {
        rtfm::wfi();
    }
}

fn i2c_event(_t: &mut Threshold, r: I2C1_EV_EXTI23::Resources) {
    (**r.I2C_1).event_int(); //Call the i2c event manager
    writeln!(
        hio::hstdout().unwrap(),
        "i2c_event->i2c1::isr = {:x}",
        r.I2C1.isr.read().bits()
    ).unwrap();
}

fn i2c_error(_t: &mut Threshold, r: I2C1_ER::Resources) {
    writeln!(
        hio::hstdout().unwrap(),
        "i2c_error->i2c1::isr = {:x}",
        r.I2C1.isr.read().bits()
    ).unwrap();
}
