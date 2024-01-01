//! # Analogue to digital converter (ADC)
//!
//! In the watch, the ADC is used for reading the temperature as well as reading
//! the battery cell voltage.
//!
//! ## Calibrating
//!
//! The ADC needs to be recalibrated when an environmental change occurs. The
//! biggest factor is the battery voltage however temperature can also affect
//! it's readings. It's recommended to run the [`calibrate()`](Adc::calibrate)
//! method on a regular bases to ensure the ADC stays accurate.
//!
//! The resulting calibration value is written to the RTC backup register and
//! read back in before each conversion sequence. This allows for the
//! calibration to persist when the MCU enters STOP mode.
//!
//! ## Sample time
//!
//! With a system clock of 65.536 kHz, an ADC clock prescaler of /2 and sample
//! duration of 1.5 clock cycles, this equates to a sample time of is approx
//! 46μs. The prescaler is there to ensure that the ADC can get a 50% duty
//! cycle, square wave clock signal.
//!
//! The minimum sample time for the temperature sensor and VREFINT voltage is
//! 10μs.

use core::ops::{DerefMut, Deref};

use crate::rtc::Rtc;
use crate::system::System;
use stm32l0::stm32l0x3::{ADC, SYSCFG};

const VREFINT_CAL_VREF: u16 = 3000; // mV

const TS_CAL1_TEMP: u16 = 30; // °C
const TS_CAL2_TEMP: u16 = 130; // °C

/// The results of an ADC measurement
pub struct AdcMeasurement {
    vrefint: u16,
    tsense: u16,
}

impl AdcMeasurement {
    /// Get the battery voltage in millivolts
    pub unsafe fn voltage(&self) -> u16 {
        let vrefint_cal = 0x1FF80078 as *const u16;

        (VREFINT_CAL_VREF * *vrefint_cal) / self.vrefint
    }

    /// Get the temperature in degrees celsius
    ///
    pub unsafe fn temperature(&self) -> u16 {
        // FIXME: Make this millidegrees
        let ts_cal1 = 0x1FF8007A as *const u16;
        let ts_cal2 = 0x1FF8007E as *const u16;

        let gradient = (TS_CAL2_TEMP - TS_CAL1_TEMP) / (*ts_cal2 - *ts_cal1);

        gradient * (self.tsense - *ts_cal1) + TS_CAL1_TEMP
    }
}

/// # ADC
///
/// TODO: 
///
/// See [`crate::adc`] for a more information.
pub struct Adc(ADC);

impl Deref for Adc {
    type Target = ADC;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Adc {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Adc {
    /// Configure the ADC
    pub fn configure(adc: ADC, sys: &mut System, syscfg: &mut SYSCFG) -> Self {
        sys.enable_adc_clk();

        // Use PCLK/2 as the ADC clock
        adc.cfgr2.write(|w| w.ckmode().pclk_div2());

        // Enable low frequency mode as PCLK is <3.5 MHz
        adc.ccr.write(|w| w.lfmen().enabled());

        // Configure SYSCFG reference control and status register
        //
        // * Enable temperature sensor reference
        // * Enable VREFINT reference
        // FIXME: Does having the references always enabled consume power?
        syscfg
            .cfgr3
            .modify(|_, w| w.enbuf_sensor_adc().enabled().enbuf_vrefint_adc().enabled());

        // Configure ADC channel selection register
        //
        // * Select VREFINT (channel 17)
        // * Select temperature sensor (channel 18)
        adc.chselr
            .write(|w| w.chsel17().selected().chsel18().selected());

        Self(adc)
    }

    /// Enable the ADC
    pub fn enabled(&mut self, f: impl FnOnce(Enabled)) {
        // Configure ADC common configuration register
        //
        // * Enable vrefint
        // * Enable temperature sensor
        (*self)
            .ccr
            .modify(|_, w| w.vrefen().enabled().tsen().enabled());

        // Both VREFINT and TSEN have a maximum start time of 10us. As the sysclk is at 65.536 kHz,
        // each clock cycle is ~15us. Therefore by the time the ADC is ready,
        // they will be stabilized.

        // Enable ADC
        (*self).cr.modify(|_, w| w.aden().enabled());

        // Wait for the ADC to power up
        while self.0.isr.read().adrdy().is_not_ready() {}
        (*self).isr.modify(|_, w| w.adrdy().clear());
        
        // Execute f
        f(Enabled(&mut *self));

        // Configure ADC common configuration register
        //
        // * Disable vrefint
        // * Disable temperature sensor
        (*self)
            .ccr
            .modify(|_, w| w.vrefen().disabled().tsen().disabled());

        // Disable ADC
        (*self).cr.modify(|_, w| w.addis().disable());
    }

    /// Calibrate the ADC.
    pub fn calibrate(&mut self, rtc: &mut Rtc) {
        (*self).cr.modify(|_, w| w.adcal().start_calibration());

        while (*self).isr.read().eocal().is_not_complete() {}
        (*self).isr.modify(|_, w| w.eocal().clear());

        rtc.set_adc_calibration((*self).calfact.read().calfact().bits());

        // Ensure ADCAL = 0 before continuing
        while (*self).cr.read().adcal().is_calibrating() {}
    }
}

/// ADC enabled implementation
pub struct Enabled<'a>(&'a mut ADC);

impl<'a> Deref for Enabled<'a> {
    type Target = ADC;

    fn deref(&self) -> &Self::Target {
        &self.0        
    }
}

impl<'a> DerefMut for Enabled<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> Enabled<'a> {
    /// Read the next adc conversion
    fn read(&self) -> u16 {
        // Wait for the conversion to finish
        while (*self).isr.read().eoc().is_not_complete() {}

        // Reading ADC_DR clears the conversion finished status bit
        (*self).dr.read().data().bits()
    }

    /// Apply the calibration stored in the RTC backup registers
    fn apply_calibration(&mut self, rtc: &Rtc) {
        (*self)
            .calfact
            .write(|w| w.calfact().bits(rtc.get_adc_calibration()));
    }

    /// Perform a measurement
    pub fn measure(&mut self, rtc: &Rtc) -> AdcMeasurement {
        self.apply_calibration(rtc);

        let mut vrefint = 0;
        let mut tsense = 0;

        // First measurement is vrefint, followed immediately by the temperature
        // sensor
        cortex_m::interrupt::free(|_| {
            // Start conversion sequence
            (*self).cr.modify(|_, w| w.adstart().start_conversion());

            vrefint = self.read();
            tsense = self.read();
        });

        // Ensure ADSTART = 0 before continuing
        while (*self).cr.read().adstart().is_active() {}

        AdcMeasurement { vrefint, tsense }
    }
}
