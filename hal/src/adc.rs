use crate::rtc::Rtc;
use crate::system::System;
use core::marker::PhantomData;
use once_cell::sync::Lazy;
use stm32l0::stm32l0x3::{ADC, SYSCFG};

const VREFINT_CAL_VREF: usize = 3000; // mV
const VREFINT_CAL: *const u16 = 0x1FF80078 as *const u16;

const TS_CAL1_TEMP: usize = 30; // °C
const TS_CAL1: *const u16 = 0x1FF8007A as *const u16;

const TS_CAL2_TEMP: usize = 130; // °C
const TS_CAL2: *const u16 = 0x1FF8007E as *const u16;

static VREFINT_VREF_BY_CAL: Lazy<usize> =
    Lazy::new(|| unsafe { VREFINT_CAL_VREF * (*VREFINT_CAL as usize) });
static TS_GRADIENT: Lazy<usize> = Lazy::new(|| unsafe {
    (TS_CAL2_TEMP - TS_CAL1_TEMP) / (*TS_CAL2 as usize - *TS_CAL1 as usize)
});

/// The results of an ADC measurement
pub struct AdcMeasurement {
    vrefint: u16,
    tsense: u16,
}

impl AdcMeasurement {
    /// Get the battery voltage in millivolts
    pub fn voltage(&self) -> usize {
        *VREFINT_VREF_BY_CAL / self.vrefint as usize
    }

    /// Get the temperature in degrees celsius
    ///
    /// FIXME: Make this millidegrees
    pub fn temperature(&self) -> usize {
        *TS_GRADIENT * (self.tsense as usize - TS_CAL1 as usize) + TS_CAL1_TEMP
    }
}

struct Enabled;
struct Disabled;

/// # Analogue to digital converter (ADC)
///
/// In the watch, the ADC is used for reading the temperature of the watch as well as reading the
/// battery cell voltage.
///
/// ## Calibrating
///
/// The ADC needs to be recalibrated when an environmental change occurs. The biggest factor is the
/// battery voltage however temperature can also affect it's readings. It's recommended to run the
/// [`calibrate()`](Adc::calibrate) method on a regular bases to ensure the ADC stays accurate.
///
/// The resulting calibration value is written to the RTC backup register and read back in before
/// each conversion sequence. This allows for the calibration to presist when the MCU enters STOP
/// mode.
///
/// ## Sample time
///
/// With a system clock of 65.536 kHz, an ADC clock prescaler of /2 and sample duration of 1.5 clock
/// cycles, this equates to a sample time of is approx 46μs. The prescaler is there to ensure that
/// the ADC can get a 50% duty cycle, square wave clock signal.
///
/// The minimum sample time for the temperature sensor and VREFINT voltage is 10μs.
pub struct Adc<S>(ADC, PhantomData<S>);

impl Adc<Disabled> {
    pub fn configure(adc: ADC, sys: &mut System, syscfg: &mut SYSCFG) -> Adc<Disabled> {
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

        Self(adc, PhantomData)
    }

    /// Enable the ADC
    pub fn enable(self) -> Adc<Enabled> {
        Adc::from(self)
    }

    /// Calibrate the ADC.
    pub fn calibrate<S>(&mut self, rtc: &mut Rtc<S>) {
        self.0.cr.modify(|_, w| w.adcal().start_calibration());

        while self.0.isr.read().eocal().is_not_complete() {}
        self.0.isr.modify(|_, w| w.eocal().clear());

        rtc.set_adc_calibration(self.0.calfact.read().calfact().bits());

        // Ensure ADCAL = 0 before continuing
        while self.0.cr.read().adcal().is_calibrating() {}
    }
}

impl Adc<Enabled> {
    /// Disable the ADC
    pub fn disable(self) -> Adc<Disabled> {
        Adc::from(self)
    }

    /// Read the next adc conversion
    fn read(&self) -> u16 {
        // Wait for the conversion to finish
        while self.0.isr.read().eoc().is_not_complete() {}

        // Reading ADC_DR clears the conversion finished status bit
        self.0.dr.read().data().bits()
    }

    /// Apply the calibration stored in the RTC backup registers
    fn apply_calibration<S>(&mut self, rtc: &Rtc<S>) {
        self.0
            .calfact
            .write(|w| w.calfact().bits(rtc.get_adc_calibration()));
    }

    /// Perform a measurement
    pub fn measure<S>(&mut self, rtc: &Rtc<S>) -> AdcMeasurement {
        self.apply_calibration(rtc);

        let mut vrefint = 0;
        let mut tsense = 0;

        // First measurement is vrefint, followed immediently by the temperature sensor
        cortex_m::interrupt::free(|_| {
            // Start conversion sequence
            self.0.cr.modify(|_, w| w.adstart().start_conversion());

            vrefint = self.read();
            tsense = self.read();
        });

        // Ensure ADSTART = 0 before continuing
        while self.0.cr.read().adstart().is_active() {}

        AdcMeasurement { vrefint, tsense }
    }
}

impl From<Adc<Disabled>> for Adc<Enabled> {
    /// Enable the ADC
    fn from(adc: Adc<Disabled>) -> Adc<Enabled> {
        // Configure ADC common configuration register
        //
        // * Enable vrefint
        // * Enable temperature sensor
        adc.0
            .ccr
            .modify(|_, w| w.vrefen().enabled().tsen().enabled());

        // Both VREFINT and TSEN have a maximum start time of 10us. As the sysclk is at 65.536 kHz,
        // each clock cycle is ~15us. Therefore by the time the ADC is ready, they will be stablised.

        // Enable ADC
        adc.0.cr.modify(|_, w| w.aden().enabled());

        // Wait for the ADC to power up
        while adc.0.isr.read().adrdy().is_not_ready() {}
        adc.0.isr.modify(|_, w| w.adrdy().clear());

        Adc(adc.0, PhantomData)
    }
}

impl From<Adc<Enabled>> for Adc<Disabled> {
    /// Disable the ADC, powering down it's regulator
    fn from(adc: Adc<Enabled>) -> Adc<Disabled> {
        // Configure ADC common configuration register
        //
        // * Disable vrefint
        // * Disable temperature sensor
        adc.0
            .ccr
            .modify(|_, w| w.vrefen().disabled().tsen().disabled());

        // Disable ADC
        adc.0.cr.modify(|_, w| w.addis().disable());

        Adc(adc.0, PhantomData)
    }
}
