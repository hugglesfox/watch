#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use defmt_rtt as _; // global logger
use panic_probe as _; // panic handler

mod buzzer;

#[rtic::app(
    device = stm32l0xx_hal::pac,
    dispatchers = [I2C1, I2C2, I2C3]
)]
mod app {
    use crate::buzzer::{self, Buzzer};

    use cortex_m::peripheral::SCB;
    use stm32l0xx_hal::prelude::*;

    use stm32l0xx_hal::exti::Exti;
    use stm32l0xx_hal::pwr::PWR;
    use stm32l0xx_hal::rcc::Rcc;
    use stm32l0xx_hal::rtc::Rtc;
    use stm32l0xx_hal::{exti, pwr, rcc, rtc};

    use rtic_monotonics::systick::{fugit::ExtU32, Systick};

    #[shared]
    struct Shared {
        pwr: PWR,
        scb: SCB,
        rcc: Rcc,
    }

    #[local]
    struct Local {
        buzzer: Buzzer,
        rtc: Rtc,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("init");

        let dp = cx.device;
        let cp = cx.core;

        // Set clock to MSI range 0
        let mut rcc = dp.RCC.freeze(rcc::Config::msi(rcc::MSIRange::Range0));
        let mut pwr = PWR::new(dp.PWR, &mut rcc);

        pwr.enter_low_power_run_mode(rcc.clocks);

        // Setup buzzer
        let gpioa = dp.GPIOA.split(&mut rcc);
        let buzzer = buzzer::init(dp.TIM2, gpioa.pa0, 440.Hz(), &mut rcc);

        // Setup systick timer
        let systick_token = rtic_monotonics::create_systick_token!();
        Systick::start(cp.SYST, rcc.clocks.sys_clk().0, systick_token);

        // Setup RTC
        let mut rtc = Rtc::new(dp.RTC, &mut rcc, &pwr, None).unwrap();
        rtc.enable_interrupts(rtc::Interrupts {
            wakeup_timer: true,
            ..rtc::Interrupts::default()
        });

        // Listen for RTC wakeup timer interrupt requests
        let mut exti = Exti::new(dp.EXTI);
        exti.listen_configurable(exti::ConfigurableLine::RtcWakeup, exti::TriggerEdge::Rising);

        // Start wakeup timer to update watch face every second
        rtc.wakeup_timer().start(1_u32);

        (
            Shared {
                pwr,
                rcc,
                scb: cp.SCB,
            },
            Local { rtc, buzzer },
        )
    }

    #[idle(shared = [pwr, scb, rcc])]
    fn idle(cx: idle::Context) -> ! {
        defmt::info!("idle");

        let pwr = cx.shared.pwr;
        let scb = cx.shared.scb;
        let rcc = cx.shared.rcc;

        (pwr, scb, rcc).lock(|pwr, mut scb, mut rcc| {
            defmt::info!("entering stop mode");
            // ULP needs to be disabled for the LCD to work
            pwr.stop_mode(
                &mut scb,
                &mut rcc,
                pwr::StopModeConfig {
                    ultra_low_power: false,
                },
            )
            .enter();
        });

        // Should never be reached
        loop {}
    }

    #[task(binds = RTC, local = [rtc])]
    fn wakeup(cx: wakeup::Context) {
        let rtc = cx.local.rtc;

        // Clear interrupt
        rtc.wakeup_timer().wait().unwrap();
    }

    #[task(priority = 1, local = [buzzer])]
    async fn beep(cx: beep::Context) {
        let buzzer = cx.local.buzzer;

        buzzer.enable();
        Systick::delay(1.secs()).await;
        buzzer.disable();
    }
}
