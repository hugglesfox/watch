use crate::system::{System, CLK_FREQ};
use core::marker::PhantomData;
use stm32l0::stm32l0x3::{GPIOA, TIM2};

// Timer prescaler value to give a 1 Hz tick
const PRESCALER: u16 = (CLK_FREQ - 1) as u16;

/// Calculate the value for the auto reload register from a frequency (Hz)
///
/// See [`Buzzer::arr()`] for usage information.
pub const fn arr_from_frequency(freq: usize) -> u16 {
    ((CLK_FREQ / (freq * (PRESCALER as usize + 1))) - 1) as u16
}

/// Calculate the value for the compare capture register from a duty cycle (%) and AAR value
/// ([`arr_from_frequency`])
///
/// See [`Buzzer::ccr()`] for usage information.
pub const fn ccr_from_duty(duty: usize, aar: u16) -> u16 {
    (duty as u16 * aar / 100) as u16
}

pub struct Running;
pub struct Stopped;

/// Pezio buzzer
///
/// Note that the buzzer is connected to the channel 1 output pin of TIM2 (PA0)
pub struct Buzzer<S>(TIM2, PhantomData<S>);

impl Buzzer<Stopped> {
    pub fn configure(timer: TIM2, sys: &mut System, gpio: &mut GPIOA) -> Buzzer<Stopped> {
        sys.enable_tim2_clk();

        // Configure PA0 to use alternate function mode
        gpio.moder.modify(|_, w| w.mode0().alternate());

        // Set PA0 alternate function to TIM2_CH1 (AF2)
        gpio.afrl.modify(|_, w| w.afsel0().af2());

        // Enable PWM mode 1 for TIM2_CH1
        timer.ccmr1_output().write(|w| w.oc1m().pwm_mode1());

        // Enable TIM2_CH1 output pin
        timer.ccer.write(|w| w.cc1e().enabled());

        // Configure prescaler so that timer has frequency of 1Hz (clk / (PSC + 1))
        timer.psc.write(|w| w.psc().bits(PRESCALER));

        Self(timer, PhantomData)
    }

    /// Start the buzzer
    pub fn start(self) -> Buzzer<Running> {
        Buzzer::from(self)
    }
}

impl Buzzer<Running> {
    /// Stop the buzzer
    pub fn stop(self) -> Buzzer<Stopped> {
        Buzzer::from(self)
    }
}

impl From<Buzzer<Running>> for Buzzer<Stopped> {
    /// Stop the buzzer
    fn from(buzzer: Buzzer<Running>) -> Buzzer<Stopped> {
        buzzer.0.cr1.modify(|_, w| w.cen().disabled());

        Buzzer(buzzer.0, PhantomData)
    }
}

impl From<Buzzer<Stopped>> for Buzzer<Running> {
    /// Start the buzzer
    fn from(buzzer: Buzzer<Stopped>) -> Buzzer<Running> {
        buzzer.0.cr1.modify(|_, w| w.cen().enabled());

        Buzzer(buzzer.0, PhantomData)
    }
}

impl<S> Buzzer<S> {
    /// Set the auto reload register.
    ///
    /// This value correlates to the frequency of the buzzer and can be calulated using the
    /// [`arr_from_frequency()`] function. The intended use case is for the auto reload
    /// value to be calculated as a constant which can then be set after configuring the buzzer.
    ///
    /// ARR can be set at any time.
    ///
    /// ```rust
    /// // Calculate a buzzer frequency of 1 kHz
    /// const BUZZER_FREQ = arr_from_frequency(1000);
    ///
    /// buzzer.aar(BUZZER_FREQ);
    /// ```
    pub fn arr(&mut self, arr: u16) {
        self.0.arr.write(|w| w.arr().bits(arr));
    }

    /// Set the capture compare register.
    ///
    /// This value correlates to the duty cycle of the buzzer and can be calulated using the
    /// [`ccr_from_duty()`] function. The intended use case is for the auto reload
    /// value to be calculated as a constant which can then be set after configuring the buzzer.
    ///
    /// CCR can be set at any time.
    ///
    /// ```rust
    /// // Calculate a buzzer frequency of 1 kHz
    /// const BUZZER_FREQ: u16 = arr_from_frequency(1000);
    ///
    /// // Calculate a buzzer duty cycle of 50%
    /// const BUZZER_DUTY: u16 = ccr_from_duty(50, BUZZER_FREQ);
    ///
    /// buzzer.crr(BUZZER_DUTY);
    /// ```
    pub fn ccr(&mut self, arr: u16) {
        self.0.arr.write(|w| w.arr().bits(arr));
    }
}
