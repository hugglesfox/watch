use core::ops::Deref;
use core::default::Default;
use stm32l0xx_hal::calibration::{VtempCal30, VtempCal130, VrefintCal};

/// An ADC temperature reading
pub struct Temperature(u16);

impl From<u16> for Temperature {
    /// Convert the raw ADC reading into degrees C
    fn from(raw: u16) -> Self {
        let ts_cal1 = VtempCal30::get();
        let ts_cal2 = VtempCal130::get();
        let cal_gradient = (130 - 30) / (ts_cal2.read() - ts_cal1.read());

        Self(cal_gradient * (raw - ts_cal1.read()) + 30)
    }
}

impl Deref for Temperature {
    type Target = u16;

    fn deref(&self) -> &u16 {
        &self.0
    }
}

impl Default for Temperature {
    fn default() -> Self {
        Self(0)
    }
}


/// An ADC voltage reading
pub struct Voltage(u16);

impl From<u16> for Voltage {
    /// Convert the raw ADC value into volts
    fn from(raw: u16) -> Self {
        let vrefint_cal = VrefintCal::get();
        Self(3 * vrefint_cal.read() / raw)
    }
}

impl Deref for Voltage {
    type Target = u16;

    fn deref(&self) -> &u16 {
        &self.0
    }
}

impl Default for Voltage {
    fn default() -> Self {
        Self(0)
    }
}
