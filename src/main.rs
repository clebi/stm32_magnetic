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

extern crate f3;
extern crate cortex_m;
extern crate cortex_m_rtfm as rtfm;
extern crate cortex_m_semihosting as semihosting;

use core::fmt::Write;
use rtfm::{app, Threshold};
use semihosting::hio;
use cortex_m::peripheral::SystClkSource;

const GPIO_FREQ_HIGH: u8 = 0x3;
const GPIO_PULLUP: u8 = 0x1;
const GPIO_AF_4: u8 = 0x4;

const ADDR_MAGNETIC_SENSOR: u8 = 0x1E;
const MAGNETIC_REG_CRA_REG_M: u32 = 0x0;
//const MAGNETIC_REG_OUT_X_H_M: u32 = 0x03;
//const MAGNETIC_REG_IRA_M: u32 = 0x03;
//const TEMP_REG_OUT_H_M: u32 = 0x31;

app! {
    device: f3::stm32f30x,

    tasks: {
        I2C1_EV_EXTI23: {
            path: i2c_event,
            resources: [I2C1],
        },
        I2C1_ER: {
            path: i2c_error,
            resources: [I2C1],
        }
    },
}

fn init(p: init::Peripherals) {
    p.SYST.set_clock_source(SystClkSource::Core);
    p.SYST.set_reload(9_000_000);
    p.SYST.enable_interrupt();
    p.SYST.enable_counter();

    // Configure gpiob : PB6 & PB7
    p.RCC.ahbenr.modify(|_, w| w.iopben().enabled());
    p.GPIOB.moder.modify(|_, w| w.moder6().alternate().moder7().alternate());
    p.GPIOB.otyper.modify(|_, w| w.ot6().set_bit().ot7().set_bit());
    unsafe {
        p.GPIOB.ospeedr.modify(|_, w| {w.ospeedr6().bits(GPIO_FREQ_HIGH).ospeedr7().bits(GPIO_FREQ_HIGH)});
        p.GPIOB.pupdr.modify(|_, w| w.pupdr6().bits(GPIO_PULLUP).pupdr7().bits(GPIO_PULLUP));
        p.GPIOB.afrl.modify(|_, w| w.afrl6().bits(GPIO_AF_4).afrl7().bits(GPIO_AF_4));
    }

    // Reset i2c1
    p.RCC.apb1rstr.modify(|_, w| w.i2c1rst().set_bit());
    p.RCC.apb1rstr.modify(|_, w| w.i2c1rst().clear_bit());
    // Enable i2c1 clock
    p.RCC.cfgr3.modify(|_, w| w.i2c1sw().clear_bit());
    p.RCC.apb1enr.modify(|_, w| w.i2c1en().enabled());

    // Disable i2c1 peripheral
    p.I2C1.cr1.modify(|_, w| w.pe().clear_bit());

    // Configure i2c1
    unsafe {
        p.I2C1.cr1.modify(|_, w| w.anfoff().clear_bit()
            .dnf().bits(0)
            .nostretch().clear_bit()
            .errie().set_bit()
            .tcie().set_bit()
            .stopie().set_bit()
            .nackie().set_bit()
            .rxie().set_bit()
            .txie().set_bit());
        p.I2C1.timingr.modify(|_, w| {
            w.presc().bits(1)
                .scll().bits(0x13)
                .sclh().bits(0xF)
                .sdadel().bits(0x2)
                .scldel().bits(0x4)
        });
    }

    // Enable i2c1 peripheral and set 7 bit adress mode
    p.I2C1.cr1.modify(|_, w| w.pe().set_bit());
    p.I2C1.cr2.modify(|_, w| w.add10().clear_bit());
    unsafe {
        // write to slave
        p.I2C1.cr2.modify(|_, w| w.sadd1().bits(ADDR_MAGNETIC_SENSOR).rd_wrn().clear_bit().autoend().clear_bit());
        p.I2C1.txdr.modify(|_, w| w.bits(MAGNETIC_REG_CRA_REG_M));
        p.I2C1.cr2.modify(|_, w| w.nbytes().bits(1));
        p.I2C1.cr2.modify(|_, w| w.start().set_bit());
    }
}

fn idle() -> ! {
    loop {
        rtfm::wfi();
    }
}

fn i2c_event(_t: &mut Threshold, r: I2C1_EV_EXTI23::Resources) {
    writeln!(hio::hstdout().unwrap(), "i2c_event->i2c1::isr = {:x}", r.I2C1.isr.read().bits()).unwrap();
    if r.I2C1.isr.read().rxne().bit_is_set() { // read rx buffer and stop
        writeln!(hio::hstdout().unwrap(), "i2c_event->i2c1::rxdr = {:x}", r.I2C1.rxdr.read().bits()).unwrap();
        r.I2C1.cr2.modify(|_, w| w.stop().set_bit());
    } else if r.I2C1.isr.read().stopf().bit_is_set() { // stop
        r.I2C1.icr.write(|w| w.stopcf().set_bit());
    } else if r.I2C1.isr.read().tc().bit_is_set() { // read on slave
        r.I2C1.cr2.modify(|_, w| w.rd_wrn().set_bit().start().set_bit());
    }
}

fn i2c_error(_t: &mut Threshold, r: I2C1_ER::Resources) {
    writeln!(hio::hstdout().unwrap(), "i2c_error->i2c1::isr = {:x}", r.I2C1.isr.read().bits()).unwrap();
}
