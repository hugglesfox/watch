use cortex_m::peripheral::SCB;
use stm32l0::stm32l0x3::{PWR, RCC};

/// The system clock frequency (Hz)
pub const CLK_FREQ: usize = 65536;

/// # System management
///
/// The general clock and power configuration is such that to provide ultra low power operation
///
/// * The system clock (MSI) is set to range 0 (~65.536 kHz)
/// * The voltage regulator is set to range 3 (1.2v)
///
/// Note that the LPRUN mode isn't used as it would require a full reset after each wakeup from
/// stop. As the device is designed to constantly be entering and exiting stop mode, using
/// LPRUN isn't feasible.
pub struct System(RCC);

impl System {
    pub fn configure(rcc: RCC, pwr: &mut PWR, scb: &mut SCB) -> Self {
        // Enter stop mode on WFI
        scb.set_sleepdeep();

        // Set the MSI clock to 65.536 kHz
        rcc.icscr.write(|w| w.msirange().range0());

        // Enable PWR clock
        rcc.apb1enr.modify(|_, w| w.pwren().enabled());

        // Configure PWR control register
        //
        // * Enable voltage regulator range 3 (1.2V)
        // * Switch the regulator into low power mode when sleep or deep sleep is entered
        // * Enter stop mode on deepsleep
        // * Enable RTC write access
        pwr.cr.write(|w| {
            w.vos()
                .v1_2()
                .lpsdsr()
                .low_power_mode()
                .pdds()
                .stop_mode()
                .dbp()
                .enabled()
        });

        // Enable SYSCFG clock
        rcc.apb2enr.modify(|_, w| w.syscfgen().enabled());

        // Enable GPIO port clocks
        rcc.iopenr
            .write(|w| w.iopaen().enabled().iopben().enabled().iopcen().enabled());

        // Configure the Control/Status register
        //
        // * Set the RTC/LCD to use the LSE
        // * Set LSE to medium-high drive capability
        rcc.csr
            .modify(|_, w| w.rtcsel().lse().lsedrv().medium_high());

        // Turn on the LSE
        rcc.csr.modify(|_, w| w.lseon().on());

        // Wait for the LSE to stabilise
        while rcc.csr.read().lserdy().is_not_ready() {}

        Self(rcc)
    }

    /// Enable the ADC peripheral clock (PCLK)
    pub(crate) fn enable_adc_clk(&mut self) {
        self.0.apb2enr.modify(|_, w| w.adcen().enabled());

        // Disable ADC clock during sleep
        self.0.apb2smenr.modify(|_, w| w.adcsmen().disabled());
    }

    /// Enable the RTC
    pub(crate) fn enable_rtc(&mut self) {
        self.0.csr.modify(|_, w| w.rtcen().enabled());
    }

    /// Enable LCD perihpheral clock
    pub(crate) fn enable_lcd_clk(&mut self) {
        self.0
            .apb1enr
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << 9) });

        // Enable LCD clock during sleep
        self.0
            .apb1smenr
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << 9) });
    }

    /// Enable TIM2 peripheral clock
    pub(crate) fn enable_tim2_clk(&mut self) {
        self.0.apb1enr.modify(|_, w| w.tim2en().enabled());

        // Disable TIM2 clock during sleep
        self.0.apb1smenr.modify(|_, w| w.tim2smen().disabled());
    }
}
