//! # Watch OS
//!
//! A hardware abstraction library for my stm32l0x3 based watch.
//!
//! ---
//!
//! This library provides an opinionated way to configure and interact with the hardware. The
//! primary goal with the hardware configuration is to ensure as low power consumption as
//! possible. This is done in a few ways
//!
//! - ARM's SLEEPONEXIT is configured to keep the MCU in the MCU is in the STOP state as much as
//!   reasonable
//! - The system clock runs off the multispeed internal oscillator (MSI) which is set to 65.536
//!   kHz
//! - The voltage regulator runs at 1.2v
//! - The LCD and RTC are clocked by the external 32.768 kHz crystal (LSE) so they can continue
//!   running when the MSI is stopped

#![no_std]

pub mod adc;
pub mod buzzer;
pub mod lcd;
pub mod rtc;
pub mod system;

pub use adc::Adc;
pub use buzzer::Buzzer;
pub use lcd::Lcd;
pub use rtc::Rtc;
pub use system::System;

pub use stm32l0::stm32l0x3 as pac;
