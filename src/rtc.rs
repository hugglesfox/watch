use stm32l0xx_hal::exti::{Exti, ConfigurableLine, TriggerEdge};
use stm32l0xx_hal::rtc::{Rtc, Interrupts};

pub trait RtcInt {
    /// Enable interrupts for the RTC wakeup timer
    fn enable_wakeup_interrupt(&mut self, exti: &mut Exti);
}

impl RtcInt for Rtc {
    fn enable_wakeup_interrupt(&mut self, exti: &mut Exti) {
        self.enable_interrupts(Interrupts {
            wakeup_timer: true,
            ..Interrupts::default()
        });

        // Listen for RTC wakeup timer interrupt requests
        exti.listen_configurable(ConfigurableLine::RtcWakeup, TriggerEdge::Rising);
    }
}
