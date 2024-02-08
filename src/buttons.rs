use stm32l0xx_hal::gpio::{gpioa::PA2, gpiob::PB9, Input, PullDown};
use stm32l0xx_hal::exti::{Exti, TriggerEdge, GpioLine};
use stm32l0xx_hal::syscfg::SYSCFG;

type AlarmBtn = PA2<Input<PullDown>>;
type ModeBtn = PB9<Input<PullDown>>;

/// Setup button interrupts
pub fn init(alarm: AlarmBtn, mode: ModeBtn, exti: &mut Exti, syscfg: &mut SYSCFG) {
    use stm32l0xx_hal::exti::ExtiLine as _;
    let alarm_line = GpioLine::from_raw_line(alarm.pin_number()).unwrap();
    exti.listen_gpio(syscfg, alarm.port(), alarm_line, TriggerEdge::Rising);

    let mode_line = GpioLine::from_raw_line(mode.pin_number()).unwrap();
    exti.listen_gpio(syscfg, mode.port(), mode_line, TriggerEdge::Rising);
}
