use crate::system::System;
use core::marker::PhantomData;
use stm32l0::stm32l0x3::RTC;

use stm32l0::stm32l0x3::rtc::tr::R as TR_R;

/// Binary coded decimal represenation of the time
pub struct Time {
    pub hour_tens: u8,
    pub hour_units: u8,

    pub minute_tens: u8,
    pub minute_units: u8,

    pub seconds_tens: u8,
    pub seconds_units: u8,
}

pub struct Init;
pub struct Run;

/// # Real time clock
///
/// The RTC has two states, run mode and initialisation (init) mode. While in run mode, the RTC
/// measures time however the time registers are read only. While in init mode, the RTC is stopped
/// but the time registers become writeable, allowing the time to be set. Moving between init mode
/// and run mode can be done using the [`Rtc<Run>::init()`] and [`Rtc<Init>::run()`] methods respectively.
///
/// Note that the RTC uses 24 hour notation.
pub struct Rtc<S>(RTC, PhantomData<S>);

impl Rtc<Run> {
    pub fn configure(rtc: RTC, sys: &mut System) -> Rtc<Run> {
        // Unlock RTC registers
        rtc.wpr.write(|w| w.key().bits(0xCA));
        rtc.wpr.write(|w| w.key().bits(0x53));

        // Configure the RTC control register
        //
        // * Bypass the shadow registers. This is required due to the low APB1 clock speed
        // * Set the wakeup clock to ck_spre (1 Hz)
        rtc.cr
            .write(|w| w.bypshad().bypass_shadow_reg().wucksel().clock_spare());

        // Enable 512 Hz calibration output on PC13
        // TODO: create a feature flag to enable/disable
        rtc.cr.modify(|_, w| w.coe().enabled());

        sys.enable_rtc();

        sys.enable_rtc();

        Self(rtc, PhantomData)
    }

    /// Read the time register.
    ///
    /// Due to the system clock being slow, the register needs to be read twice to ensure a clock
    /// tick doesn't occur during the read.
    fn read_tr(&self) -> TR_R {
        let first = self.0.tr.read();
        let second = self.0.tr.read();

        if first.su().bits() != second.su().bits() {
            // An update occured during the first or second read. A third read will definately give
            // a correct result
            return self.0.tr.read();
        }

        second
    }

    /// Get the current time
    pub fn time(&self) -> Time {
        let tr = self.read_tr();

        Time {
            hour_tens: tr.ht().bits(),
            hour_units: tr.hu().bits(),

            minute_tens: tr.mnt().bits(),
            minute_units: tr.mnu().bits(),

            seconds_tens: tr.st().bits(),
            seconds_units: tr.su().bits(),
        }
    }

    /// Enter initialisation mode
    pub fn init(self) -> Rtc<Init> {
        Rtc::from(self)
    }
}

impl Rtc<Init> {
    /// Set the RTC to the given time
    pub fn set_time(&mut self, time: Time) {
        // Set time
        self.0.tr.write(|w| {
            w.ht()
                .bits(time.hour_tens)
                .hu()
                .bits(time.hour_units)
                .mnt()
                .bits(time.minute_tens)
                .mnu()
                .bits(time.minute_units)
                .st()
                .bits(time.seconds_tens)
                .su()
                .bits(time.seconds_units)
        });
    }

    /// Enter run mode
    pub fn run(self) -> Rtc<Run> {
        Rtc::from(self)
    }
}

impl From<Rtc<Run>> for Rtc<Init> {
    /// Enter initialisation mode
    fn from(rtc: Rtc<Run>) -> Rtc<Init> {
        rtc.0.isr.modify(|_, w| w.init().init_mode());

        // Wait for initialisation mode to be entered
        while rtc.0.isr.read().initf().is_not_allowed() {}

        Rtc(rtc.0, PhantomData)
    }
}

impl From<Rtc<Init>> for Rtc<Run> {
    /// Enter run mode
    fn from(rtc: Rtc<Init>) -> Rtc<Run> {
        rtc.0.isr.modify(|_, w| w.init().free_running_mode());

        // Wait for run mode to be entered
        while rtc.0.isr.read().initf().is_allowed() {}

        Rtc(rtc.0, PhantomData)
    }
}

impl<S> Rtc<S> {
    /// Write ADC calibration to RTC backup register 0
    pub(crate) fn set_adc_calibration(&mut self, calibration: u8) {
        self.0.bkpr[0].write(|w| w.bkp().bits(calibration as u32));
    }

    /// Read ADC calibration to RTC backup register 0
    pub(crate) fn get_adc_calibration(&self) -> u8 {
        self.0.bkpr[0].read().bkp().bits() as u8
    }
}
