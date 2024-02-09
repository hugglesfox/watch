use stm32l0xx_hal::gpio::{gpioa::PA0, Analog};
use stm32l0xx_hal::pwm::{Assigned, Pwm, C1};
use stm32l0xx_hal::pac::TIM2;

pub type Buzzer = Pwm<TIM2, C1, Assigned<PA0<Analog>>>;
