use stm32l0xx_hal::exti::{Exti, GpioLine, TriggerEdge};
use stm32l0xx_hal::gpio::{gpioa::PA2, gpiob::PB9, Input, PullDown};
use stm32l0xx_hal::syscfg::SYSCFG;

type AlarmBtn = PA2<Input<PullDown>>;
type ModeBtn = PB9<Input<PullDown>>;

pub trait Button {
    fn gpio_line(&self) -> GpioLine;

    /// Enable interrupt for button
    ///
    /// For the watch, the alarm button is on interrupt `EXTI2_3` and the mode button is on
    /// interrupt `EXTI4_15`
    fn enable_interrupt(&self, exti: &mut Exti, syscfg: &mut SYSCFG);
}

macro_rules! buttons {
    ( $( $Btn:ident ),* ) => {
        $(
            impl Button for $Btn {
                fn gpio_line(&self) -> GpioLine {
                    use stm32l0xx_hal::exti::ExtiLine as _;
                    GpioLine::from_raw_line(self.pin_number()).unwrap()
                }

                fn enable_interrupt(&self, exti: &mut Exti, syscfg: &mut SYSCFG) {
                    exti.listen_gpio(syscfg, self.port(), self.gpio_line(), TriggerEdge::Rising);
                }
            }
        )*
    }
}

buttons!{AlarmBtn, ModeBtn}
