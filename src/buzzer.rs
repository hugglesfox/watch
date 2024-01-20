use stm32l0xx_hal::gpio::{gpioa::PA0, Analog};
use stm32l0xx_hal::pac::TIM2;
use stm32l0xx_hal::pwm::{Assigned, Pwm, Timer, C1};
use stm32l0xx_hal::rcc::Rcc;

use embedded_time::rate::Hertz;

pub type Buzzer = Pwm<TIM2, C1, Assigned<PA0<Analog>>>;

///  Initialize the buzzer
pub fn init(tim2: TIM2, pa0: PA0<Analog>, freq: Hertz, rcc: &mut Rcc) -> Buzzer {
    let timer = Timer::new(tim2, freq, rcc);
    let mut pwm = timer.channel1.assign(pa0);

    // 50% duty cycle
    use cortex_m::prelude::_embedded_hal_PwmPin as _;
    pwm.set_duty(pwm.get_max_duty() / 2);

    pwm
}

